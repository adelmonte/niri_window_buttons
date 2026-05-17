use std::{cell::{Cell, RefCell}, collections::HashMap, mem, rc::Rc, time::Duration};
use async_channel::Sender;
use libpulse_binding::{
    callbacks::ListResult,
    context::{
        Context, FlagSet, State,
        subscribe::{Facility, InterestMaskSet},
    },
    proplist::properties,
};
use libpulse_glib_binding::Mainloop;
use waybar_cffi::gtk::glib;

#[derive(Debug, Clone)]
pub struct SinkInput {
    pub index: u32,
    pub muted: bool,
}

#[derive(Debug, Clone, Default)]
pub struct AudioState {
    pub by_pid: HashMap<u32, Vec<SinkInput>>,
    pub by_name: HashMap<String, Vec<SinkInput>>,
}

thread_local! {
    static PA_MAINLOOP: RefCell<Option<Mainloop>> = RefCell::new(None);
    static PA_CONTEXT: RefCell<Option<Context>> = RefCell::new(None);
    static PA_RECONNECT_BACKOFF: Cell<u64> = const { Cell::new(1) };
}

pub fn create_stream() -> async_channel::Receiver<AudioState> {
    let (tx, rx) = async_channel::unbounded();
    glib::idle_add_local_once(move || setup_pulse_audio(tx));
    rx
}

pub fn toggle_mute(sink_inputs: &[(u32, bool)]) {
    let all_muted = sink_inputs.iter().all(|(_, muted)| *muted);
    let target_mute = !all_muted;
    PA_CONTEXT.with(|ctx| {
        if let Some(ctx) = ctx.borrow().as_ref() {
            let mut introspector = ctx.introspect();
            for &(index, _) in sink_inputs {
                let _ = introspector.set_sink_input_mute(index, target_mute, None);
            }
        }
    });
}

fn schedule_reconnect(tx: Sender<AudioState>) {
    PA_CONTEXT.with(|ctx| { ctx.borrow_mut().take(); });
    PA_MAINLOOP.with(|ml| { ml.borrow_mut().take(); });

    let backoff = PA_RECONNECT_BACKOFF.with(|b| {
        let current = b.get();
        b.set((current * 2).min(30));
        current
    });

    tracing::warn!(backoff, "scheduling PulseAudio reconnect");
    glib::timeout_add_local_once(Duration::from_secs(backoff), move || {
        setup_pulse_audio(tx);
    });
}

fn setup_pulse_audio(tx: Sender<AudioState>) {
    let mainloop = match Mainloop::new(None) {
        Some(m) => m,
        None => {
            tracing::error!("failed to create PulseAudio GLib mainloop");
            schedule_reconnect(tx);
            return;
        }
    };

    let mut context = match Context::new(&mainloop, "niri-window-buttons") {
        Some(c) => c,
        None => {
            tracing::error!("failed to create PulseAudio context");
            schedule_reconnect(tx);
            return;
        }
    };

    let tx_state = tx.clone();
    let tx_reconnect = tx.clone();
    context.set_state_callback(Some(Box::new(move || {
        let state = PA_CONTEXT.with(|ctx| ctx.borrow().as_ref().map(|c| c.get_state()));
        match state {
            Some(State::Ready) => {
                PA_RECONNECT_BACKOFF.with(|b| b.set(1));
                on_context_ready(tx_state.clone());
            }
            Some(State::Failed | State::Terminated) => {
                tracing::error!("PulseAudio context disconnected");
                let _ = tx_reconnect.try_send(AudioState::default());
                let tx = tx_reconnect.clone();
                // Defer cleanup so we don't drop the context mid-callback
                glib::idle_add_local_once(move || schedule_reconnect(tx));
            }
            _ => {}
        }
    })));

    if let Err(e) = context.connect(None, FlagSet::NOFLAGS, None) {
        tracing::error!("failed to connect to PulseAudio: {:?}", e);
        context.set_state_callback(None);
        schedule_reconnect(tx);
        return;
    }

    PA_MAINLOOP.with(|ml| *ml.borrow_mut() = Some(mainloop));
    PA_CONTEXT.with(|ctx| *ctx.borrow_mut() = Some(context));
}

fn on_context_ready(tx: Sender<AudioState>) {
    query_audio_state(tx.clone());

    PA_CONTEXT.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();
        if let Some(ctx) = ctx_ref.as_mut() {
            let _ = ctx.subscribe(InterestMaskSet::SINK_INPUT, |_| {});
            let tx_cb = tx;
            ctx.set_subscribe_callback(Some(Box::new(move |facility, _op, _index| {
                if matches!(facility, Some(Facility::SinkInput)) {
                    query_audio_state(tx_cb.clone());
                }
            })));
        }
    });
}

fn read_parent_pid(pid: u32) -> Option<u32> {
    let content = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in content.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            let ppid: u32 = rest.trim().parse().ok()?;
            if ppid <= 1 {
                return None;
            }
            return Some(ppid);
        }
    }
    None
}

fn read_process_name(pid: u32) -> Option<String> {
    std::fs::read_to_string(format!("/proc/{pid}/comm"))
        .ok()
        .map(|s| s.trim().to_lowercase())
}

fn query_audio_state(tx: Sender<AudioState>) {
    PA_CONTEXT.with(|ctx| {
        let ctx_ref = ctx.borrow();
        if let Some(ctx) = ctx_ref.as_ref() {
            let introspector = ctx.introspect();
            let client_pids: Rc<RefCell<HashMap<u32, u32>>> = Rc::new(RefCell::new(HashMap::new()));
            let _ = introspector.get_client_info_list(move |result| {
                match result {
                    ListResult::Item(info) => {
                        let pid = info.proplist.get_str(properties::APPLICATION_PROCESS_ID)
                            .or_else(|| info.proplist.get_str("pipewire.sec.pid"))
                            .and_then(|s| s.trim().parse::<u32>().ok());
                        if let Some(pid) = pid {
                            client_pids.borrow_mut().insert(info.index, pid);
                        }
                    }
                    ListResult::End => {
                        query_sink_inputs(tx.clone(), Rc::clone(&client_pids));
                    }
                    ListResult::Error => {}
                }
            });
        }
    });
}

fn query_sink_inputs(tx: Sender<AudioState>, client_pids: Rc<RefCell<HashMap<u32, u32>>>) {
    PA_CONTEXT.with(|ctx| {
        let ctx_ref = ctx.borrow();
        if let Some(ctx) = ctx_ref.as_ref() {
            let introspector = ctx.introspect();
            let accumulator: Rc<RefCell<Vec<(u32, u32, bool)>>> = Rc::new(RefCell::new(Vec::new()));
            let _ = introspector.get_sink_input_info_list(move |result| {
                match result {
                    ListResult::Item(info) => {
                        if info.corked {
                            return;
                        }
                        if let Some(role) = info.proplist.get_str(properties::MEDIA_ROLE) {
                            match role.trim() {
                                "event" | "a11y" => return,
                                _ => {}
                            }
                        }
                        let pid = info.proplist.get_str(properties::APPLICATION_PROCESS_ID)
                            .and_then(|s| s.trim().parse::<u32>().ok())
                            .or_else(|| {
                                info.client.and_then(|cid| client_pids.borrow().get(&cid).copied())
                            });
                        if let Some(pid) = pid {
                            accumulator.borrow_mut().push((info.index, pid, info.mute));
                        }
                    }
                    ListResult::End => {
                        let items = mem::take(&mut *accumulator.borrow_mut());
                        let mut state = AudioState::default();
                        for (index, pid, muted) in items {
                            let sink_input = SinkInput { index, muted };
                            if let Some(name) = read_process_name(pid) {
                                state.by_name.entry(name).or_default().push(sink_input.clone());
                            }
                            let mut current = pid;
                            loop {
                                state.by_pid.entry(current).or_default().push(sink_input.clone());
                                match read_parent_pid(current) {
                                    Some(parent) => current = parent,
                                    None => break,
                                }
                            }
                        }
                        let _ = tx.try_send(state);
                    }
                    ListResult::Error => {
                        tracing::error!("error querying PulseAudio sink inputs");
                    }
                }
            });
        }
    });
}

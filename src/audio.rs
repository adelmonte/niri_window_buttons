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

struct PulseConnection {
    // Fields drop in declaration order: the context must release its events
    // before the GLib main loop that owns them is freed.
    context: Context,
    _mainloop: Mainloop,
}

thread_local! {
    static PA_CONNECTION: RefCell<Option<PulseConnection>> = const { RefCell::new(None) };
    static PA_STARTED: Cell<bool> = const { Cell::new(false) };
    static PA_SUBSCRIBERS: RefCell<Vec<Sender<AudioState>>> = const { RefCell::new(Vec::new()) };
    static PA_LAST_STATE: RefCell<Option<AudioState>> = const { RefCell::new(None) };
    static PA_RECONNECT_BACKOFF: Cell<u64> = const { Cell::new(1) };
}

pub fn create_stream() -> async_channel::Receiver<AudioState> {
    let (tx, rx) = async_channel::unbounded();
    PA_LAST_STATE.with(|state| {
        if let Some(state) = state.borrow().as_ref() {
            let _ = tx.try_send(state.clone());
        }
    });
    PA_SUBSCRIBERS.with(|subscribers| {
        let mut subscribers = subscribers.borrow_mut();
        subscribers.retain(|subscriber| !subscriber.is_closed());
        subscribers.push(tx);
    });
    // Waybar creates one module per output. They share a single PulseAudio
    // connection, including while the initial setup or a reconnect is pending.
    if !PA_STARTED.with(|started| started.replace(true)) {
        glib::idle_add_local_once(setup_pulse_audio);
    }
    rx
}

fn publish_audio_state(state: AudioState) {
    PA_SUBSCRIBERS.with(|subscribers| {
        subscribers.borrow_mut().retain(|tx| tx.try_send(state.clone()).is_ok());
    });
    PA_LAST_STATE.with(|last| *last.borrow_mut() = Some(state));
}

pub fn toggle_mute(sink_inputs: &[(u32, bool)]) {
    let all_muted = sink_inputs.iter().all(|(_, muted)| *muted);
    let target_mute = !all_muted;
    PA_CONNECTION.with(|ctx| {
        if let Some(ctx) = ctx.borrow().as_ref().map(|connection| &connection.context) {
            let mut introspector = ctx.introspect();
            for &(index, _) in sink_inputs {
                let _ = introspector.set_sink_input_mute(index, target_mute, None);
            }
        }
    });
}

fn schedule_reconnect() {
    PA_CONNECTION.with(|ctx| {
        if let Some(mut connection) = ctx.borrow_mut().take() {
            connection.context.set_state_callback(None);
        }
    });

    let backoff = PA_RECONNECT_BACKOFF.with(|b| {
        let current = b.get();
        b.set((current * 2).min(30));
        current
    });

    tracing::warn!(backoff, "scheduling PulseAudio reconnect");
    glib::timeout_add_local_once(Duration::from_secs(backoff), setup_pulse_audio);
}

fn setup_pulse_audio() {
    let mainloop = match Mainloop::new(None) {
        Some(m) => m,
        None => {
            tracing::error!("failed to create PulseAudio GLib mainloop");
            schedule_reconnect();
            return;
        }
    };

    let mut context = match Context::new(&mainloop, "niri-window-buttons") {
        Some(c) => c,
        None => {
            tracing::error!("failed to create PulseAudio context");
            schedule_reconnect();
            return;
        }
    };

    context.set_state_callback(Some(Box::new(|| {
        let state = PA_CONNECTION.with(|ctx| ctx.borrow().as_ref().map(|connection| connection.context.get_state()));
        match state {
            Some(State::Ready) => {
                PA_RECONNECT_BACKOFF.with(|b| b.set(1));
                on_context_ready();
            }
            Some(State::Failed | State::Terminated) => {
                tracing::error!("PulseAudio context disconnected");
                publish_audio_state(AudioState::default());
                // Defer cleanup so we don't drop the context mid-callback
                glib::idle_add_local_once(schedule_reconnect);
            }
            _ => {}
        }
    })));

    if let Err(e) = context.connect(None, FlagSet::NOFLAGS, None) {
        tracing::error!("failed to connect to PulseAudio: {:?}", e);
        context.set_state_callback(None);
        schedule_reconnect();
        return;
    }

    PA_CONNECTION.with(|connection| {
        *connection.borrow_mut() = Some(PulseConnection { context, _mainloop: mainloop });
    });
}

fn on_context_ready() {
    query_audio_state();

    PA_CONNECTION.with(|ctx| {
        let mut ctx_ref = ctx.borrow_mut();
        if let Some(ctx) = ctx_ref.as_mut().map(|connection| &mut connection.context) {
            let _ = ctx.subscribe(InterestMaskSet::SINK_INPUT, |_| {});
            ctx.set_subscribe_callback(Some(Box::new(|facility, _op, _index| {
                if matches!(facility, Some(Facility::SinkInput)) {
                    query_audio_state();
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

fn query_audio_state() {
    PA_CONNECTION.with(|ctx| {
        let ctx_ref = ctx.borrow();
        if let Some(ctx) = ctx_ref.as_ref().map(|connection| &connection.context) {
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
                        query_sink_inputs(Rc::clone(&client_pids));
                    }
                    ListResult::Error => {}
                }
            });
        }
    });
}

fn query_sink_inputs(client_pids: Rc<RefCell<HashMap<u32, u32>>>) {
    PA_CONNECTION.with(|ctx| {
        let ctx_ref = ctx.borrow();
        if let Some(ctx) = ctx_ref.as_ref().map(|connection| &connection.context) {
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
                        publish_audio_state(state);
                    }
                    ListResult::Error => {
                        tracing::error!("error querying PulseAudio sink inputs");
                    }
                }
            });
        }
    });
}

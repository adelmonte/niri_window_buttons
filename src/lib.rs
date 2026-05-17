use std::{
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    sync::LazyLock,
};

use futures::StreamExt;
use settings::{Settings, ButtonAlignment};
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};
use waybar_cffi::{
    Module,
    gtk::{self, Orientation, Overlay, ReliefStyle, ScrolledWindow, gio, glib::MainContext, traits::{AdjustmentExt, BoxExt, ButtonExt, ContainerExt, ScrolledWindowExt, StyleContextExt, WidgetExt}, gdk::EventMask, prelude::WidgetExtManual},
    waybar_module,
};

mod audio;
mod compositor;
mod errors;
mod global;
mod icons;
mod notifications;
mod screen;
mod settings;
mod system;
mod widget;

use audio::AudioState;
use compositor::{WindowInfo, WindowSnapshot};
use errors::ModuleError;
use global::{EventMessage, SharedState};
use notifications::NotificationData;
use system::ProcessInfo;
use widget::{WindowButton, SelectionState, create_selection_state, clear_selection, set_taskbar_adjustment};

static LOGGING: LazyLock<()> = LazyLock::new(|| {
    if let Err(e) = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_span_events(FmtSpan::CLOSE)
        .try_init()
    {
        eprintln!("tracing subscriber initialization failed: {e}");
    }
});

struct WindowButtonsModule;

impl Module for WindowButtonsModule {
    type Config = Settings;

    fn init(info: &waybar_cffi::InitInfo, settings: Settings) -> Self {
        *LOGGING;

        let shared_state = SharedState::create(settings);
        let context = MainContext::default();

        if let Err(e) = context.block_on(initialize_module(info, shared_state)) {
            tracing::error!(%e, "module initialization failed");
        }

        Self
    }
}

waybar_module!(WindowButtonsModule);

async fn initialize_module(info: &waybar_cffi::InitInfo, state: SharedState) -> Result<(), ModuleError> {
    let root = info.get_root_widget();

    let main_container = gtk::Box::new(Orientation::Horizontal, 0);

    let left_arrow = gtk::Button::new();
    left_arrow.set_label(state.settings().scroll_arrow_left());
    left_arrow.set_relief(ReliefStyle::None);
    left_arrow.style_context().add_class("scroll-arrow");
    left_arrow.style_context().add_class("scroll-arrow-left");
    left_arrow.set_sensitive(false);
    left_arrow.set_no_show_all(true);
    left_arrow.hide();
    
    let scrolled = ScrolledWindow::new(None::<&gtk::Adjustment>, None::<&gtk::Adjustment>);
    scrolled.set_policy(gtk::PolicyType::External, gtk::PolicyType::Never);
    scrolled.set_overlay_scrolling(false);
    scrolled.set_propagate_natural_width(false);

    let initial_max_width = state.settings().max_taskbar_width_for_output(None);
    main_container.set_size_request(initial_max_width, -1);
    main_container.set_hexpand(false);

    let button_container = gtk::Box::new(Orientation::Horizontal, 0);
    button_container.style_context().add_class("niri-window-buttons");
    button_container.add_events(EventMask::SCROLL_MASK | EventMask::SMOOTH_SCROLL_MASK);

    let alignment_spacer = gtk::Box::new(Orientation::Horizontal, 0);
    alignment_spacer.set_size_request(0, 0);
    button_container.add(&alignment_spacer);

    let drag_overlay = Overlay::new();
    drag_overlay.add(&button_container);
    scrolled.add(&drag_overlay);

    set_taskbar_adjustment(scrolled.hadjustment());

    let scrolled_for_scroll = scrolled.clone();
    button_container.connect_scroll_event(move |_, event| {
        use waybar_cffi::gtk::gdk::ScrollDirection;

        let hadj = scrolled_for_scroll.hadjustment();
        let step = hadj.page_size() / 4.0;

        match event.direction() {
           ScrollDirection::Up | ScrollDirection::Left => {
               hadj.set_value((hadj.value() - step).max(0.0));
               gtk::glib::Propagation::Stop
           }
           ScrollDirection::Down | ScrollDirection::Right => {
               let max = hadj.upper() - hadj.page_size();
               hadj.set_value((hadj.value() + step).min(max));
               gtk::glib::Propagation::Stop
           }
           ScrollDirection::Smooth => {
               let (delta_x, delta_y) = event.delta();
               let delta = if delta_x.abs() > delta_y.abs() { delta_x } else { delta_y };
               let max = hadj.upper() - hadj.page_size();
               let new_value = (hadj.value() + delta * step).clamp(0.0, max);
               hadj.set_value(new_value);
               gtk::glib::Propagation::Stop
           }
           _ => gtk::glib::Propagation::Proceed
        }
    });
    
    let right_arrow = gtk::Button::new();
    right_arrow.set_label(state.settings().scroll_arrow_right());
    right_arrow.set_relief(ReliefStyle::None);
    right_arrow.style_context().add_class("scroll-arrow");
    right_arrow.style_context().add_class("scroll-arrow-right");
    right_arrow.set_sensitive(false);
    right_arrow.set_no_show_all(true);
    right_arrow.hide();
    
    main_container.pack_start(&left_arrow, false, false, 0);
    main_container.pack_start(&scrolled, true, true, 0);
    main_container.pack_start(&right_arrow, false, false, 0);
    
    root.add(&main_container);
   
    let hadj = scrolled.hadjustment();
    
    let update_arrows = {
        let hadj = hadj.clone();
        let left_arrow = left_arrow.clone();
        let right_arrow = right_arrow.clone();
        
        move || {
            let value = hadj.value();
            let upper = hadj.upper();
            let page_size = hadj.page_size();
            let has_overflow = upper > page_size + 0.5;
            
            if !has_overflow {
                left_arrow.hide();
                right_arrow.hide();
            } else {
                left_arrow.show();
                right_arrow.show();
                
                let at_start = value < 0.5;
                let max_scroll = upper - page_size;
                let at_end = value >= max_scroll - 0.5;
                
                left_arrow.set_sensitive(!at_start);
                right_arrow.set_sensitive(!at_end);
            }
        }
    };
    
    let update_on_changed = update_arrows.clone();
    hadj.connect_changed(move |_| {
        let update = update_on_changed.clone();
        gtk::glib::idle_add_local_once(move || {
            update();
        });
    });

    let update_on_value = update_arrows.clone();
    hadj.connect_value_changed(move |_| {
        let update = update_on_value.clone();
        gtk::glib::idle_add_local_once(move || {
            update();
        });
    });
    
    let hadj_left = hadj.clone();
    left_arrow.connect_clicked(move |_| {
        let current = hadj_left.value();
        let target = (current - hadj_left.page_size()).max(0.0);
        smooth_scroll_to(&hadj_left, target);
    });
    
    let hadj_right = hadj.clone();
    right_arrow.connect_clicked(move |_| {
        let current = hadj_right.value();
        let max = hadj_right.upper() - hadj_right.page_size();
        let target = (current + hadj_right.page_size()).min(max);
        smooth_scroll_to(&hadj_right, target);
    });

    let context = MainContext::default();
    let main_container_clone = main_container.clone();
    context.spawn_local(async move {
        ModuleInstance::create(state, button_container, drag_overlay, scrolled, main_container_clone, alignment_spacer).run_event_loop().await
    });

    Ok(())
}

fn smooth_scroll_to(adjustment: &gtk::Adjustment, target: f64) {
    let start = adjustment.value();
    let distance = target - start;
    
    if distance.abs() < 0.1 {
        adjustment.set_value(target);
        return;
    }
    
    let duration = 150.0;
    let start_time = std::time::Instant::now();
    let adj = adjustment.clone();
    
    gtk::glib::timeout_add_local(std::time::Duration::from_millis(16), move || {
        let elapsed = start_time.elapsed().as_millis() as f64;
        let progress = (elapsed / duration).min(1.0);
        
        let eased = ease_out_cubic(progress);
        let new_value = start + (distance * eased);
        
        adj.set_value(new_value);
        
        if progress >= 1.0 {
            gtk::glib::ControlFlow::Break
        } else {
            gtk::glib::ControlFlow::Continue
        }
    });
}

fn ease_out_cubic(t: f64) -> f64 {
    let t = t - 1.0;
    t * t * t + 1.0
}

struct ModuleInstance {
    buttons: BTreeMap<u64, WindowButton>,
    container: gtk::Box,
    drag_overlay: Overlay,
    scrolled_window: ScrolledWindow,
    main_container: gtk::Box,
    alignment_spacer: gtk::Box,
    previous_snapshot: Option<WindowSnapshot>,
    current_output: Option<String>,
    previous_focused: Option<u64>,
    display_filter: screen::DisplayFilter,
    state: SharedState,
    selection: SelectionState,
    audio_state: AudioState,
    window_pids: HashMap<u64, u32>,
    window_app_ids: HashMap<u64, String>,
    audio_window_state: HashSet<u64>,
}

impl ModuleInstance {
    fn create(state: SharedState, container: gtk::Box, drag_overlay: Overlay, scrolled_window: ScrolledWindow, main_container: gtk::Box, alignment_spacer: gtk::Box) -> Self {
        Self {
            buttons: BTreeMap::new(),
            container,
            drag_overlay,
            scrolled_window,
            main_container,
            alignment_spacer,
            previous_snapshot: None,
            current_output: None,
            previous_focused: None,
            display_filter: screen::DisplayFilter::ShowAll,
            state,
            selection: create_selection_state(),
            audio_state: AudioState::default(),
            window_pids: HashMap::new(),
            window_app_ids: HashMap::new(),
            audio_window_state: HashSet::new(),
        }
    }

    async fn run_event_loop(&mut self) {
        self.display_filter = self.determine_display_filter().await;

        let mut event_stream = Box::pin(self.state.create_event_stream());

        while let Some(event) = event_stream.next().await {
            match event {
                EventMessage::Notification(notif) => self.handle_notification(notif).await,
                EventMessage::AudioUpdate(state) => self.handle_audio_update(state),
                EventMessage::WindowUpdate(snapshot) => {
                    let filter = self.display_filter.clone();
                    self.handle_window_update(snapshot, filter).await;
                }
                EventMessage::Workspaces => {
                    let updated_filter = self.determine_display_filter().await;
                    let filter_changed = self.display_filter != updated_filter;
                    self.display_filter = updated_filter;

                    if filter_changed && self.update_output_and_resize().await {
                        if let Some(snapshot) = self.previous_snapshot.clone() {
                            self.handle_window_update(snapshot, screen::DisplayFilter::ShowAll).await;
                        }
                    }
                }
            }
        }
    }

    async fn update_output_and_resize(&mut self) -> bool {
        let new_output = self.get_current_output_name();

        if self.current_output.as_deref() != new_output.as_deref() {
            self.current_output = new_output.clone();

            let max_width = self.state.settings().max_taskbar_width_for_output(new_output.as_deref());
            self.main_container.set_size_request(max_width, -1);

            return true;
        }

        false
    }

    fn get_current_output_name(&self) -> Option<String> {
        let gdk_window = self.container.window()?;
        let display = gdk_window.display();
        let monitor = display.monitor_at_window(&gdk_window)?;
        
        let compositor = self.state.compositor().clone();
        let outputs = compositor.query_outputs().ok()?;
        
        for (output_name, output_info) in outputs.into_iter() {
            let match_result = screen::OutputMatcher::compare(&monitor, &output_info);
            if match_result == screen::OutputMatcher::all() {
                return Some(output_name);
            }
        }
        
        None
    }

    #[tracing::instrument(level = "DEBUG", skip(self))]
    async fn determine_display_filter(&self) -> screen::DisplayFilter {
        if self.state.settings().show_all_outputs() {
            return screen::DisplayFilter::ShowAll;
        }

        let compositor = self.state.compositor().clone();
        let available_outputs = match gio::spawn_blocking(move || compositor.query_outputs()).await {
            Ok(Ok(outputs)) => outputs,
            Ok(Err(e)) => {
                tracing::warn!(%e, "failed to query compositor outputs");
                return screen::DisplayFilter::ShowAll;
            }
            Err(_) => {
                tracing::error!("task spawning error");
                return screen::DisplayFilter::ShowAll;
            }
        };

        if available_outputs.len() == 1 {
            return screen::DisplayFilter::ShowAll;
        }

        let Some(gdk_window) = self.container.window() else {
            tracing::warn!("container has no GDK window");
            return screen::DisplayFilter::ShowAll;
        };

        let display = gdk_window.display();
        let Some(monitor) = display.monitor_at_window(&gdk_window) else {
            tracing::warn!(display = ?gdk_window.display(), geometry = ?gdk_window.geometry(), 
                "no monitor found for window");
            return screen::DisplayFilter::ShowAll;
        };

        for (output_name, output_info) in available_outputs.into_iter() {
            let match_result = screen::OutputMatcher::compare(&monitor, &output_info);
            if match_result == screen::OutputMatcher::all() {
                return screen::DisplayFilter::Only(output_name);
            }
        }

        tracing::warn!(?monitor, "no matching compositor output found");
        screen::DisplayFilter::ShowAll
    }

    #[tracing::instrument(level = "TRACE", skip(self))]
    async fn handle_notification(&mut self, notification: Box<NotificationData>) {
        let Some(windows) = &self.previous_snapshot else {
            return;
        };

        if let Some(mut process_id) = notification.get_process_id() {
            tracing::trace!(process_id, "attempting PID-based notification matching");

            let process_map = ProcessWindowMap::build(windows.iter());
            let mut matched = false;

            loop {
                if let Some(window) = process_map.lookup(process_id) {
                    if !window.is_focused {
                        if let Some(button) = self.buttons.get(&window.id) {
                            tracing::trace!(?button, ?window, process_id, 
                                "marking window as urgent via PID match");
                            button.mark_urgent();
                            matched = true;
                        }
                    }
                }

                match ProcessInfo::query(process_id).await {
                    Ok(ProcessInfo { parent_id }) => {
                        if let Some(parent) = parent_id {
                            process_id = parent;
                        } else {
                            break;
                        }
                    }
                    Err(e) => {
                        tracing::info!(process_id, %e, "process tree traversal ended");
                        break;
                    }
                }
            }

            if matched {
                return;
            }
        }

        tracing::trace!("no PID match found for notification");

        if !self.state.settings().notifications_use_desktop_entry() {
            tracing::trace!("desktop entry matching disabled");
            return;
        }

        let Some(desktop_entry) = &notification.get_notification().hints.desktop_entry else {
            tracing::trace!("no desktop entry in notification");
            return;
        };

        let fuzzy_enabled = self.state.settings().notifications_use_fuzzy_matching();
        let mut fuzzy_matches = Vec::new();

        let mapped_entry = self.state.settings()
            .notifications_app_map(desktop_entry)
            .unwrap_or(desktop_entry);
        let entry_lower = mapped_entry.to_lowercase();
        let entry_suffix = mapped_entry.split('.').next_back().unwrap_or_default().to_lowercase();

        let mut exact_match = false;
        for window in windows.iter() {
            let Some(app_identifier) = window.app_id.as_deref() else {
                continue;
            };

            if app_identifier == mapped_entry {
                if let Some(button) = self.buttons.get(&window.id) {
                    tracing::trace!(app_identifier, ?button, ?window, 
                        "exact app ID match for notification");
                    button.mark_urgent();
                    exact_match = true;
                }
            } else if fuzzy_enabled {
                if app_identifier.to_lowercase() == entry_lower {
                    tracing::trace!(app_identifier, ?window, 
                        "case-insensitive app ID match");
                    fuzzy_matches.push(window.id);
                } else if app_identifier.contains('.') {
                    if let Some(suffix) = app_identifier.split('.').next_back() {
                        if suffix.to_lowercase() == entry_suffix {
                            tracing::trace!(app_identifier, ?window, 
                                "suffix-based app ID match");
                            fuzzy_matches.push(window.id);
                        }
                    }
                }
            }
        }

        if !exact_match {
            for window_id in fuzzy_matches {
                if let Some(button) = self.buttons.get(&window_id) {
                    button.mark_urgent();
                }
            }
        }
    }

    #[tracing::instrument(level = "DEBUG", skip(self))]
    async fn handle_window_update(
        &mut self,
        snapshot: WindowSnapshot,
        filter: screen::DisplayFilter,
    ) {
        self.update_output_and_resize().await;

        let current_focused = snapshot.iter().find(|w| w.is_focused).map(|w| w.id);
        if current_focused != self.previous_focused {
            clear_selection(&self.selection);
            self.previous_focused = current_focused;
        }

        let mut removed_windows = self.buttons.keys().copied().collect::<BTreeSet<_>>();
        let config = self.state.settings();
        let mut new_button_added = false;

        for window in snapshot.iter().filter(|w| {
            if !filter.should_display(w.get_output().unwrap_or_default()) {
                return false;
            }
            if w.app_id.is_some() && config.should_ignore(w.app_id.as_deref(), w.title.as_deref(), w.workspace_id) {
                return false;
            }
            true
        }) {
            let initial_width = self.calculate_button_width((self.buttons.len() + 1) as i32);

            if let Some(pid) = window.pid.and_then(|p| u32::try_from(p).ok()) {
                self.window_pids.insert(window.id, pid);
            }
            if let Some(app_id) = &window.app_id {
                self.window_app_ids.insert(window.id, app_id.to_lowercase());
            }

            let button = self.buttons.entry(window.id).or_insert_with(|| {
                new_button_added = true;
                let btn = WindowButton::create(&self.state, window, self.selection.clone(), self.drag_overlay.clone());
                btn.get_widget().set_size_request(initial_width, -1);
                self.container.add(btn.get_widget());
                btn
            });

            button.update_focus(window.is_focused);
            button.update_title(window.title.as_deref());
            
            if window.is_focused {
                let button_widget = button.get_widget().clone();
                let scrolled = self.scrolled_window.clone();
                gtk::glib::idle_add_local_once(move || {
                    let allocation = button_widget.allocation();
                    let hadj = scrolled.hadjustment();
                    let button_x = allocation.x() as f64;
                    let button_width = allocation.width() as f64;
                    let current_scroll = hadj.value();
                    let page_size = hadj.page_size();
                    
                    let button_right = button_x + button_width;
                    let visible_right = current_scroll + page_size;
                    
                    if button_x < current_scroll {
                       hadj.set_value(button_x);
                    } else if button_right > visible_right {
                       hadj.set_value(button_right - page_size);
                    }
                });
            }

            removed_windows.remove(&window.id);
            self.container.reorder_child(button.get_widget(), -1);
        }

        for window_id in removed_windows {
            if let Some(button) = self.buttons.remove(&window_id) {
                self.container.remove(button.get_widget());
            }
            self.selection.borrow_mut().remove(&window_id);
            self.window_pids.remove(&window_id);
            self.window_app_ids.remove(&window_id);
        }

        if !self.buttons.is_empty() {
            let button_count = self.buttons.len() as i32;
            let final_width = self.calculate_button_width(button_count);

            for button in self.buttons.values() {
                button.get_widget().set_size_request(final_width, -1);
                button.resize_for_width(final_width);
            }

            let total_buttons_width = final_width * button_count;
            let page_size = self.scrolled_window.hadjustment().page_size() as i32;
            let total_limit = self.state.settings().max_taskbar_width_for_output(self.current_output.as_deref());
            let available_width = if page_size > 0 { page_size } else { total_limit };
            let spacer_width = match self.state.settings().button_alignment() {
                ButtonAlignment::Left => 0,
                ButtonAlignment::Center => ((available_width - total_buttons_width) / 2).max(0),
                ButtonAlignment::Right => (available_width - total_buttons_width).max(0),
            };
            self.alignment_spacer.set_size_request(spacer_width, 0);
            self.container.reorder_child(&self.alignment_spacer, 0);
        } else {
            self.alignment_spacer.set_size_request(0, 0);
        }

        self.container.show_all();

        if new_button_added {
            let scrolled = self.scrolled_window.clone();
            gtk::glib::timeout_add_local_once(std::time::Duration::from_millis(50), move || {
                let hadj = scrolled.hadjustment();
                hadj.set_value(hadj.upper() - hadj.page_size());
            });
        }

        self.previous_snapshot = Some(snapshot);
        self.update_button_audio_states();
    }

    fn calculate_button_width(&self, button_count: i32) -> i32 {
        let output = self.current_output.as_deref();
        let min_width = self.state.settings().min_button_width(output);
        let max_width = self.state.settings().max_button_width(output);
        let total_limit = self.state.settings().max_taskbar_width_for_output(output);

        if max_width * button_count > total_limit {
            (total_limit / button_count).max(min_width).max(1)
        } else {
            max_width
        }
    }

    fn handle_audio_update(&mut self, state: AudioState) {
        self.audio_state = state;
        self.update_button_audio_states();
    }

    fn update_button_audio_states(&mut self) {
        let mut pid_to_windows: HashMap<u32, Vec<u64>> = HashMap::new();
        for (&window_id, &pid) in &self.window_pids {
            pid_to_windows.entry(pid).or_default().push(window_id);
        }

        let mut new_audio_showing: HashSet<u64> = HashSet::new();

        for (window_id, button) in &self.buttons {
            let Some(&pid) = self.window_pids.get(window_id) else {
                button.update_audio_state(&[]);
                continue;
            };
            let inputs = self.audio_state.by_pid.get(&pid)
                .or_else(|| self.window_app_ids.get(window_id)
                    .and_then(|name| self.audio_state.by_name.get(name)));
            let Some(inputs) = inputs else {
                button.update_audio_state(&[]);
                continue;
            };

            let siblings = pid_to_windows.get(&pid).map_or(1, |v| v.len());
            let show = if siblings <= 1 {
                true
            } else if self.audio_window_state.contains(window_id) {
                true
            } else {
                let any_sibling_showing = pid_to_windows.get(&pid)
                    .map(|wids| wids.iter().any(|wid| self.audio_window_state.contains(wid)))
                    .unwrap_or(false);
                if any_sibling_showing {
                    false
                } else {
                    let focused_has_pid = self.previous_focused
                        .and_then(|fid| self.window_pids.get(&fid))
                        .map(|&fpid| fpid == pid)
                        .unwrap_or(false);
                    !focused_has_pid || Some(*window_id) == self.previous_focused
                }
            };

            if show {
                button.update_audio_state(inputs);
                new_audio_showing.insert(*window_id);
            } else {
                button.update_audio_state(&[]);
            }
        }

        self.audio_window_state = new_audio_showing;
    }
}

struct ProcessWindowMap<'a>(HashMap<i64, &'a WindowInfo>);

impl<'a> ProcessWindowMap<'a> {
    fn build(windows: impl Iterator<Item = &'a WindowInfo>) -> Self {
        Self(
            windows
                .filter_map(|w| w.pid.map(|pid| (i64::from(pid), w)))
                .collect()
        )
    }

    fn lookup(&self, pid: i64) -> Option<&'a WindowInfo> {
        self.0.get(&pid).copied()
    }
}
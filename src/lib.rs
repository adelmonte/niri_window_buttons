use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    sync::{Arc, LazyLock, Mutex},
};

use futures::StreamExt;
use settings::Settings;
use tracing_subscriber::{EnvFilter, fmt::format::FmtSpan};
use waybar_cffi::{
    Module,
    gtk::{self, Orientation, ReliefStyle, ScrolledWindow, gio, glib::MainContext, traits::{AdjustmentExt, BoxExt, ButtonExt, ContainerExt, ScrolledWindowExt, StyleContextExt, WidgetExt}},
    waybar_module,
};

mod compositor;
mod errors;
mod global;
mod icons;
mod notifications;
mod screen;
mod settings;
mod system;
mod widget;

use compositor::{WindowInfo, WindowSnapshot};
use errors::ModuleError;
use global::{EventMessage, SharedState};
use notifications::NotificationData;
use system::ProcessInfo;
use widget::WindowButton;

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

    let scrolled_clone = scrolled.clone();
    scrolled.connect_scroll_event(move |_, event| {
        use waybar_cffi::gtk::gdk::ScrollDirection;
        
        let hadj = scrolled_clone.hadjustment();
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
           _ => gtk::glib::Propagation::Proceed
        }
    });

    main_container.set_size_request(-1, -1);

    let button_container = gtk::Box::new(Orientation::Horizontal, 0);
    button_container.style_context().add_class("niri-window-buttons");
    scrolled.add(&button_container);
    
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
        ModuleInstance::create(state, button_container, scrolled, main_container_clone).run_event_loop().await
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
    scrolled_window: ScrolledWindow,
    main_container: gtk::Box,
    previous_snapshot: Option<WindowSnapshot>,
    current_output: Option<String>,
    state: SharedState,
}

impl ModuleInstance {
    fn create(state: SharedState, container: gtk::Box, scrolled_window: ScrolledWindow, main_container: gtk::Box) -> Self {
        Self {
            buttons: BTreeMap::new(),
            container,
            scrolled_window,
            main_container,
            previous_snapshot: None,
            current_output: None,
            state,
        }
    }

    async fn run_event_loop(&mut self) {
        let display_filter = Arc::new(Mutex::new(self.determine_display_filter().await));

        let mut event_stream = Box::pin(self.state.create_event_stream());

        while let Some(event) = event_stream.next().await {
            match event {
                EventMessage::Notification(notif) => self.handle_notification(notif).await,
                EventMessage::WindowUpdate(snapshot) => {
                    self.handle_window_update(snapshot, display_filter.clone()).await
                }
                EventMessage::Workspaces(_) => {
                    let updated_filter = self.determine_display_filter().await;
                    let filter_changed = {
                        let mut filter_lock = display_filter.lock().expect("display filter lock");
                        let changed = *filter_lock != updated_filter;
                        *filter_lock = updated_filter;
                        changed
                    };

                    if filter_changed && self.update_output_and_resize().await {
                        if let Some(snapshot) = self.previous_snapshot.clone() {
                            let filter = Arc::new(Mutex::new(screen::DisplayFilter::ShowAll));
                            self.handle_window_update(snapshot, filter).await;
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
        filter: Arc<Mutex<screen::DisplayFilter>>,
    ) {
        self.update_output_and_resize().await;

        let mut removed_windows = self.buttons.keys().copied().collect::<BTreeSet<_>>();
        let config = self.state.settings();
        let mut new_button_added = false;

        for window in snapshot.iter().filter(|w| {
            if !filter.lock().expect("filter lock").should_display(w.get_output().unwrap_or_default()) {
                return false;
            }
            if let Some(_app_id) = &w.app_id {
                if config.should_ignore(w.app_id.as_deref(), w.title.as_deref(), w.workspace_id) {
                   return false;
                }
            }
            true
        }) {
            let button_count = (self.buttons.len() + 1) as i32;
            let output = self.current_output.as_deref();
            let min_width = self.state.settings().min_button_width(output);
            let max_width = self.state.settings().max_button_width(output);
            let total_limit = self.state.settings().max_taskbar_width_for_output(output);
            
            let initial_width = if max_width * button_count > total_limit {
                (total_limit / button_count).max(min_width).max(1)
            } else {
                max_width
            }.max(1);

            let button = self.buttons.entry(window.id).or_insert_with(|| {
                new_button_added = true;
                let btn = WindowButton::create(&self.state, window);
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
        }

        if !self.buttons.is_empty() {
            let button_count = self.buttons.len() as i32;
            let output = self.current_output.as_deref();
            let min_width = self.state.settings().min_button_width(output);
            let max_width = self.state.settings().max_button_width(output);
            let total_limit = self.state.settings().max_taskbar_width_for_output(output);
            
            let final_width = if max_width * button_count > total_limit {
                (total_limit / button_count).max(min_width).max(1)
            } else {
                max_width
            }.max(1);

            for button in self.buttons.values() {
                button.get_widget().set_size_request(final_width, -1);
                button.resize_for_width(final_width);
            }
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
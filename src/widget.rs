use std::{cell::{Cell, RefCell}, collections::HashMap, fmt::Debug, path::PathBuf, process::Command, rc::Rc, time::{Duration, Instant}};
use waybar_cffi::gtk::{
    self as gtk, CssProvider, EventBox, IconLookupFlags, IconSize, IconTheme, Menu, MenuItem, Orientation, ReliefStyle,
    gdk_pixbuf::Pixbuf,
    prelude::{AdjustmentExt, BoxExt, ButtonExt, Cast, ContainerExt, CssProviderExt, DragContextExtManual, GdkPixbufExt, GtkMenuExt, GtkMenuItemExt, IconThemeExt, LabelExt, MenuShellExt, OverlayExt, StyleContextExt, WidgetExt, WidgetExtManual},
    DestDefaults, TargetEntry, TargetFlags,
};
use crate::audio::SinkInput;
use crate::global::SharedState;
use crate::settings::{DragStyle, ModifierKey, MultiSelectAction};

pub type SelectionState = Rc<RefCell<HashMap<u64, gtk::Button>>>;

pub fn create_selection_state() -> SelectionState {
    Rc::new(RefCell::new(HashMap::new()))
}

pub fn clear_selection(selection: &SelectionState) {
    let mut sel = selection.borrow_mut();
    for (_, button) in sel.drain() {
        button.style_context().remove_class("selected");
    }
}

pub struct WindowButton {
    app_id: Option<String>,
    gtk_button: gtk::Button,
    layout_box: gtk::Box,
    title_label: gtk::Label,
    audio_event_box: EventBox,
    audio_label: gtk::Label,
    audio_sink_inputs: Rc<RefCell<Vec<(u32, bool)>>>,
    display_titles: bool,
    state: SharedState,
    window_id: u64,
    title: Rc<RefCell<Option<String>>>,
    selection: SelectionState,
    tooltip_timeout: Rc<RefCell<Option<gtk::glib::SourceId>>>,
    skip_clicked: Rc<RefCell<bool>>,
    overlay: gtk::Overlay,
}

impl Debug for WindowButton {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WindowButton")
            .field("app_id", &self.app_id)
            .field("display_titles", &self.display_titles)
            .field("window_id", &self.window_id)
            .finish_non_exhaustive()
    }
}

thread_local! {
    static BUTTON_STYLES: CssProvider = {
        let provider = CssProvider::new();
        if let Err(e) = provider.load_from_data(include_bytes!("styles.css")) {
            tracing::error!(%e, "failed to load CSS");
        }
        provider
    };

    static ICON_THEME_INSTANCE: IconTheme = IconTheme::default().unwrap_or_default();

    static TASKBAR_ADJUSTMENT: std::cell::RefCell<Option<gtk::Adjustment>> = const { std::cell::RefCell::new(None) };
}

pub fn set_taskbar_adjustment(adj: gtk::Adjustment) {
    TASKBAR_ADJUSTMENT.with(|cell| {
        *cell.borrow_mut() = Some(adj);
    });
}

fn scroll_taskbar(delta: f64) {
    TASKBAR_ADJUSTMENT.with(|cell| {
        if let Some(ref adj) = *cell.borrow() {
            let step = adj.page_size() / 4.0;
            let max = adj.upper() - adj.page_size();
            let new_value = (adj.value() + delta * step).clamp(0.0, max);
            adj.set_value(new_value);
        }
    });
}


impl WindowButton {
    #[tracing::instrument(level = "TRACE", fields(app_id = &window.app_id))]
    pub fn create(state: &SharedState, window: &niri_ipc::Window, selection: SelectionState, overlay: gtk::Overlay) -> Self {
        let state_clone = state.clone();
        let display_titles = state.settings().show_window_titles();

        let icon_gap = state.settings().icon_spacing();
        let layout_box = gtk::Box::new(Orientation::Horizontal, icon_gap);

        let title_label = gtk::Label::new(None);
        let truncate_titles = state.settings().truncate_titles();
        if truncate_titles {
            title_label.set_ellipsize(gtk::pango::EllipsizeMode::End);
        } else {
            title_label.set_ellipsize(gtk::pango::EllipsizeMode::None);
        }
        title_label.set_xalign(0.0);

        let gtk_button = gtk::Button::new();
        gtk_button.set_always_show_image(true);
        gtk_button.set_relief(ReliefStyle::None);
        gtk_button.add(&layout_box);
        gtk_button.add_events(
            gtk::gdk::EventMask::SCROLL_MASK |
            gtk::gdk::EventMask::SMOOTH_SCROLL_MASK |
            gtk::gdk::EventMask::ENTER_NOTIFY_MASK |
            gtk::gdk::EventMask::LEAVE_NOTIFY_MASK
        );

        let max_width = state.settings().max_button_width(None);
        gtk_button.set_size_request(max_width, -1);

        if display_titles && truncate_titles {
            let icon_dim = state.settings().icon_size();
            let max_chars = (max_width - icon_dim - icon_gap - 16) / 8;
            title_label.set_max_width_chars(max_chars);
        }

        BUTTON_STYLES.with(|provider| {
            gtk_button.style_context().add_provider(provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        });

        let app_id = window.app_id.clone();
        let icon_location = app_id.as_deref().and_then(|id| state_clone.icon_resolver().resolve(id));

        let audio_label = gtk::Label::new(None);
        audio_label.show();
        let audio_event_box = EventBox::new();
        audio_event_box.add(&audio_label);
        audio_event_box.set_no_show_all(true);
        audio_event_box.style_context().add_class("audio-indicator");
        BUTTON_STYLES.with(|provider| {
            audio_event_box.style_context().add_provider(provider, gtk::STYLE_PROVIDER_PRIORITY_APPLICATION);
        });

        let audio_sink_inputs = Rc::new(RefCell::new(Vec::<(u32, bool)>::new()));

        let button = Self {
            app_id,
            gtk_button,
            layout_box,
            title_label,
            audio_event_box,
            audio_label,
            audio_sink_inputs,
            display_titles,
            state: state_clone,
            window_id: window.id,
            title: Rc::new(RefCell::new(window.title.clone())),
            selection,
            tooltip_timeout: Rc::new(RefCell::new(None)),
            skip_clicked: Rc::new(RefCell::new(false)),
            overlay,
        };

        button.setup_click_handlers(window.id);
        button.setup_audio_click_handler();
        button.setup_drag_reorder();
        button.setup_icon_rendering(icon_location);
        button.setup_tooltip();

        button
    }

    #[tracing::instrument(level = "TRACE")]
    pub fn update_focus(&self, is_focused: bool) {
        let style_ctx = self.gtk_button.style_context();
        if is_focused {
            style_ctx.add_class("focused");
            style_ctx.remove_class("urgent");
        } else {
            style_ctx.remove_class("focused");
        }
        self.gtk_button.queue_draw();
    }

    #[tracing::instrument(level = "TRACE")]
    pub fn update_title(&self, title: Option<&str>) {
        if let Some(t) = title {
            *self.title.borrow_mut() = Some(t.to_string());
        }

        if self.display_titles {
            if let Some(text) = title {
                let display_text = if self.state.settings().allow_title_linebreaks() {
                    text.to_string()
                } else {
                    text.replace('\n', " ").replace('\r', " ")
                };
                self.title_label.set_text(&display_text);
                self.title_label.show();
            } else {
                self.title_label.set_text("");
                self.title_label.hide();
            }
        }

        if let Some(app_id) = &self.app_id {
            if let Some(window_title) = title {
                let config = self.state.settings();
                let style_ctx = self.gtk_button.style_context();

                for class in config.get_app_classes(app_id) {
                    style_ctx.remove_class(class);
                }

                for class in config.match_app_rules(app_id, window_title) {
                    style_ctx.add_class(class);
                }
            }
        }
    }

    #[tracing::instrument(level = "TRACE")]
    pub fn mark_urgent(&self) {
        self.gtk_button.style_context().add_class("urgent");
    }

    pub fn get_widget(&self) -> &gtk::Button {
        &self.gtk_button
    }

    pub fn update_audio_state(&self, sink_inputs: &[SinkInput]) {
        if !self.state.settings().audio_indicator().enabled {
            return;
        }

        if sink_inputs.is_empty() {
            self.audio_event_box.hide();
            self.audio_sink_inputs.borrow_mut().clear();
            return;
        }

        let all_muted = sink_inputs.iter().all(|s| s.muted);
        let config = self.state.settings().audio_indicator();
        let icon = if all_muted { config.muted_icon.as_str() } else { config.playing_icon.as_str() };

        self.audio_label.set_text(icon);
        self.audio_event_box.show();

        *self.audio_sink_inputs.borrow_mut() = sink_inputs.iter().map(|s| (s.index, s.muted)).collect();
    }

    fn setup_audio_click_handler(&self) {
        let config = self.state.settings().audio_indicator();
        if !config.enabled || !config.clickable {
            return;
        }

        let sink_inputs_ref = self.audio_sink_inputs.clone();
        self.audio_event_box.connect_button_press_event(move |_, event| {
            if event.button() == 1 {
                let inputs = sink_inputs_ref.borrow().clone();
                if !inputs.is_empty() {
                    crate::audio::toggle_mute(&inputs);
                }
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });

        self.audio_event_box.set_tooltip_text(Some("Toggle mute"));
    }

    fn setup_click_handlers(&self, window_id: u64) {
        let state = self.state.clone();
        let button_ref = self.gtk_button.clone();
        let last_click_time = Rc::new(RefCell::new(Instant::now() - Duration::from_secs(1)));
        let skip_clicked_press = self.skip_clicked.clone();
        let skip_clicked_click = self.skip_clicked.clone();
        let app_id = self.app_id.clone();
        let title = self.title.clone();
        let selection_left = self.selection.clone();

        let title_clone = title.clone();
        self.gtk_button.connect_clicked(move |_| {
            if *skip_clicked_click.borrow() {
                *skip_clicked_click.borrow_mut() = false;
                return;
            }

            let is_currently_focused = button_ref.style_context().has_class("focused");
            let app_id_ref = app_id.as_deref();
            let title_ref = title_clone.borrow();
            let title_str = title_ref.as_deref();
            let actions = state.settings().get_click_actions(app_id_ref, title_str);

            if is_currently_focused {
                let mut last_click = last_click_time.borrow_mut();
                let now = Instant::now();
                let time_since_last = now.duration_since(*last_click);

                if time_since_last < Duration::from_millis(300) {
                    clear_selection(&selection_left);
                    Self::execute_click_action(&state, window_id, &actions.double_click, app_id_ref, title_str);
                    *last_click = Instant::now() - Duration::from_secs(1);
                } else {
                    clear_selection(&selection_left);
                    Self::execute_click_action(&state, window_id, &actions.left_click_focused, app_id_ref, title_str);
                    *last_click = now;
                }
            } else {
                clear_selection(&selection_left);
                Self::execute_click_action(&state, window_id, &actions.left_click_unfocused, app_id_ref, title_str);
            }
        });

        let state_press = self.state.clone();
        let button_ref_press = self.gtk_button.clone();
        let app_id_press = self.app_id.clone();
        let title_press = title.clone();
        let selection_press = self.selection.clone();
        let menu_self = self.clone_for_menu();

        self.gtk_button.connect_button_press_event(move |_btn, event| {
            if event.button() == 1 {
                let modifier_held = Self::check_modifier_static(state_press.settings().multi_select_modifier());
                if modifier_held {
                    *skip_clicked_press.borrow_mut() = true;
                    let mut sel = selection_press.borrow_mut();
                    if sel.contains_key(&window_id) {
                        sel.remove(&window_id);
                        button_ref_press.style_context().remove_class("selected");
                    } else {
                        sel.insert(window_id, button_ref_press.clone());
                        button_ref_press.style_context().add_class("selected");
                    }
                } else if state_press.settings().left_click_focus_on_press() {
                    let is_currently_focused = button_ref_press.style_context().has_class("focused");
                    if !is_currently_focused {
                        *skip_clicked_press.borrow_mut() = true;
                        clear_selection(&selection_press);
                        let app_id_ref = app_id_press.as_deref();
                        let title_ref = title_press.borrow();
                        let title_str = title_ref.as_deref();
                        let actions = state_press.settings().get_click_actions(app_id_ref, title_str);
                        Self::execute_click_action(&state_press, window_id, &actions.left_click_unfocused, app_id_ref, title_str);
                    }
                }
                gtk::glib::Propagation::Proceed
            } else if event.button() == 2 {
                let is_currently_focused = button_ref_press.style_context().has_class("focused");
                let app_id_ref = app_id_press.as_deref();
                let title_ref = title_press.borrow();
                let title_str = title_ref.as_deref();
                let actions = state_press.settings().get_click_actions(app_id_ref, title_str);
                let action = if is_currently_focused {
                    &actions.middle_click_focused
                } else {
                    &actions.middle_click_unfocused
                };
                if action.is_menu() {
                    menu_self.display_context_menu(window_id);
                } else {
                    Self::execute_click_action(&state_press, window_id, action, app_id_ref, title_str);
                }
                gtk::glib::Propagation::Stop
            } else if event.button() == 3 {
                let selection_count = selection_press.borrow().len();
                if selection_count > 0 {
                    menu_self.display_multi_select_menu();
                } else {
                    let is_currently_focused = button_ref_press.style_context().has_class("focused");
                    let app_id_ref = app_id_press.as_deref();
                    let title_ref = title_press.borrow();
                    let title_str = title_ref.as_deref();
                    let actions = state_press.settings().get_click_actions(app_id_ref, title_str);
                    let action = if is_currently_focused {
                        &actions.right_click_focused
                    } else {
                        &actions.right_click_unfocused
                    };
                    if action.is_menu() {
                        menu_self.display_context_menu(window_id);
                    } else {
                        Self::execute_click_action(&state_press, window_id, action, app_id_ref, title_str);
                    }
                }
                gtk::glib::Propagation::Stop
            } else {
                gtk::glib::Propagation::Proceed
            }
        });

        let state_scroll = self.state.clone();
        let app_id_scroll = self.app_id.clone();
        let title_scroll = title.clone();
        self.gtk_button.connect_scroll_event(move |_, event| {
            use waybar_cffi::gtk::gdk::ScrollDirection;

            let app_id_ref = app_id_scroll.as_deref();
            let title_ref = title_scroll.borrow();
            let title_str = title_ref.as_deref();
            let actions = state_scroll.settings().get_click_actions(app_id_ref, title_str);

            let (action, scroll_delta) = match event.direction() {
                ScrollDirection::Up => (&actions.scroll_up, -1.0),
                ScrollDirection::Down => (&actions.scroll_down, 1.0),
                ScrollDirection::Smooth => {
                    let (delta_x, delta_y) = event.delta();
                    let delta = if delta_x.abs() > delta_y.abs() { delta_x } else { delta_y };
                    if delta < -0.01 {
                        (&actions.scroll_up, delta)
                    } else if delta > 0.01 {
                        (&actions.scroll_down, delta)
                    } else {
                        return gtk::glib::Propagation::Stop;
                    }
                }
                _ => return gtk::glib::Propagation::Stop,
            };

            if !action.is_none() {
                Self::execute_click_action(&state_scroll, window_id, action, app_id_ref, title_str);
            } else {
                scroll_taskbar(scroll_delta);
            }
            gtk::glib::Propagation::Stop
        });
    }

    fn execute_click_action(
        state: &SharedState,
        window_id: u64,
        action: &crate::settings::ClickAction,
        app_id: Option<&str>,
        title: Option<&str>,
    ) {
        use crate::settings::ClickAction;
        match action {
            ClickAction::Action(window_action) => {
                Self::execute_action(state, window_id, window_action);
            }
            ClickAction::Command { command } => {
                Self::execute_command(command, window_id, app_id, title);
            }
        }
    }

    fn execute_action(state: &SharedState, window_id: u64, action: &crate::settings::WindowAction) {
        use crate::settings::WindowAction;
        match action {
            WindowAction::None => {}
            WindowAction::FocusWindow => {
                if let Err(e) = state.compositor().focus_window(window_id) {
                    tracing::warn!(%e, id = window_id, "focus failed");
                }
            }
            WindowAction::CloseWindow => {
                if let Err(e) = state.compositor().close_window(window_id) {
                    tracing::warn!(%e, id = window_id, "close failed");
                }
            }
            WindowAction::MaximizeColumn => {
                if let Err(e) = state.compositor().maximize_window_column(window_id) {
                    tracing::warn!(%e, id = window_id, "maximize column failed");
                }
            }
            WindowAction::MaximizeWindowToEdges => {
                if let Err(e) = state.compositor().maximize_window_to_edges(window_id) {
                    tracing::warn!(%e, id = window_id, "maximize to edges failed");
                }
            }
            WindowAction::CenterColumn => {
                if let Err(e) = state.compositor().center_column(window_id) {
                    tracing::warn!(%e, id = window_id, "center column failed");
                }
            }
            WindowAction::CenterWindow => {
                if let Err(e) = state.compositor().center_window(window_id) {
                    tracing::warn!(%e, id = window_id, "center window failed");
                }
            }
            WindowAction::CenterVisibleColumns => {
                if let Err(e) = state.compositor().center_visible_columns(window_id) {
                    tracing::warn!(%e, id = window_id, "center visible columns failed");
                }
            }
            WindowAction::ExpandColumnToAvailableWidth => {
                if let Err(e) = state.compositor().expand_column_to_available_width(window_id) {
                    tracing::warn!(%e, id = window_id, "expand column failed");
                }
            }
            WindowAction::FullscreenWindow => {
                if let Err(e) = state.compositor().fullscreen_window(window_id) {
                    tracing::warn!(%e, id = window_id, "fullscreen failed");
                }
            }
            WindowAction::ToggleWindowedFullscreen => {
                if let Err(e) = state.compositor().toggle_windowed_fullscreen(window_id) {
                    tracing::warn!(%e, id = window_id, "toggle windowed fullscreen failed");
                }
            }
            WindowAction::ToggleWindowFloating => {
                if let Err(e) = state.compositor().toggle_floating(window_id) {
                    tracing::warn!(%e, id = window_id, "toggle floating failed");
                }
            }
            WindowAction::ConsumeWindowIntoColumn => {
                if let Err(e) = state.compositor().consume_window_into_column(window_id) {
                    tracing::warn!(%e, id = window_id, "consume window into column failed");
                }
            }
            WindowAction::ExpelWindowFromColumn => {
                if let Err(e) = state.compositor().expel_window_from_column(window_id) {
                    tracing::warn!(%e, id = window_id, "expel window from column failed");
                }
            }
            WindowAction::ResetWindowHeight => {
                if let Err(e) = state.compositor().reset_window_height(window_id) {
                    tracing::warn!(%e, id = window_id, "reset window height failed");
                }
            }
            WindowAction::SwitchPresetColumnWidth => {
                if let Err(e) = state.compositor().switch_preset_column_width(window_id) {
                    tracing::warn!(%e, id = window_id, "switch preset column width failed");
                }
            }
            WindowAction::SwitchPresetWindowHeight => {
                if let Err(e) = state.compositor().switch_preset_window_height(window_id) {
                    tracing::warn!(%e, id = window_id, "switch preset window height failed");
                }
            }
            WindowAction::MoveWindowToWorkspaceDown => {
                if let Err(e) = state.compositor().move_window_to_workspace_down(window_id) {
                    tracing::warn!(%e, id = window_id, "move window to workspace down failed");
                }
            }
            WindowAction::MoveWindowToWorkspaceUp => {
                if let Err(e) = state.compositor().move_window_to_workspace_up(window_id) {
                    tracing::warn!(%e, id = window_id, "move window to workspace up failed");
                }
            }
            WindowAction::MoveWindowToMonitorLeft => {
                if let Err(e) = state.compositor().move_window_to_monitor_left(window_id) {
                    tracing::warn!(%e, id = window_id, "move window to monitor left failed");
                }
            }
            WindowAction::MoveWindowToMonitorRight => {
                if let Err(e) = state.compositor().move_window_to_monitor_right(window_id) {
                    tracing::warn!(%e, id = window_id, "move window to monitor right failed");
                }
            }
            WindowAction::ToggleColumnTabbedDisplay => {
                if let Err(e) = state.compositor().toggle_column_tabbed_display(window_id) {
                    tracing::warn!(%e, id = window_id, "toggle column tabbed display failed");
                }
            }
            WindowAction::FocusWorkspacePrevious => {
                if let Err(e) = state.compositor().focus_workspace_previous(window_id) {
                    tracing::warn!(%e, id = window_id, "focus workspace previous failed");
                }
            }
            WindowAction::MoveColumnLeft => {
                if let Err(e) = state.compositor().move_column_left(window_id) {
                    tracing::warn!(%e, id = window_id, "move column left failed");
                }
            }
            WindowAction::MoveColumnRight => {
                if let Err(e) = state.compositor().move_column_right(window_id) {
                    tracing::warn!(%e, id = window_id, "move column right failed");
                }
            }
            WindowAction::MoveColumnToFirst => {
                if let Err(e) = state.compositor().move_column_to_first(window_id) {
                    tracing::warn!(%e, id = window_id, "move column to first failed");
                }
            }
            WindowAction::MoveColumnToLast => {
                if let Err(e) = state.compositor().move_column_to_last(window_id) {
                    tracing::warn!(%e, id = window_id, "move column to last failed");
                }
            }
            WindowAction::MoveWindowDown => {
                if let Err(e) = state.compositor().move_window_down(window_id) {
                    tracing::warn!(%e, id = window_id, "move window down failed");
                }
            }
            WindowAction::MoveWindowUp => {
                if let Err(e) = state.compositor().move_window_up(window_id) {
                    tracing::warn!(%e, id = window_id, "move window up failed");
                }
            }
            WindowAction::MoveWindowDownOrToWorkspaceDown => {
                if let Err(e) = state.compositor().move_window_down_or_to_workspace_down(window_id) {
                    tracing::warn!(%e, id = window_id, "move window down or to workspace down failed");
                }
            }
            WindowAction::MoveWindowUpOrToWorkspaceUp => {
                if let Err(e) = state.compositor().move_window_up_or_to_workspace_up(window_id) {
                    tracing::warn!(%e, id = window_id, "move window up or to workspace up failed");
                }
            }
            WindowAction::MoveColumnLeftOrToMonitorLeft => {
                if let Err(e) = state.compositor().move_column_left_or_to_monitor_left(window_id) {
                    tracing::warn!(%e, id = window_id, "move column left or to monitor left failed");
                }
            }
            WindowAction::MoveColumnRightOrToMonitorRight => {
                if let Err(e) = state.compositor().move_column_right_or_to_monitor_right(window_id) {
                    tracing::warn!(%e, id = window_id, "move column right or to monitor right failed");
                }
            }
            WindowAction::Menu => {}
        }
    }

    #[tracing::instrument(level = "TRACE", skip(self))]
    fn display_context_menu(&self, window_id: u64) {
        let menu = Menu::new();
        menu.set_reserve_toggle_size(false);

        let menu_items = self.state.settings().context_menu();

        for menu_item in menu_items {
            let item = MenuItem::with_label(&menu_item.label);
            menu.append(&item);

            let state = self.state.clone();
            let action = menu_item.action.clone();
            let command = menu_item.command.clone();
            let app_id = self.app_id.clone();
            let title = self.title.borrow().clone();
            item.connect_activate(move |_| {
                if let Some(ref cmd) = command {
                    Self::execute_command(cmd, window_id, app_id.as_deref(), title.as_deref());
                } else if let Some(ref act) = action {
                    Self::execute_action(&state, window_id, act);
                }
            });
        }

        menu.show_all();
        menu.popup_at_pointer(None);
    }

    fn execute_command(command: &str, window_id: u64, app_id: Option<&str>, title: Option<&str>) {
        let cmd = command
            .replace("{window_id}", &window_id.to_string())
            .replace("{app_id}", app_id.unwrap_or(""))
            .replace("{title}", title.unwrap_or(""));

        std::thread::spawn(move || {
            if let Err(e) = Command::new("sh").arg("-c").arg(&cmd).spawn() {
                tracing::error!(%e, "failed to execute command: {}", cmd);
            }
        });
    }

    fn check_modifier_static(modifier: ModifierKey) -> bool {
        use evdev::Key;

        let keys_to_check: &[Key] = match modifier {
            ModifierKey::Ctrl => &[Key::KEY_LEFTCTRL, Key::KEY_RIGHTCTRL],
            ModifierKey::Shift => &[Key::KEY_LEFTSHIFT, Key::KEY_RIGHTSHIFT],
            ModifierKey::Alt => &[Key::KEY_LEFTALT, Key::KEY_RIGHTALT],
            ModifierKey::Super => &[Key::KEY_LEFTMETA, Key::KEY_RIGHTMETA],
        };

        evdev::enumerate()
            .filter_map(|(_, device)| {
                if device.supported_keys().map_or(false, |keys| keys.contains(Key::KEY_LEFTCTRL)) {
                    Some(device)
                } else {
                    None
                }
            })
            .any(|device| {
                if let Ok(key_state) = device.get_key_state() {
                    keys_to_check.iter().any(|&key| key_state.contains(key))
                } else {
                    false
                }
            })
    }

    fn display_multi_select_menu(&self) {
        let menu = Menu::new();
        menu.set_reserve_toggle_size(false);

        let menu_items = self.state.settings().multi_select_menu();
        let selected_windows: Vec<u64> = self.selection.borrow().keys().copied().collect();

        for menu_item in menu_items {
            let item = MenuItem::with_label(&menu_item.label);
            menu.append(&item);

            let state = self.state.clone();
            let selection = self.selection.clone();
            let action = menu_item.action.clone();
            let command = menu_item.command.clone();
            let windows = selected_windows.clone();
            item.connect_activate(move |_| {
                if let Some(ref cmd) = command {
                    let windows_str = windows.iter().map(|w| w.to_string()).collect::<Vec<_>>().join(",");
                    let cmd = cmd.replace("{window_ids}", &windows_str);
                    std::thread::spawn(move || {
                        if let Err(e) = Command::new("sh").arg("-c").arg(&cmd).spawn() {
                            tracing::error!(%e, "failed to execute multi-select command");
                        }
                    });
                } else if let Some(ref act) = action {
                    Self::execute_multi_select_action(&state, &windows, act);
                }
                clear_selection(&selection);
            });
        }

        menu.show_all();
        menu.popup_at_pointer(None);
    }

    fn execute_multi_select_action(state: &SharedState, window_ids: &[u64], action: &MultiSelectAction) {
        for &window_id in window_ids {
            let result = match action {
                MultiSelectAction::CloseWindows => state.compositor().close_window(window_id),
                MultiSelectAction::MoveToWorkspaceUp => state.compositor().move_window_to_workspace_up(window_id),
                MultiSelectAction::MoveToWorkspaceDown => state.compositor().move_window_to_workspace_down(window_id),
                MultiSelectAction::MoveToMonitorLeft => state.compositor().move_window_to_monitor_left(window_id),
                MultiSelectAction::MoveToMonitorRight => state.compositor().move_window_to_monitor_right(window_id),
                MultiSelectAction::MoveToMonitorUp => state.compositor().move_window_to_monitor_up(window_id),
                MultiSelectAction::MoveToMonitorDown => state.compositor().move_window_to_monitor_down(window_id),
                MultiSelectAction::MoveColumnLeft => state.compositor().move_column_left(window_id),
                MultiSelectAction::MoveColumnRight => state.compositor().move_column_right(window_id),
                MultiSelectAction::ToggleFloating => state.compositor().toggle_floating(window_id),
                MultiSelectAction::FullscreenWindows => state.compositor().fullscreen_window(window_id),
                MultiSelectAction::MaximizeColumns => state.compositor().maximize_window_column(window_id),
                MultiSelectAction::CenterColumns => state.compositor().center_column(window_id),
                MultiSelectAction::ConsumeIntoColumn => state.compositor().consume_window_into_column(window_id),
                MultiSelectAction::ToggleTabbedDisplay => state.compositor().toggle_column_tabbed_display(window_id),
            };
            if let Err(e) = result {
                tracing::warn!(%e, id = window_id, "multi-select action failed");
            }
        }
    }

    fn clone_for_menu(&self) -> Self {
        Self {
            app_id: self.app_id.clone(),
            gtk_button: self.gtk_button.clone(),
            layout_box: self.layout_box.clone(),
            title_label: self.title_label.clone(),
            audio_event_box: self.audio_event_box.clone(),
            audio_label: self.audio_label.clone(),
            audio_sink_inputs: self.audio_sink_inputs.clone(),
            display_titles: self.display_titles,
            state: self.state.clone(),
            window_id: self.window_id,
            title: self.title.clone(),
            selection: self.selection.clone(),
            tooltip_timeout: self.tooltip_timeout.clone(),
            skip_clicked: self.skip_clicked.clone(),
            overlay: self.overlay.clone(),
        }
    }

    fn setup_drag_reorder(&self) {
        match self.state.settings().drag_style() {
            DragStyle::Browser => self.setup_drag_reorder_browser(),
            DragStyle::Classic => self.setup_drag_reorder_classic(),
        }
    }

    fn setup_drag_reorder_browser(&self) {
        self.gtk_button.add_events(gtk::gdk::EventMask::BUTTON1_MOTION_MASK);

        let ext_targets = vec![
            TargetEntry::new("text/uri-list", TargetFlags::OTHER_APP, 1),
            TargetEntry::new("text/plain", TargetFlags::OTHER_APP, 2),
        ];
        self.gtk_button.drag_dest_set(
            DestDefaults::MOTION | DestDefaults::HIGHLIGHT,
            &ext_targets,
            gtk::gdk::DragAction::COPY,
        );

        let drag_start: Rc<RefCell<Option<(f64, i32)>>> = Rc::new(RefCell::new(None));
        let is_dragging: Rc<RefCell<bool>> = Rc::new(RefCell::new(false));
        let committed_offset: Rc<Cell<i32>> = Rc::new(Cell::new(0));
        let ghost_pos: Rc<Cell<f64>> = Rc::new(Cell::new(0.0));
        let ghost_surface: Rc<RefCell<Option<gtk::cairo::ImageSurface>>> = Rc::new(RefCell::new(None));
        let ghost_drawing: Rc<RefCell<Option<gtk::DrawingArea>>> = Rc::new(RefCell::new(None));

        let window_id = self.window_id;
        let state = self.state.clone();
        let overlay = self.overlay.clone();
        let skip = self.skip_clicked.clone();
        let drag_start_press = drag_start.clone();
        let skip_for_press = skip.clone();
        self.gtk_button.connect_button_press_event(move |btn, event| {
            if event.button() == 1 && !*skip_for_press.borrow() {
                if let Some(parent) = btn.parent() {
                    if let Ok(container) = parent.downcast::<gtk::Box>() {
                        let (ex, _) = event.position();
                        if let Some((cx, _)) = btn.translate_coordinates(&container, ex as i32, 0) {
                            *drag_start_press.borrow_mut() = Some((cx as f64, container.child_position(btn)));
                        }
                    }
                }
            }
            gtk::glib::Propagation::Proceed
        });

        let drag_start_motion = drag_start.clone();
        let is_dragging_motion = is_dragging.clone();
        let committed_offset_motion = committed_offset.clone();
        let ghost_pos_motion = ghost_pos.clone();
        let ghost_surface_motion = ghost_surface.clone();
        let ghost_drawing_motion = ghost_drawing.clone();
        let overlay_motion = overlay.clone();
        let skip_motion = skip.clone();
        self.gtk_button.connect_motion_notify_event(move |btn, event| {
            let Some(parent) = btn.parent() else { return gtk::glib::Propagation::Proceed; };
            let Ok(container) = parent.downcast::<gtk::Box>() else { return gtk::glib::Propagation::Proceed; };
            let (ex, _) = event.position();
            let Some((cx, _)) = btn.translate_coordinates(&container, ex as i32, 0) else {
                return gtk::glib::Propagation::Proceed;
            };
            let cx = cx as f64;

            if !*is_dragging_motion.borrow() {
                if let Some((press_cx, _)) = *drag_start_motion.borrow() {
                    if (cx - press_cx).abs() > 5.0 {
                        *is_dragging_motion.borrow_mut() = true;
                        *skip_motion.borrow_mut() = true;

                        let alloc = btn.allocation();
                        let scale = btn.scale_factor();
                        if let Ok(surface) = gtk::cairo::ImageSurface::create(
                            gtk::cairo::Format::ARgb32,
                            alloc.width() * scale,
                            alloc.height() * scale,
                        ) {
                            surface.set_device_scale(scale as f64, scale as f64);
                            if let Ok(cr) = gtk::cairo::Context::new(&surface) {
                                btn.draw(&cr);
                            }
                            *ghost_surface_motion.borrow_mut() = Some(surface);
                        }

                        let da = gtk::DrawingArea::new();
                        da.set_halign(gtk::Align::Fill);
                        da.set_valign(gtk::Align::Fill);
                        let gpos = ghost_pos_motion.clone();
                        let gsrf = ghost_surface_motion.clone();
                        da.connect_draw(move |_, cr| {
                            if let Some(ref surface) = *gsrf.borrow() {
                                let _ = cr.set_source_surface(surface, gpos.get(), 0.0);
                                let _ = cr.paint();
                            }
                            gtk::glib::Propagation::Proceed
                        });

                        overlay_motion.add_overlay(&da);
                        overlay_motion.set_overlay_pass_through(&da, true);
                        da.show();
                        *ghost_drawing_motion.borrow_mut() = Some(da);

                        btn.set_opacity(0.0);
                    }
                }
            }

            if *is_dragging_motion.borrow() {
                if let Some((press_cx, start_slot)) = *drag_start_motion.borrow() {
                    let btn_w = btn.allocation().width() as f64;
                    if btn_w == 0.0 { return gtk::glib::Propagation::Proceed; }

                    let displacement = cx - press_cx;

                    let ghost_x = (cx - btn_w / 2.0).max(0.0);
                    ghost_pos_motion.set(ghost_x);
                    if let Some(da) = ghost_drawing_motion.borrow().as_ref() {
                        da.queue_draw();
                    }

                    let raw_offset = displacement / btn_w;
                    let current = committed_offset_motion.get();
                    let hysteresis = 5.0 / btn_w;
                    let slot_offset = if raw_offset > current as f64 + 0.5 {
                        (raw_offset + 0.5).floor() as i32
                    } else if raw_offset < current as f64 - 0.5 {
                        (raw_offset - 0.5).ceil() as i32
                    } else {
                        current
                    };

                    if slot_offset != current {
                        let threshold = if slot_offset > current {
                            current as f64 + 0.5 + hysteresis
                        } else {
                            current as f64 - 0.5 - hysteresis
                        };
                        let crossed = if slot_offset > current {
                            raw_offset >= threshold
                        } else {
                            raw_offset <= threshold
                        };

                        if crossed {
                            committed_offset_motion.set(slot_offset);
                            let n = container.children().len() as i32;
                            let target_slot = (start_slot + slot_offset).clamp(1, n - 1);
                            let current_pos = container.child_position(btn);
                            if current_pos != target_slot {
                                container.reorder_child(btn, target_slot);
                            }
                        }
                    }
                }
            }

            gtk::glib::Propagation::Proceed
        });

        let drag_start_release = drag_start.clone();
        let is_dragging_release = is_dragging.clone();
        let ghost_drawing_release = ghost_drawing.clone();
        let ghost_surface_release = ghost_surface.clone();
        let overlay_release = overlay.clone();
        self.gtk_button.connect_button_release_event(move |btn, event| {
            if event.button() == 1 {
                let was_dragging = *is_dragging_release.borrow();
                *is_dragging_release.borrow_mut() = false;

                if was_dragging {
                    if let Some(da) = ghost_drawing_release.borrow_mut().take() {
                        overlay_release.remove(&da);
                    }
                    ghost_surface_release.borrow_mut().take();

                    btn.set_opacity(1.0);

                    if let Some(container) = btn.parent().and_then(|p| p.downcast::<gtk::Box>().ok()) {
                        if let Some((_, start_slot)) = *drag_start_release.borrow() {
                            let final_slot = container.child_position(btn);
                            let delta = final_slot - start_slot;
                            if delta != 0 {
                                let keep_stacked = Self::check_modifier_static(state.settings().multi_select_modifier());
                                if let Err(e) = state.compositor().reposition_window(window_id, delta, keep_stacked) {
                                    tracing::error!("reposition failed: {}", e);
                                }
                            }
                        }
                    }
                }

                *drag_start_release.borrow_mut() = None;
            }
            gtk::glib::Propagation::Proceed
        });

        let hover_timeout: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));
        let timeout_leave = hover_timeout.clone();
        let state_motion = self.state.clone();
        let button_motion = self.gtk_button.clone();
        let window_id_motion = self.window_id;
        let timeout_motion = hover_timeout.clone();

        self.gtk_button.connect_drag_motion(move |_, ctx, _, _, _| {
            if ctx.drag_get_source_widget().is_some() { return true; }
            if state_motion.settings().drag_hover_focus() && timeout_motion.borrow().is_none() {
                button_motion.style_context().add_class("drag-over");
                let state = state_motion.clone();
                let wid = window_id_motion;
                let delay = state_motion.settings().drag_hover_focus_delay();
                let timeout_ref = timeout_motion.clone();
                let source_id = gtk::glib::timeout_add_local_once(
                    Duration::from_millis(delay as u64),
                    move || {
                        if let Err(e) = state.compositor().focus_window(wid) {
                            tracing::error!("drag hover focus failed: {}", e);
                        }
                        timeout_ref.borrow_mut().take();
                    }
                );
                *timeout_motion.borrow_mut() = Some(source_id);
            }
            true
        });

        let button_leave = self.gtk_button.clone();
        self.gtk_button.connect_drag_leave(move |_, _, _| {
            button_leave.style_context().remove_class("drag-over");
            if let Some(id) = timeout_leave.borrow_mut().take() {
                id.remove();
            }
        });
    }

    fn setup_drag_reorder_classic(&self) {
        let internal_targets = vec![TargetEntry::new("text/plain", TargetFlags::SAME_APP, 0)];

        self.gtk_button.drag_source_set(
            gtk::gdk::ModifierType::BUTTON1_MASK,
            &internal_targets,
            gtk::gdk::DragAction::MOVE,
        );

        let dest_targets = vec![
            TargetEntry::new("text/plain", TargetFlags::SAME_APP, 0),
            TargetEntry::new("text/uri-list", TargetFlags::OTHER_APP, 1),
            TargetEntry::new("text/plain", TargetFlags::OTHER_APP, 2),
        ];

        self.gtk_button.drag_dest_set(
            DestDefaults::MOTION | DestDefaults::HIGHLIGHT,
            &dest_targets,
            gtk::gdk::DragAction::MOVE | gtk::gdk::DragAction::COPY,
        );

        let initial_position = Rc::new(RefCell::new(0));
        let pos_for_begin = initial_position.clone();

        self.gtk_button.connect_drag_begin(move |widget, _| {
            if let Some(parent) = widget.parent() {
                if let Ok(container) = parent.downcast::<gtk::Box>() {
                    *pos_for_begin.borrow_mut() = container.child_position(widget);
                }
            }
            widget.style_context().add_class("dragging");
        });

        let window_id = self.window_id;
        self.gtk_button.connect_drag_data_get(move |_, _, data, _, _| {
            data.set_text(&window_id.to_string());
        });

        let button_for_end = self.gtk_button.clone();
        let skip_clicked_drag = self.skip_clicked.clone();
        self.gtk_button.connect_drag_end(move |_, _| {
            button_for_end.style_context().remove_class("dragging");
            *skip_clicked_drag.borrow_mut() = false;
        });

        let hover_timeout: Rc<RefCell<Option<gtk::glib::SourceId>>> = Rc::new(RefCell::new(None));
        let timeout_for_motion = hover_timeout.clone();
        let timeout_for_leave = hover_timeout.clone();
        let timeout_for_drop = hover_timeout.clone();

        let state_for_motion = self.state.clone();
        let window_id_for_motion = self.window_id;
        let button_for_motion = self.gtk_button.clone();
        self.gtk_button.connect_drag_motion(move |widget, ctx, _x, _y, _time| {
            let is_external = ctx.drag_get_source_widget().is_none();

            if is_external {
                if state_for_motion.settings().drag_hover_focus() && timeout_for_motion.borrow().is_none() {
                    button_for_motion.style_context().add_class("drag-over");

                    let state = state_for_motion.clone();
                    let wid = window_id_for_motion;
                    let delay = state_for_motion.settings().drag_hover_focus_delay();
                    let timeout_ref = timeout_for_motion.clone();

                    let source_id = gtk::glib::timeout_add_local_once(
                        Duration::from_millis(delay as u64),
                        move || {
                            if let Err(e) = state.compositor().focus_window(wid) {
                                tracing::error!("failed to focus window on drag hover: {}", e);
                            }
                            timeout_ref.borrow_mut().take();
                        }
                    );

                    *timeout_for_motion.borrow_mut() = Some(source_id);
                }
                return true;
            }

            if let Some(source) = ctx.drag_get_source_widget() {
                if source != *widget {
                    if let Some(parent) = widget.parent() {
                        if let Ok(container) = parent.downcast::<gtk::Box>() {
                            let source_pos = container.child_position(&source);
                            let target_pos = container.child_position(widget);

                            if source_pos != target_pos {
                                container.reorder_child(&source, target_pos);
                            }
                        }
                    }
                }
            }
            true
        });

        let button_for_leave = self.gtk_button.clone();
        self.gtk_button.connect_drag_leave(move |_, _, _| {
            button_for_leave.style_context().remove_class("drag-over");

            if let Some(timeout_id) = timeout_for_leave.borrow_mut().take() {
                timeout_id.remove();
            }
        });

        let state_for_drop = self.state.clone();
        let pos_for_drop = initial_position.clone();
        self.gtk_button.connect_drag_drop(move |widget, ctx, _x, _y, time| {
            if let Some(timeout_id) = timeout_for_drop.borrow_mut().take() {
                timeout_id.remove();
            }

            let is_internal = ctx.drag_get_source_widget().is_some();

            if is_internal {
                let target = widget.drag_dest_find_target(ctx, None);
                if let Some(target) = target {
                    widget.drag_get_data(ctx, &target, time);
                    return true;
                }
            }

            false
        });

        let state = state_for_drop;
        self.gtk_button.connect_drag_data_received(move |_widget, ctx, _, _, data, _, time| {
            if let Some(text) = data.text() {
                if let Ok(dragged_window_id) = text.parse::<u64>() {
                    if let Some(source) = ctx.drag_get_source_widget() {
                        if let Some(parent) = source.parent() {
                            if let Ok(container) = parent.downcast::<gtk::Box>() {
                                let start_pos = *pos_for_drop.borrow();
                                let end_pos = container.child_position(&source);
                                let delta = end_pos - start_pos;

                                let keep_stacked = Self::check_modifier_static(state.settings().multi_select_modifier());

                                match state.compositor().reposition_window(dragged_window_id, delta, keep_stacked) {
                                    Ok(()) => {
                                        ctx.drag_finish(true, false, time);
                                        return;
                                    }
                                    Err(e) => {
                                        tracing::error!("reposition failed: {}", e);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            ctx.drag_finish(false, false, time);
        });
    }

    #[tracing::instrument(level = "TRACE")]
    fn setup_icon_rendering(&self, icon_path: Option<PathBuf>) {
        let last_allocation = RefCell::new(None);
        let container = self.layout_box.clone();
        let label = self.title_label.clone();
        let audio_event_box = self.audio_event_box.clone();
        let show_titles = self.display_titles;
        let icon_dimension = self.state.settings().icon_size();

        self.gtk_button.connect_size_allocate(move |button, allocation| {
            let mut needs_render = container.children().is_empty();

            if !needs_render {
                if let Some(prev_alloc) = last_allocation.take() {
                    if &prev_alloc != allocation {
                        needs_render = true;
                    }
                } else {
                    needs_render = true;
                }

                last_allocation.replace(Some(*allocation));
            }

            if needs_render {
                let dimension = icon_dimension;

                let icon_image = Self::load_icon_image(icon_path.as_ref(), button, dimension)
                    .unwrap_or_else(|| {
                        static FALLBACK: &str = "application-x-executable";

                        ICON_THEME_INSTANCE.with(|theme| {
                            theme.lookup_icon_for_scale(
                                FALLBACK,
                                dimension,
                                button.scale_factor(),
                                IconLookupFlags::empty(),
                            )
                        })
                        .and_then(|info| Self::load_icon_image(info.filename().as_ref(), button, dimension))
                        .unwrap_or_else(|| gtk::Image::from_icon_name(Some(FALLBACK), IconSize::Button))
                    });

                let container_copy = container.clone();
                let label_copy = label.clone();
                let audio_copy = audio_event_box.clone();
                let button_copy = button.clone();
                gtk::glib::source::idle_add_local_once(move || {
                    for child in container_copy.children() {
                        container_copy.remove(&child);
                    }

                    container_copy.pack_start(&icon_image, false, false, 0);
                    container_copy.pack_start(&audio_copy, false, false, 0);

                    if show_titles {
                        container_copy.pack_start(&label_copy, true, true, 0);
                    }

                    container_copy.show_all();
                    button_copy.show_all();
                });
            }
        });
    }

    fn load_icon_image(
        path: Option<&PathBuf>,
        button: &gtk::Button,
        size: i32,
    ) -> Option<gtk::Image> {
        let scaled_size = size * button.scale_factor();

        path.and_then(|p| match Pixbuf::from_file_at_scale(p, scaled_size, scaled_size, true) {
            Ok(pixbuf) => Some(pixbuf),
            Err(e) => {
                tracing::info!(%e, ?p, "icon load failed");
                None
            }
        })
        .and_then(|pixbuf| pixbuf.create_surface(0, button.window().as_ref()))
        .map(|surface| gtk::Image::from_surface(Some(&surface)))
    }

    fn setup_tooltip(&self) {
        if !self.state.settings().show_tooltip() {
            return;
        }

        let delay = self.state.settings().tooltip_delay();
        let title = self.title.clone();
        let tooltip_timeout = self.tooltip_timeout.clone();

        self.gtk_button.connect_enter_notify_event(move |btn, _| {
            let title_clone = title.clone();
            let btn_clone = btn.clone();
            let timeout_ref = tooltip_timeout.clone();

            let source_id = gtk::glib::timeout_add_local_once(
                Duration::from_millis(delay as u64),
                move || {
                    if let Some(ref text) = *title_clone.borrow() {
                        btn_clone.set_tooltip_text(Some(text));
                        btn_clone.trigger_tooltip_query();
                    }
                    timeout_ref.borrow_mut().take();
                }
            );

            *tooltip_timeout.borrow_mut() = Some(source_id);
            gtk::glib::Propagation::Proceed
        });

        let tooltip_timeout_leave = self.tooltip_timeout.clone();
        let button_leave = self.gtk_button.clone();
        self.gtk_button.connect_leave_notify_event(move |_, _| {
            if let Some(timeout_id) = tooltip_timeout_leave.borrow_mut().take() {
                timeout_id.remove();
            }
            button_leave.set_tooltip_text(None);
            gtk::glib::Propagation::Proceed
        });
    }

    pub fn resize_for_width(&self, width: i32) {
        if self.display_titles && self.state.settings().truncate_titles() {
            let icon_dim = self.state.settings().icon_size();
            let icon_gap = self.state.settings().icon_spacing();
            let max_chars = ((width - icon_dim - icon_gap - 16) / 8).max(0);
            self.title_label.set_max_width_chars(max_chars);

            if max_chars == 0 {
                self.title_label.hide();
            }
        }
    }
}
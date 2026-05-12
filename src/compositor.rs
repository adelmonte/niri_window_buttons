use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ops::Deref,
};
use niri_ipc::{Action, Event, Output, Reply, Request, Workspace, socket::Socket};
use waybar_cffi::gtk::glib;
use crate::{errors::ModuleError, settings::Settings};

#[derive(Debug, Clone)]
pub struct CompositorClient {
    settings: Settings,
}

impl CompositorClient {
    pub fn create(settings: Settings) -> Self {
        Self { settings }
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn focus_window(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::FocusWindow { id: window_id })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn close_window(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::CloseWindow { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn maximize_window_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MaximizeColumn {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn maximize_window_to_edges(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        send_action(Action::MaximizeWindowToEdges { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn center_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::CenterColumn {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn fullscreen_window(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::FullscreenWindow { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_floating(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::ToggleWindowFloating { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn center_window(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::CenterWindow { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn center_visible_columns(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::CenterVisibleColumns {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn expand_column_to_available_width(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::ExpandColumnToAvailableWidth {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_windowed_fullscreen(&self, window_id: u64) -> Result<(), ModuleError> {
        send_action(Action::ToggleWindowedFullscreen { id: Some(window_id) })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn consume_window_into_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::ConsumeWindowIntoColumn {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn expel_window_from_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::ExpelWindowFromColumn {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn reset_window_height(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::ResetWindowHeight { id: None })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn switch_preset_column_width(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::SwitchPresetColumnWidth {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn switch_preset_window_height(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::SwitchPresetWindowHeight { id: None })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_workspace_down(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToWorkspaceDown { focus: false })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_workspace_up(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToWorkspaceUp { focus: false })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_left(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToMonitorLeft {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_right(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToMonitorRight {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_up(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToMonitorUp {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_down(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowToMonitorDown {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_column_tabbed_display(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::ToggleColumnTabbedDisplay {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn focus_workspace_previous(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::FocusWorkspacePrevious {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_left(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnLeft {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_right(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnRight {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_to_first(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnToFirst {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_to_last(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnToLast {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_down(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowDown {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_up(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowUp {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_down_or_to_workspace_down(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowDownOrToWorkspaceDown {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_up_or_to_workspace_up(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveWindowUpOrToWorkspaceUp {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_left_or_to_monitor_left(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnLeftOrToMonitorLeft {})
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_right_or_to_monitor_right(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focused_action(window_id, Action::MoveColumnRightOrToMonitorRight {})
    }

    fn focused_action(&self, window_id: u64, action: Action) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        send_action(action)
    }

    pub fn query_outputs(&self) -> Result<HashMap<String, Output>, ModuleError> {
        let response = send_request(Request::Outputs)?;
        match response {
            Ok(niri_ipc::Response::Outputs(outputs)) => Ok(outputs),
            Ok(other) => Err(ModuleError::unexpected_response("Outputs", other)),
            Err(msg) => Err(ModuleError::CompositorReply(msg)),
        }
    }

    pub fn only_current_workspace(&self) -> bool {
        self.settings.only_current_workspace()
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn reposition_window(&self, window_id: u64, position_delta: i32, keep_stacked: bool) -> Result<(), ModuleError> {
        if position_delta == 0 {
            return Ok(());
        }

        let all_windows = query_windows()?;
        let Some(target) = all_windows.iter().find(|w| w.id == window_id) else {
            tracing::warn!("target window not found in window list");
            return Ok(());
        };
        let (current_col, _) = target.layout.pos_in_scrolling_layout.unwrap_or((1, 1));
        let target_index = (current_col as i32 + position_delta).max(1) as usize;

        self.move_column_to_absolute_index(window_id, target_index, keep_stacked)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_column_to_absolute_index(&self, window_id: u64, target_index: usize, keep_stacked: bool) -> Result<(), ModuleError> {
        let all_windows = query_windows()?;
        let currently_focused = all_windows.iter().find(|w| w.is_focused).map(|w| w.id);

        self.move_column_inner(window_id, target_index, keep_stacked)?;

        if let Some(original_focus) = currently_focused {
            if original_focus != window_id {
                self.focus_window(original_focus)?;
            }
        }
        Ok(())
    }

    #[tracing::instrument(level = "TRACE", err, skip(moves))]
    pub fn reposition_windows_group(&self, moves: &[(u64, usize)], keep_stacked: bool) -> Result<(), ModuleError> {
        if moves.is_empty() {
            return Ok(());
        }

        let all_windows = query_windows()?;
        let currently_focused = all_windows.iter().find(|w| w.is_focused).map(|w| w.id);

        let filtered: Vec<(u64, usize)> = if keep_stacked {
            let mut best_per_col: HashMap<usize, (u64, usize)> = HashMap::new();
            for &(wid, target) in moves {
                let Some(w) = all_windows.iter().find(|w| w.id == wid) else { continue };
                let Some((col, _)) = w.layout.pos_in_scrolling_layout else { continue };
                best_per_col
                    .entry(col)
                    .and_modify(|e| if target < e.1 { *e = (wid, target); })
                    .or_insert((wid, target));
            }
            best_per_col.into_values().collect()
        } else {
            moves.to_vec()
        };

        let sum_current: i32 = filtered
            .iter()
            .filter_map(|(wid, _)| all_windows.iter().find(|w| w.id == *wid))
            .filter_map(|w| w.layout.pos_in_scrolling_layout.map(|(c, _)| c as i32))
            .sum();
        let sum_target: i32 = filtered.iter().map(|(_, t)| *t as i32).sum();

        let mut ordered = filtered;
        if sum_target >= sum_current {
            ordered.sort_by(|a, b| b.1.cmp(&a.1));
        } else {
            ordered.sort_by(|a, b| a.1.cmp(&b.1));
        }

        for (window_id, target_index) in ordered {
            if let Err(e) = self.move_column_inner(window_id, target_index, keep_stacked) {
                tracing::error!("group move failed for window {}: {}", window_id, e);
            }
        }

        if let Some(original_focus) = currently_focused {
            let _ = self.focus_window(original_focus);
        }
        Ok(())
    }

    fn move_column_inner(&self, window_id: u64, target_index: usize, keep_stacked: bool) -> Result<(), ModuleError> {
        let all_windows = query_windows()?;
        let Some(target) = all_windows.iter().find(|w| w.id == window_id) else {
            tracing::warn!("window {} not found in window list", window_id);
            return Ok(());
        };
        let (_, tile_position) = target.layout.pos_in_scrolling_layout.unwrap_or((1, 1));
        let is_stacked = tile_position > 1;

        self.focus_window(window_id)?;

        if is_stacked && !keep_stacked {
            send_action(Action::ExpelWindowFromColumn {})?;
        }

        let target_index = target_index.max(1);
        send_action(Action::MoveColumnToIndex { index: target_index })?;
        Ok(())
    }
}

fn query_windows() -> Result<Vec<niri_ipc::Window>, ModuleError> {
    match send_request(Request::Windows)? {
        Ok(niri_ipc::Response::Windows(windows)) => Ok(windows),
        Ok(other) => Err(ModuleError::unexpected_response("Windows", other)),
        Err(msg) => Err(ModuleError::CompositorReply(msg)),
    }
}

fn send_action(action: Action) -> Result<(), ModuleError> {
    validate_handled(send_request(Request::Action(action))?)
}

fn send_request(request: Request) -> Result<Reply, ModuleError> {
    Socket::connect()
        .map_err(ModuleError::CompositorIpc)?
        .send(request)
        .map_err(ModuleError::CompositorIpc)
}

fn validate_handled(response: Reply) -> Result<(), ModuleError> {
    match response {
        Ok(niri_ipc::Response::Handled) => Ok(()),
        Ok(other) => Err(ModuleError::unexpected_response("Handled", other)),
        Err(msg) => Err(ModuleError::CompositorReply(msg)),
    }
}

fn connect_socket() -> Result<Socket, ModuleError> {
    Socket::connect().map_err(ModuleError::CompositorIpc)
}

pub fn start_window_stream(tx: glib::Sender<WindowSnapshot>, filter_workspace: bool) {
    std::thread::spawn(move || {
        if let Err(e) = run_window_stream(&tx, filter_workspace) {
            tracing::error!(%e, "window event stream terminated");
        }
    });
}

pub fn start_workspace_stream(tx: glib::Sender<Vec<Workspace>>) {
    std::thread::spawn(move || {
        if let Err(e) = run_workspace_stream(&tx) {
            tracing::error!(%e, "workspace event stream terminated");
        }
    });
}

fn run_workspace_stream(tx: &glib::Sender<Vec<Workspace>>) -> Result<(), ModuleError> {
    const MAX_BACKOFF_SECS: u64 = 30;
    let mut backoff_secs = 1u64;

    loop {
        match try_run_workspace_stream(tx) {
            Ok(()) | Err(ModuleError::SnapshotChannelClosed) => {
                tracing::info!("workspace event stream ended");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(%e, backoff_secs, "workspace event stream error, reconnecting");
                std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

fn try_run_workspace_stream(tx: &glib::Sender<Vec<Workspace>>) -> Result<(), ModuleError> {
    let mut socket = connect_socket()?;
    let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
    validate_handled(response)?;

    tracing::info!("workspace event stream connected");
    let mut event_reader = socket.read_events();

    loop {
        match event_reader() {
            Ok(Event::WorkspacesChanged { workspaces }) => {
                tx.send(workspaces).map_err(|_| ModuleError::SnapshotChannelClosed)?;
            }
            Ok(_) => {}
            Err(e) => {
                return Err(ModuleError::CompositorIpc(e));
            }
        }
    }
}

fn run_window_stream(tx: &glib::Sender<WindowSnapshot>, filter_workspace: bool) -> Result<(), ModuleError> {
    const MAX_BACKOFF_SECS: u64 = 30;
    let mut backoff_secs = 1u64;
    let mut window_state = WindowTracker::new();

    loop {
        match try_run_window_stream(tx, &mut window_state, filter_workspace) {
            Ok(()) | Err(ModuleError::SnapshotChannelClosed) => {
                tracing::info!("window event stream ended");
                return Ok(());
            }
            Err(e) => {
                tracing::warn!(%e, backoff_secs, "window event stream error, reconnecting");
                std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

fn try_run_window_stream(
    tx: &glib::Sender<WindowSnapshot>,
    window_state: &mut WindowTracker,
    filter_workspace: bool,
) -> Result<(), ModuleError> {
    let mut socket = connect_socket()?;
    let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
    validate_handled(response)?;

    tracing::info!("window event stream connected");
    let mut event_reader = socket.read_events();

    loop {
        match event_reader() {
            Ok(event) => {
                if let Some(snapshot) = window_state.process_event(event, filter_workspace) {
                    tx.send(snapshot).map_err(|_| ModuleError::SnapshotChannelClosed)?;
                }
            }
            Err(e) => {
                return Err(ModuleError::CompositorIpc(e));
            }
        }
    }
}

#[derive(Debug)]
struct WindowTracker {
    state: Option<TrackerState>,
    floating_anchors: HashMap<u64, u64>,
}

#[derive(Debug)]
enum TrackerState {
    WindowsOnly(Vec<niri_ipc::Window>),
    WorkspacesOnly(Vec<Workspace>),
    Ready {
        windows: BTreeMap<u64, niri_ipc::Window>,
        workspaces: BTreeMap<u64, Workspace>,
        active_per_workspace: BTreeMap<u64, u64>,
        last_focused_per_workspace: BTreeMap<u64, u64>,
    },
}

impl WindowTracker {
    fn new() -> Self {
        Self { state: None, floating_anchors: HashMap::new() }
    }

    #[tracing::instrument(level = "TRACE", skip(self))]
    fn process_event(&mut self, event: Event, filter_workspace: bool) -> Option<WindowSnapshot> {
        use TrackerState::*;

        match event {
            Event::WindowsChanged { windows } => {
                self.state = match self.state.take() {
                    Some(WorkspacesOnly(ws)) => Some(Ready {
                        windows: windows.iter().map(|w| (w.id, w.clone())).collect(),
                        workspaces: ws.into_iter().map(|w| (w.id, w)).collect(),
                        active_per_workspace: BTreeMap::new(),
                        last_focused_per_workspace: BTreeMap::new(),
                    }),
                    Some(Ready { workspaces, active_per_workspace, last_focused_per_workspace, .. }) => Some(Ready {
                        windows: windows.iter().map(|w| (w.id, w.clone())).collect(),
                        workspaces,
                        active_per_workspace,
                        last_focused_per_workspace,
                    }),
                    _ => Some(WindowsOnly(windows)),
                };
            }
            Event::WorkspacesChanged { workspaces } => {
                self.state = match self.state.take() {
                    Some(WindowsOnly(wins)) => Some(Ready {
                        windows: wins.iter().map(|w| (w.id, w.clone())).collect(),
                        workspaces: workspaces.into_iter().map(|w| (w.id, w)).collect(),
                        active_per_workspace: BTreeMap::new(),
                        last_focused_per_workspace: BTreeMap::new(),
                    }),
                    Some(Ready { windows, active_per_workspace, last_focused_per_workspace, .. }) => Some(Ready {
                        windows,
                        workspaces: workspaces.into_iter().map(|w| (w.id, w)).collect(),
                        active_per_workspace,
                        last_focused_per_workspace,
                    }),
                    _ => Some(WorkspacesOnly(workspaces)),
                };
            }
            Event::WindowClosed { id } => {
                if let Some(Ready { windows, .. }) = &mut self.state {
                    windows.remove(&id);
                }
                self.floating_anchors.remove(&id);
                self.floating_anchors.retain(|_, anchor_id| *anchor_id != id);
            }
            Event::WindowOpenedOrChanged { window } => {
                if let Some(Ready { windows, last_focused_per_workspace, .. }) = &mut self.state {
                    if window.is_focused {
                        if let Some(old_focused) = windows.values().find(|w| w.is_focused).map(|w| w.id) {
                            if let Some(old_window) = windows.get(&old_focused) {
                                if old_window.layout.pos_in_scrolling_layout.is_some() {
                                    if let Some(ws_id) = old_window.workspace_id {
                                        last_focused_per_workspace.insert(ws_id, old_focused);
                                    }
                                }
                            }
                        }

                        for w in windows.values_mut() {
                            w.is_focused = false;
                        }
                    }
                    windows.insert(window.id, window);
                }
            }
            Event::WindowFocusChanged { id } => {
                if let Some(Ready { windows, last_focused_per_workspace, .. }) = &mut self.state {
                    if let Some(old_focused) = windows.values().find(|w| w.is_focused).map(|w| w.id) {
                        if let Some(window) = windows.get(&old_focused) {
                            if window.layout.pos_in_scrolling_layout.is_some() {
                                if let Some(ws_id) = window.workspace_id {
                                    last_focused_per_workspace.insert(ws_id, old_focused);
                                }
                            }
                        }
                    }

                    for window in windows.values_mut() {
                        window.is_focused = Some(window.id) == id;
                    }

                    if let Some(focused_id) = id {
                        if let Some(window) = windows.get(&focused_id) {
                            if window.layout.pos_in_scrolling_layout.is_some() {
                                if let Some(ws_id) = window.workspace_id {
                                    last_focused_per_workspace.insert(ws_id, focused_id);
                                }
                            }
                        }
                    }
                }
            }
            Event::WorkspaceActivated { id, .. } => {
                if let Some(Ready { workspaces, .. }) = &mut self.state {
                    let activated_output = workspaces.get(&id).and_then(|ws| ws.output.clone());

                    for ws in workspaces.values_mut() {
                        if ws.output == activated_output {
                            ws.is_active = ws.id == id;
                        }
                    }
                }
            }
            Event::WorkspaceActiveWindowChanged { workspace_id, active_window_id } => {
                tracing::debug!("workspace {} active window changed to {:?}", workspace_id, active_window_id);
                if let Some(Ready { active_per_workspace, .. }) = &mut self.state {
                    if let Some(win_id) = active_window_id {
                        active_per_workspace.insert(workspace_id, win_id);
                    } else {
                        active_per_workspace.remove(&workspace_id);
                    }
                    tracing::debug!("active window map: {:?}", active_per_workspace);
                }
            }
            Event::WindowLayoutsChanged { changes } => {
                if let Some(Ready { windows, .. }) = &mut self.state {
                    for (win_id, layout) in changes {
                        if let Some(window) = windows.get_mut(&win_id) {
                            window.layout = layout;
                        } else {
                            tracing::warn!(win_id, ?layout, "layout update for unknown window");
                        }
                    }
                }
            }
            _ => {}
        }

        if let Some(Ready { windows, workspaces, active_per_workspace, last_focused_per_workspace }) = &self.state {
            Some(Self::generate_snapshot(windows, workspaces, active_per_workspace, last_focused_per_workspace, &mut self.floating_anchors, filter_workspace))
        } else {
            None
        }
    }

    fn generate_snapshot(
        windows: &BTreeMap<u64, niri_ipc::Window>,
        workspaces: &BTreeMap<u64, Workspace>,
        active_per_workspace: &BTreeMap<u64, u64>,
        last_focused_per_workspace: &BTreeMap<u64, u64>,
        floating_anchors: &mut HashMap<u64, u64>,
        filter_workspace: bool,
    ) -> WindowSnapshot {
        struct WindowWithWorkspace<'a> {
            window: &'a niri_ipc::Window,
            workspace: &'a Workspace,
        }

        let active_workspace_per_output: HashMap<_, _> = workspaces
            .values()
            .filter(|ws| ws.is_active)
            .filter_map(|ws| ws.output.as_ref().map(|output| (output.clone(), ws.id)))
            .collect();

        let mut window_workspace_pairs: Vec<_> = windows
            .values()
            .filter_map(|window| {
                window.workspace_id.and_then(|ws_id| {
                    workspaces.get(&ws_id).and_then(|ws| {
                        if filter_workspace {
                            let is_active_on_output = ws.output.as_ref()
                                .and_then(|output| active_workspace_per_output.get(output))
                                .map(|active_ws_id| *active_ws_id == ws.id)
                                .unwrap_or(false);
                            
                            if !is_active_on_output {
                                return None;
                            }
                        }
                        Some(WindowWithWorkspace { window, workspace: ws })
                    })
                })
            })
            .collect();

        let mut position_map: HashMap<u64, (usize, usize)> = HashMap::new();

        for ws_id in window_workspace_pairs.iter().map(|p| p.workspace.id).collect::<BTreeSet<_>>() {
            let fallback_anchor_win_id = last_focused_per_workspace.get(&ws_id).copied();
            let fallback_anchor_pos = fallback_anchor_win_id
                .and_then(|win_id| {
                    window_workspace_pairs.iter()
                        .find(|p| p.window.id == win_id)
                        .and_then(|p| p.window.layout.pos_in_scrolling_layout)
                })
                .unwrap_or_else(|| {
                    window_workspace_pairs.iter()
                        .filter(|p| p.workspace.id == ws_id && p.window.layout.pos_in_scrolling_layout.is_some())
                        .filter_map(|p| p.window.layout.pos_in_scrolling_layout)
                        .max_by_key(|pos| (pos.0, pos.1))
                        .unwrap_or((0, 0))
                });

            let floating_ids: Vec<u64> = window_workspace_pairs.iter()
                .filter(|p| p.workspace.id == ws_id && p.window.layout.pos_in_scrolling_layout.is_none())
                .map(|p| p.window.id)
                .collect();

            for float_id in floating_ids {
                let stable_pos = floating_anchors.get(&float_id)
                    .and_then(|anchor_id| {
                        window_workspace_pairs.iter()
                            .find(|p| p.window.id == *anchor_id)
                            .and_then(|p| p.window.layout.pos_in_scrolling_layout)
                    });

                let pos = match stable_pos {
                    Some(p) => p,
                    None => {
                        if let Some(win_id) = fallback_anchor_win_id {
                            floating_anchors.insert(float_id, win_id);
                        }
                        fallback_anchor_pos
                    }
                };

                position_map.insert(float_id, (pos.0, pos.1 + 1));
            }
        }

        window_workspace_pairs.sort_by(|a, b| {
            a.workspace.idx
                .cmp(&b.workspace.idx)
                .then_with(|| {
                    let a_pos = a.window.layout.pos_in_scrolling_layout.or_else(|| position_map.get(&a.window.id).copied()).unwrap_or((usize::MAX, 0));
                    let b_pos = b.window.layout.pos_in_scrolling_layout.or_else(|| position_map.get(&b.window.id).copied()).unwrap_or((usize::MAX, 0));
                    a_pos.0.cmp(&b_pos.0).then_with(|| a_pos.1.cmp(&b_pos.1))
                })
                .then_with(|| a.window.id.cmp(&b.window.id))
        });

        let active_workspace = workspaces.values().find(|ws| ws.is_active).map(|ws| ws.id);
        let overview_active = active_workspace.and_then(|ws_id| active_per_workspace.get(&ws_id).copied());
        let has_focused = window_workspace_pairs.iter().any(|pair| pair.window.is_focused);

        let highlight_window = if !has_focused {
            overview_active.or_else(|| {
                active_workspace.and_then(|ws_id| last_focused_per_workspace.get(&ws_id).copied())
            }).or_else(|| {
                active_workspace.and_then(|active_ws| {
                    window_workspace_pairs.iter()
                        .find(|pair| pair.workspace.id == active_ws)
                        .map(|pair| pair.window.id)
                })
            })
        } else {
            None
        };

        tracing::debug!("snapshot: active_ws={:?}, overview={:?}, last_focused={:?}, highlight={:?}",
            active_workspace, overview_active, last_focused_per_workspace, highlight_window);

        window_workspace_pairs
            .into_iter()
            .map(|pair| {
                let mut window_copy = pair.window.clone();
                if !window_copy.is_focused && Some(window_copy.id) == highlight_window {
                    tracing::debug!("highlighting window {}", window_copy.id);
                    window_copy.is_focused = true;
                }
                WindowInfo {
                    inner: window_copy,
                    output_name: pair.workspace.output.clone(),
                }
            })
            .collect()
    }
}

pub type WindowSnapshot = Vec<WindowInfo>;

#[derive(Debug, Clone)]
pub struct WindowInfo {
    inner: niri_ipc::Window,
    output_name: Option<String>,
}

impl WindowInfo {
    pub fn get_output(&self) -> Option<&str> {
        self.output_name.as_deref()
    }
}

impl Deref for WindowInfo {
    type Target = niri_ipc::Window;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

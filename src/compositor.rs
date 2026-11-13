use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    io,
    ops::Deref,
};
use niri_ipc::{Action, Event, Output, Reply, Request, Workspace, socket::Socket};
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

pub fn start_window_stream(tx: async_channel::Sender<WindowSnapshot>, filter_workspace: bool) {
    std::thread::spawn(move || {
        if let Err(e) = run_window_stream(&tx, filter_workspace) {
            tracing::error!(%e, "window event stream terminated");
        }
    });
}

pub fn start_workspace_stream(tx: async_channel::Sender<Vec<Workspace>>) {
    std::thread::spawn(move || {
        if let Err(e) = run_workspace_stream(&tx) {
            tracing::error!(%e, "workspace event stream terminated");
        }
    });
}

fn run_workspace_stream(tx: &async_channel::Sender<Vec<Workspace>>) -> Result<(), ModuleError> {
    const MAX_BACKOFF_SECS: u64 = 30;
    let mut backoff_secs = 1u64;

    loop {
        let attempt_started = std::time::Instant::now();
        match try_run_workspace_stream(tx) {
            Ok(()) | Err(ModuleError::SnapshotChannelClosed) => {
                tracing::info!("workspace event stream ended");
                return Ok(());
            }
            Err(e) => {
                // A connection that stayed up a while was healthy, so start the backoff over.
                if attempt_started.elapsed().as_secs() >= MAX_BACKOFF_SECS {
                    backoff_secs = 1;
                }
                tracing::warn!(%e, backoff_secs, "workspace event stream error, reconnecting");
                std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

fn try_run_workspace_stream(tx: &async_channel::Sender<Vec<Workspace>>) -> Result<(), ModuleError> {
    let mut socket = connect_socket()?;
    let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
    validate_handled(response)?;

    tracing::info!("workspace event stream connected");
    read_event_stream(socket.read_events(), |event| {
        if let Event::WorkspacesChanged { workspaces } = event {
            tx.send_blocking(workspaces)
                .map_err(|_| ModuleError::SnapshotChannelClosed)?;
        }
        Ok(())
    })
}

fn run_window_stream(tx: &async_channel::Sender<WindowSnapshot>, filter_workspace: bool) -> Result<(), ModuleError> {
    const MAX_BACKOFF_SECS: u64 = 30;
    let mut backoff_secs = 1u64;
    let mut window_state = WindowTracker::new();

    loop {
        let attempt_started = std::time::Instant::now();
        match try_run_window_stream(tx, &mut window_state, filter_workspace) {
            Ok(()) | Err(ModuleError::SnapshotChannelClosed) => {
                tracing::info!("window event stream ended");
                return Ok(());
            }
            Err(e) => {
                // A connection that stayed up a while was healthy, so start the backoff over.
                if attempt_started.elapsed().as_secs() >= MAX_BACKOFF_SECS {
                    backoff_secs = 1;
                }
                tracing::warn!(%e, backoff_secs, "window event stream error, reconnecting");
                std::thread::sleep(std::time::Duration::from_secs(backoff_secs));
                backoff_secs = (backoff_secs * 2).min(MAX_BACKOFF_SECS);
            }
        }
    }
}

fn try_run_window_stream(
    tx: &async_channel::Sender<WindowSnapshot>,
    window_state: &mut WindowTracker,
    filter_workspace: bool,
) -> Result<(), ModuleError> {
    let mut socket = connect_socket()?;
    let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
    validate_handled(response)?;

    tracing::info!("window event stream connected");
    read_event_stream(socket.read_events(), |event| {
        if let Some(snapshot) = window_state.process_event(event, filter_workspace) {
            tx.send_blocking(snapshot)
                .map_err(|_| ModuleError::SnapshotChannelClosed)?;
        }
        Ok(())
    })
}

/// Hands each event from `next_event` to `handle_event` until `handle_event` fails or a
/// read error ends the stream, then returns that error.
///
/// An event that niri-ipc cannot parse is skipped. Every other read error ends the stream,
/// including the `UnexpectedEof` that `read_events` reports after niri has closed the
/// socket, thereby prompting the caller to reconnect.
fn read_event_stream(
    mut next_event: impl FnMut() -> io::Result<Event>,
    mut handle_event: impl FnMut(Event) -> Result<(), ModuleError>,
) -> Result<(), ModuleError> {
    loop {
        match next_event() {
            Ok(event) => handle_event(event)?,
            // niri adds new event variants over time and asks clients to handle unknown ones
            // in a graceful manner. niri-ipc reports such a line as `InvalidData` solely after
            // having read the entire line, ensuring the subsequent call commences with the
            // next event.
            Err(e) if e.kind() == io::ErrorKind::InvalidData => {
                tracing::debug!(%e, "skipping event that niri-ipc cannot parse");
            }
            Err(e) => return Err(ModuleError::CompositorIpc(e)),
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
            Event::WindowUrgencyChanged { id, urgent } => {
                if let Some(Ready { windows, .. }) = &mut self.state {
                    if let Some(window) = windows.get_mut(&id) {
                        window.is_urgent = urgent;
                    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    /// Delivers `script` one result at a time, just as `read_events` delivers what
    /// niri writes. Requesting additional data triggers a panic, so a loop that continues
    /// reading where it should have ceased causes the test to fail rather than resulting in
    /// a hang.
    fn scripted(script: Vec<io::Result<Event>>) -> impl FnMut() -> io::Result<Event> {
        let mut script = VecDeque::from(script);
        move || script.pop_front().expect("read past the end of the script")
    }

    /// Runs the read loop over `script` and returns its outcome alongside each event it
    /// conveyed.
    fn run(script: Vec<io::Result<Event>>) -> (Result<(), ModuleError>, Vec<Event>) {
        let mut handled = Vec::new();
        let result = read_event_stream(scripted(script), |event| {
            handled.push(event);
            Ok(())
        });
        (result, handled)
    }

    /// What `read_events` returns once niri has closed the socket. niri writes no line on
    /// closing, thus `read_line` returns `Ok(0)` and leaves the buffer empty, while
    /// serde_json reports "EOF while parsing a value at line 1 column 0" due to the empty
    /// buffer.
    fn end_of_stream() -> io::Error {
        io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "EOF while parsing a value at line 1 column 0",
        )
    }

    fn is_read_error(result: &Result<(), ModuleError>, kind: io::ErrorKind) -> bool {
        matches!(result, Err(ModuleError::CompositorIpc(e)) if e.kind() == kind)
    }

    // Since niri commit 45739bc, each event stream starts with a burst containing
    // `TouchpadToggled`, an event unknown to niri-ipc 26.4.0; the script repeats that burst
    // as niri-git sends it. Ending the connection upon encountering the unknown event caused
    // the taskbar to synchronize with niri only at each reconnect, with intervals ranging
    // from seconds to half a minute.
    #[test]
    fn unknown_event_in_initial_burst_is_skipped() {
        let (_, handled) = run(vec![
            Ok(Event::WorkspacesChanged { workspaces: vec![] }),
            Ok(Event::WindowsChanged { windows: vec![] }),
            Ok(Event::KeyboardLayoutsChanged {
                keyboard_layouts: niri_ipc::KeyboardLayouts {
                    names: vec![],
                    current_idx: 0,
                },
            }),
            Ok(Event::OverviewOpenedOrClosed { is_open: false }),
            Ok(Event::ConfigLoaded { failed: false }),
            Ok(Event::CastsChanged { casts: vec![] }),
            Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unknown variant `TouchpadToggled`, expected one of `WorkspacesChanged`, ...",
            )),
            Ok(Event::WindowFocusChanged { id: Some(1) }),
            Err(end_of_stream()),
        ]);

        assert!(
            matches!(
                handled.last(),
                Some(Event::WindowFocusChanged { id: Some(1) })
            ),
            "handled: {handled:?}"
        );
    }

    // Only `InvalidData` is known to leave the stream at the start of the next event.
    // Following any other error, the socket's state becomes uncertain, prompting the
    // connection to terminate and the reconnect loop to activate.
    #[test]
    fn other_read_error_ends_connection() {
        let (result, _) = run(vec![Err(io::ErrorKind::ConnectionReset.into())]);

        assert!(
            is_read_error(&result, io::ErrorKind::ConnectionReset),
            "{result:?}"
        );
    }

    // The stream ends when niri drops an event-stream client that is reading too slowly,
    // an action performed by `send_event` in niri's `src/ipc/server.rs` once more than
    // `EVENT_STREAM_BUFFER_SIZE` (64) events have accumulated for that client. niri
    // continues to operate and its socket remains available, so reconnecting works.
    // Following termination, each subsequent call to `read_events` returns
    // `UnexpectedEof`, so a loop that ignored this would persist in reading from the dead
    // socket endlessly rather than reconnecting.
    #[test]
    fn end_of_stream_ends_connection() {
        let (result, _) = run(vec![Err(end_of_stream())]);

        assert!(
            is_read_error(&result, io::ErrorKind::UnexpectedEof),
            "{result:?}"
        );
    }

    // When snapshots cease to be received, the handler fails with
    // `SnapshotChannelClosed`. This must conclude the loop, ensuring the stream thread
    // exits rather than attempting to read events from niri for nobody.
    #[test]
    fn handler_error_ends_connection() {
        let result = read_event_stream(
            scripted(vec![Ok(Event::WorkspacesChanged { workspaces: vec![] })]),
            |_| Err(ModuleError::SnapshotChannelClosed),
        );

        assert!(
            matches!(result, Err(ModuleError::SnapshotChannelClosed)),
            "{result:?}"
        );
    }
}

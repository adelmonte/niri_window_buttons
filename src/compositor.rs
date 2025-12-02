use std::{collections::HashMap, ops::Deref};
use async_channel::{Receiver, Sender};
use futures::Stream;
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
        let response = send_request(Request::Action(Action::FocusWindow { id: window_id }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn close_window(&self, window_id: u64) -> Result<(), ModuleError> {
        let response = send_request(Request::Action(Action::CloseWindow { id: Some(window_id) }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn maximize_window_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::MaximizeColumn {}))?;
        validate_handled(response)
    }

	#[tracing::instrument(level = "TRACE", err)]
	pub fn maximize_window_to_edges(&self, window_id: u64) -> Result<(), ModuleError> {
		self.focus_window(window_id)?;
		let response = send_request(Request::Action(Action::MaximizeWindowToEdges { id: Some(window_id) }))?;
		validate_handled(response)
	}

	#[tracing::instrument(level = "TRACE", err)]
	pub fn center_column(&self, window_id: u64) -> Result<(), ModuleError> {
		self.focus_window(window_id)?;
		let response = send_request(Request::Action(Action::CenterColumn {}))?;
		validate_handled(response)
	}

	#[tracing::instrument(level = "TRACE", err)]
	pub fn fullscreen_window(&self, window_id: u64) -> Result<(), ModuleError> {
		let response = send_request(Request::Action(Action::FullscreenWindow { id: Some(window_id) }))?;
		validate_handled(response)
	}

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_floating(&self, window_id: u64) -> Result<(), ModuleError> {
        let response = send_request(Request::Action(Action::ToggleWindowFloating { id: Some(window_id) }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn center_window(&self, window_id: u64) -> Result<(), ModuleError> {
        let response = send_request(Request::Action(Action::CenterWindow { id: Some(window_id) }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn center_visible_columns(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::CenterVisibleColumns {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn expand_column_to_available_width(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::ExpandColumnToAvailableWidth {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_windowed_fullscreen(&self, window_id: u64) -> Result<(), ModuleError> {
        let response = send_request(Request::Action(Action::ToggleWindowedFullscreen { id: Some(window_id) }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn consume_window_into_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::ConsumeWindowIntoColumn {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn expel_window_from_column(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::ExpelWindowFromColumn {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn reset_window_height(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::ResetWindowHeight { id: None }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn switch_preset_column_width(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::SwitchPresetColumnWidth {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn switch_preset_window_height(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::SwitchPresetWindowHeight { id: None }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_workspace_down(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::MoveWindowToWorkspaceDown { focus: false }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_workspace_up(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::MoveWindowToWorkspaceUp { focus: false }))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_left(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::MoveWindowToMonitorLeft {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn move_window_to_monitor_right(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::MoveWindowToMonitorRight {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn toggle_column_tabbed_display(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::ToggleColumnTabbedDisplay {}))?;
        validate_handled(response)
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn focus_workspace_previous(&self, window_id: u64) -> Result<(), ModuleError> {
        self.focus_window(window_id)?;
        let response = send_request(Request::Action(Action::FocusWorkspacePrevious {}))?;
        validate_handled(response)
    }

    pub fn query_outputs(&self) -> Result<HashMap<String, Output>, ModuleError> {
        let response = send_request(Request::Outputs)?;
        match response {
            Ok(niri_ipc::Response::Outputs(outputs)) => Ok(outputs),
            Ok(other) => Err(ModuleError::unexpected_response("Outputs", other)),
            Err(msg) => Err(ModuleError::CompositorReply(msg)),
        }
    }

    pub fn create_window_stream(&self) -> WindowEventStream {
        WindowEventStream::start(self.settings.only_current_workspace())
    }

    pub fn create_workspace_stream(&self) -> Result<impl Stream<Item = Vec<Workspace>>, ModuleError> {
        let mut socket = connect_socket()?;
        let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
        validate_handled(response)?;

        let mut event_reader = socket.read_events();
        Ok(async_stream::stream! {
            loop {
                match event_reader() {
                    Ok(Event::WorkspacesChanged { workspaces }) => yield workspaces,
                    Ok(_) => {},
                    Err(e) => {
                        tracing::error!(%e, "workspace event stream error");
                        break;
                    }
                }
            }
        })
    }

    #[tracing::instrument(level = "TRACE", err)]
    pub fn reposition_window(&self, window_id: u64, position_delta: i32) -> Result<(), ModuleError> {
        if position_delta == 0 {
            return Ok(());
        }

        tracing::info!("repositioning window {} by {} columns", window_id, position_delta);

        let response = send_request(Request::Windows)?;
        let all_windows: Vec<niri_ipc::Window> = match response {
            Ok(niri_ipc::Response::Windows(windows)) => windows,
            Ok(other) => return Err(ModuleError::unexpected_response("Windows", other)),
            Err(msg) => return Err(ModuleError::CompositorReply(msg)),
        };

        let currently_focused = all_windows.iter().find(|w| w.is_focused).map(|w| w.id);

        let target_window = all_windows.iter().find(|w| w.id == window_id);
        let Some(target) = target_window else {
            tracing::warn!("target window not found in window list");
            return Ok(());
        };

        let tile_position = target.layout.pos_in_scrolling_layout.map(|(_, tile)| tile).unwrap_or(1);
        let is_stacked = tile_position > 1;

        self.focus_window(window_id)?;

        if is_stacked {
            tracing::trace!("expelling stacked window from column");
            let response = send_request(Request::Action(Action::ExpelWindowFromColumn {}))?;
            validate_handled(response)?;
        }

        let (action, count) = if position_delta < 0 {
            (Action::MoveColumnLeft {}, position_delta.abs())
        } else {
            (Action::MoveColumnRight {}, position_delta)
        };

        for _ in 0..count {
            let response = send_request(Request::Action(action.clone()))?;
            validate_handled(response)?;
        }

        if let Some(original_focus) = currently_focused {
            if original_focus != window_id {
                self.focus_window(original_focus)?;
            }
        }

        Ok(())
    }
}

#[tracing::instrument(level = "TRACE", err)]
fn send_request(request: Request) -> Result<Reply, ModuleError> {
    connect_socket()?.send(request).map_err(ModuleError::CompositorIpc)
}

#[tracing::instrument(level = "TRACE", err)]
fn connect_socket() -> Result<Socket, ModuleError> {
    Socket::connect().map_err(ModuleError::CompositorIpc)
}

fn validate_handled(response: Reply) -> Result<(), ModuleError> {
    match response {
        Ok(niri_ipc::Response::Handled) => Ok(()),
        Ok(other) => Err(ModuleError::unexpected_response("Handled", other)),
        Err(msg) => Err(ModuleError::CompositorReply(msg)),
    }
}

pub struct WindowEventStream {
    receiver: Receiver<WindowSnapshot>,
}

impl WindowEventStream {
    fn start(filter_workspace: bool) -> Self {
        let (tx, rx) = async_channel::unbounded();
        std::thread::spawn(move || {
            if let Err(e) = run_window_stream(tx, filter_workspace) {
                tracing::error!(%e, "window event stream terminated");
            }
        });

        Self { receiver: rx }
    }

    pub async fn next_snapshot(&self) -> Option<WindowSnapshot> {
        self.receiver.recv().await.ok()
    }
}

fn run_window_stream(tx: Sender<WindowSnapshot>, filter_workspace: bool) -> Result<(), ModuleError> {
    let mut socket = connect_socket()?;
    let response = socket.send(Request::EventStream).map_err(ModuleError::CompositorIpc)?;
    validate_handled(response)?;

    let mut event_reader = socket.read_events();
    let mut window_state = WindowTracker::new();

    loop {
        match event_reader() {
            Ok(event) => {
                if let Some(snapshot) = window_state.process_event(event, filter_workspace) {
                    tx.send_blocking(snapshot).map_err(|_| ModuleError::SnapshotChannelClosed)?;
                }
            }
            Err(e) => {
                tracing::error!(%e, "event stream read error");
                return Err(ModuleError::CompositorIpc(e));
            }
        }
    }
}

#[derive(Debug)]
struct WindowTracker {
    state: Option<TrackerState>,
}

#[derive(Debug)]
enum TrackerState {
    WindowsOnly(Vec<niri_ipc::Window>),
    WorkspacesOnly(Vec<Workspace>),
    Ready {
        windows: std::collections::BTreeMap<u64, niri_ipc::Window>,
        workspaces: std::collections::BTreeMap<u64, Workspace>,
        active_per_workspace: std::collections::BTreeMap<u64, u64>,
        last_focused_per_workspace: std::collections::BTreeMap<u64, u64>,
    },
}

impl WindowTracker {
    fn new() -> Self {
        Self { state: None }
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
                        active_per_workspace: std::collections::BTreeMap::new(),
                        last_focused_per_workspace: std::collections::BTreeMap::new(),
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
                        active_per_workspace: std::collections::BTreeMap::new(),
                        last_focused_per_workspace: std::collections::BTreeMap::new(),
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
            }
            Event::WindowOpenedOrChanged { window } => {
                if let Some(Ready { windows, .. }) = &mut self.state {
                    if window.is_focused {
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
                            if let Some(ws_id) = window.workspace_id {
                                tracing::info!("remembering window {} on workspace {}", old_focused, ws_id);
                                last_focused_per_workspace.insert(ws_id, old_focused);
                            }
                        }
                    }

                    for window in windows.values_mut() {
                        window.is_focused = Some(window.id) == id;
                    }

                    if let Some(focused_id) = id {
                        if let Some(window) = windows.get(&focused_id) {
                            if let Some(ws_id) = window.workspace_id {
                                tracing::info!("window {} now focused on workspace {}", focused_id, ws_id);
                                last_focused_per_workspace.insert(ws_id, focused_id);
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
                tracing::info!("workspace {} active window changed to {:?}", workspace_id, active_window_id);
                if let Some(Ready { active_per_workspace, .. }) = &mut self.state {
                    if let Some(win_id) = active_window_id {
                        active_per_workspace.insert(workspace_id, win_id);
                    } else {
                        active_per_workspace.remove(&workspace_id);
                    }
                    tracing::info!("active window map: {:?}", active_per_workspace);
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
            Some(self.generate_snapshot(windows, workspaces, active_per_workspace, last_focused_per_workspace, filter_workspace))
        } else {
            None
        }
    }

	fn generate_snapshot(
		&self,
		windows: &std::collections::BTreeMap<u64, niri_ipc::Window>,
		workspaces: &std::collections::BTreeMap<u64, Workspace>,
		active_per_workspace: &std::collections::BTreeMap<u64, u64>,
		last_focused_per_workspace: &std::collections::BTreeMap<u64, u64>,
		filter_workspace: bool,
	) -> WindowSnapshot {
		struct WindowWithWorkspace<'a> {
		    window: &'a niri_ipc::Window,
		    workspace: &'a Workspace,
		}

		let active_workspace_per_output: std::collections::HashMap<_, _> = workspaces
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

		window_workspace_pairs.sort_by(|a, b| {
			a.workspace.idx
				.cmp(&b.workspace.idx)
				.then_with(|| {
				    let a_floating = a.window.layout.pos_in_scrolling_layout.is_none();
				    let b_floating = b.window.layout.pos_in_scrolling_layout.is_none();
				    a_floating.cmp(&b_floating)
				})
				.then_with(|| {
				    let a_pos = a.window.layout.pos_in_scrolling_layout.unwrap_or_default();
				    let b_pos = b.window.layout.pos_in_scrolling_layout.unwrap_or_default();
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

        tracing::info!("snapshot: active_ws={:?}, overview={:?}, last_focused={:?}, highlight={:?}",
            active_workspace, overview_active, last_focused_per_workspace, highlight_window);

        window_workspace_pairs
            .into_iter()
            .map(|pair| {
                let mut window_copy = pair.window.clone();
                if !window_copy.is_focused && Some(window_copy.id) == highlight_window {
                    tracing::info!("highlighting window {}", window_copy.id);
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

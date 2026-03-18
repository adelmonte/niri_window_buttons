use std::collections::HashMap;
use itertools::Itertools;
use regex::Regex;
use serde::{Deserialize, Deserializer};

#[derive(Debug, Clone, Deserialize)]
pub struct OutputDimensions {
    #[serde(default)]
    pub min_button_width: Option<i32>,
    #[serde(default)]
    pub max_button_width: Option<i32>,
    #[serde(default)]
    pub max_taskbar_width: Option<i32>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Settings {
    #[serde(default)]
    apps: HashMap<String, Vec<AppRule>>,
    #[serde(default)]
    notifications: NotificationConfig,
    #[serde(default)]
    show_all_outputs: bool,
    #[serde(default)]
    only_current_workspace: bool,
    #[serde(default)]
    show_window_titles: bool,
    #[serde(default = "default_min_width")]
    min_button_width: i32,
    #[serde(default = "default_max_width")]
    max_button_width: i32,
    #[serde(default = "default_icon_size")]
    icon_size: i32,
    #[serde(default = "default_spacing")]
    icon_spacing: i32,
    #[serde(default = "default_max_taskbar")]
    max_taskbar_width: i32,
    #[serde(default)]
    max_taskbar_width_per_output: HashMap<String, i32>,
    #[serde(default)]
    dimensions_per_output: HashMap<String, OutputDimensions>,
    #[serde(default = "default_scroll_arrow_left")]
    scroll_arrow_left: String,
    #[serde(default = "default_scroll_arrow_right")]
    scroll_arrow_right: String,
    #[serde(default)]
    click_actions: ClickActions,
    #[serde(default)]
    ignore_rules: Vec<IgnoreRule>,
    #[serde(default = "default_context_menu")]
    context_menu: Vec<ContextMenuItem>,
    #[serde(default = "default_modifier")]
    multi_select_modifier: ModifierKey,
    #[serde(default = "default_multi_select_menu")]
    multi_select_menu: Vec<MultiSelectMenuItem>,
    #[serde(default)]
    drag_style: DragStyle,
    #[serde(default = "default_true")]
    drag_hover_focus: bool,
    #[serde(default = "default_drag_hover_delay")]
    drag_hover_focus_delay: u32,
    #[serde(default = "default_true")]
    truncate_titles: bool,
    #[serde(default)]
    allow_title_linebreaks: bool,
    #[serde(default = "default_true")]
    show_tooltip: bool,
    #[serde(default = "default_tooltip_delay")]
    tooltip_delay: u32,
    #[serde(default)]
    button_alignment: ButtonAlignment,
    #[serde(default)]
    left_click_focus_on_press: bool,
    #[serde(default)]
    audio_indicator: AudioIndicatorConfig,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ModifierKey {
    #[default]
    Ctrl,
    Shift,
    Alt,
    Super,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AudioIndicatorConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "default_audio_playing_icon")]
    pub playing_icon: String,
    #[serde(default = "default_audio_muted_icon")]
    pub muted_icon: String,
    #[serde(default = "default_true")]
    pub clickable: bool,
}

impl Default for AudioIndicatorConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            playing_icon: default_audio_playing_icon(),
            muted_icon: default_audio_muted_icon(),
            clickable: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DragStyle {
    #[default]
    Browser,
    Classic,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum ButtonAlignment {
    #[default]
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NotificationConfig {
    #[serde(default = "default_true")]
    enabled: bool,
    #[serde(default)]
    map_app_ids: HashMap<String, String>,
    #[serde(default = "default_true")]
    use_desktop_entry: bool,
    #[serde(default)]
    use_fuzzy_matching: bool,
}

impl Default for NotificationConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            map_app_ids: HashMap::new(),
            use_desktop_entry: true,
            use_fuzzy_matching: false,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct AppRule {
    #[serde(rename = "match", deserialize_with = "parse_regex")]
    pattern: Regex,
    #[serde(default)]
    class: Option<String>,
    #[serde(default)]
    click_actions: Option<ClickActions>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClickActions {
    #[serde(default = "default_left_unfocused")]
    pub left_click_unfocused: ClickAction,
    #[serde(default = "default_left_focused")]
    pub left_click_focused: ClickAction,
    #[serde(default = "default_double_click")]
    pub double_click: ClickAction,
    #[serde(default = "default_right_click")]
    pub right_click_unfocused: ClickAction,
    #[serde(default = "default_right_click")]
    pub right_click_focused: ClickAction,
    #[serde(default = "default_middle_click")]
    pub middle_click_unfocused: ClickAction,
    #[serde(default = "default_middle_click")]
    pub middle_click_focused: ClickAction,
    #[serde(default = "default_none")]
    pub scroll_up: ClickAction,
    #[serde(default = "default_none")]
    pub scroll_down: ClickAction,
}

impl Default for ClickActions {
    fn default() -> Self {
        Self {
            left_click_unfocused: ClickAction::Action(WindowAction::FocusWindow),
            left_click_focused: ClickAction::Action(WindowAction::MaximizeColumn),
            double_click: ClickAction::Action(WindowAction::MaximizeWindowToEdges),
            right_click_unfocused: ClickAction::Action(WindowAction::Menu),
            right_click_focused: ClickAction::Action(WindowAction::Menu),
            middle_click_unfocused: ClickAction::Action(WindowAction::CloseWindow),
            middle_click_focused: ClickAction::Action(WindowAction::CloseWindow),
            scroll_up: ClickAction::Action(WindowAction::None),
            scroll_down: ClickAction::Action(WindowAction::None),
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum WindowAction {
    None,
    FocusWindow,
    CloseWindow,
    MaximizeColumn,
    MaximizeWindowToEdges,
    CenterColumn,
    CenterWindow,
    CenterVisibleColumns,
    ExpandColumnToAvailableWidth,
    FullscreenWindow,
    ToggleWindowedFullscreen,
    ToggleWindowFloating,
    ConsumeWindowIntoColumn,
    ExpelWindowFromColumn,
    ResetWindowHeight,
    SwitchPresetColumnWidth,
    SwitchPresetWindowHeight,
    MoveWindowToWorkspaceDown,
    MoveWindowToWorkspaceUp,
    MoveWindowToMonitorLeft,
    MoveWindowToMonitorRight,
    ToggleColumnTabbedDisplay,
    FocusWorkspacePrevious,
    MoveColumnLeft,
    MoveColumnRight,
    MoveColumnToFirst,
    MoveColumnToLast,
    MoveWindowDown,
    MoveWindowUp,
    MoveWindowDownOrToWorkspaceDown,
    MoveWindowUpOrToWorkspaceUp,
    MoveColumnLeftOrToMonitorLeft,
    MoveColumnRightOrToMonitorRight,
    Menu,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IgnoreRule {
    #[serde(default)]
    pub app_id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default, deserialize_with = "parse_optional_regex")]
    pub title_regex: Option<Regex>,
    #[serde(default)]
    pub title_contains: Option<String>,
    #[serde(default)]
    pub workspace: Option<u64>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ContextMenuItem {
    pub label: String,
    #[serde(default)]
    pub action: Option<WindowAction>,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MultiSelectMenuItem {
    pub label: String,
    #[serde(default)]
    pub action: Option<MultiSelectAction>,
    #[serde(default)]
    pub command: Option<String>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(rename_all = "kebab-case")]
pub enum MultiSelectAction {
    CloseWindows,
    MoveToWorkspaceUp,
    MoveToWorkspaceDown,
    MoveToMonitorLeft,
    MoveToMonitorRight,
    MoveToMonitorUp,
    MoveToMonitorDown,
    MoveColumnLeft,
    MoveColumnRight,
    ToggleFloating,
    FullscreenWindows,
    MaximizeColumns,
    CenterColumns,
    ConsumeIntoColumn,
    ToggleTabbedDisplay,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ClickAction {
    Action(WindowAction),
    Command { command: String },
}

impl ClickAction {
    pub fn is_menu(&self) -> bool {
        matches!(self, ClickAction::Action(WindowAction::Menu))
    }

    pub fn is_none(&self) -> bool {
        matches!(self, ClickAction::Action(WindowAction::None))
    }
}

fn parse_regex<'de, D>(deserializer: D) -> Result<Regex, D::Error>
where
    D: Deserializer<'de>,
{
    let pattern = String::deserialize(deserializer)?;
    Regex::new(&pattern).map_err(serde::de::Error::custom)
}

fn parse_optional_regex<'de, D>(deserializer: D) -> Result<Option<Regex>, D::Error>
where
    D: Deserializer<'de>,
{
    let pattern: Option<String> = Option::deserialize(deserializer)?;
    pattern.map(|p| Regex::new(&p).map_err(serde::de::Error::custom)).transpose()
}

fn default_true() -> bool { true }
fn default_min_width() -> i32 { 150 }
fn default_max_width() -> i32 { 235 }
fn default_icon_size() -> i32 { 24 }
fn default_spacing() -> i32 { 6 }
fn default_max_taskbar() -> i32 { 1200 }
fn default_scroll_arrow_left() -> String { "◀".to_string() }
fn default_scroll_arrow_right() -> String { "▶".to_string() }

fn default_none() -> ClickAction { ClickAction::Action(WindowAction::None) }
fn default_left_unfocused() -> ClickAction { ClickAction::Action(WindowAction::FocusWindow) }
fn default_left_focused() -> ClickAction { ClickAction::Action(WindowAction::MaximizeColumn) }
fn default_double_click() -> ClickAction { ClickAction::Action(WindowAction::MaximizeWindowToEdges) }
fn default_right_click() -> ClickAction { ClickAction::Action(WindowAction::Menu) }
fn default_middle_click() -> ClickAction { ClickAction::Action(WindowAction::CloseWindow) }

fn default_modifier() -> ModifierKey { ModifierKey::Ctrl }
fn default_drag_hover_delay() -> u32 { 500 }
fn default_tooltip_delay() -> u32 { 300 }
fn default_audio_playing_icon() -> String { "󰕾".to_string() }
fn default_audio_muted_icon() -> String { "󰖁".to_string() }

fn default_context_menu() -> Vec<ContextMenuItem> {
    vec![
        ContextMenuItem {
            label: "  Maximize Column".to_string(),
            action: Some(WindowAction::MaximizeColumn),
            command: None,
        },
        ContextMenuItem {
            label: "  Maximize to Edges".to_string(),
            action: Some(WindowAction::MaximizeWindowToEdges),
            command: None,
        },
        ContextMenuItem {
            label: "󰉩  Toggle Floating".to_string(),
            action: Some(WindowAction::ToggleWindowFloating),
            command: None,
        },
        ContextMenuItem {
            label: "  Close Window".to_string(),
            action: Some(WindowAction::CloseWindow),
            command: None,
        },
    ]
}

fn default_multi_select_menu() -> Vec<MultiSelectMenuItem> {
    vec![
        MultiSelectMenuItem {
            label: "  Close All".to_string(),
            action: Some(MultiSelectAction::CloseWindows),
            command: None,
        },
        MultiSelectMenuItem {
            label: "  Move All to Workspace Up".to_string(),
            action: Some(MultiSelectAction::MoveToWorkspaceUp),
            command: None,
        },
        MultiSelectMenuItem {
            label: "  Move All to Workspace Down".to_string(),
            action: Some(MultiSelectAction::MoveToWorkspaceDown),
            command: None,
        },
    ]
}

impl Settings {
    pub fn get_app_classes(&self, app_id: &str) -> Vec<&str> {
        self.apps
            .get(app_id)
            .map(|rules| {
                rules
                    .iter()
                    .filter_map(|r| r.class.as_deref())
                    .collect_vec()
            })
            .unwrap_or_default()
    }

    pub fn match_app_rules<'a>(
        &'a self,
        app_id: &str,
        title: &'a str,
    ) -> Box<dyn Iterator<Item = &'a str> + 'a> {
        match self.apps.get(app_id) {
            Some(rules) => Box::new(
                rules
                    .iter()
                    .filter(move |rule| rule.pattern.is_match(title))
                    .filter_map(|rule| rule.class.as_deref())
            ),
            None => Box::new(std::iter::empty()),
        }
    }

    pub fn get_click_actions(&self, app_id: Option<&str>, title: Option<&str>) -> ClickActions {
        if let (Some(id), Some(t)) = (app_id, title) {
            if let Some(rules) = self.apps.get(id) {
                for rule in rules {
                    if rule.pattern.is_match(t) {
                        if let Some(ref actions) = rule.click_actions {
                            return actions.clone();
                        }
                    }
                }
            }
        }
        self.click_actions.clone()
    }

    pub fn should_ignore(&self, app_id: Option<&str>, title: Option<&str>, workspace_id: Option<u64>) -> bool {
        for rule in &self.ignore_rules {
            let app_match = rule.app_id.as_ref().map_or(true, |id| app_id == Some(id.as_str()));
            let title_match = rule.title.as_ref().map_or(true, |t| title == Some(t.as_str()));
            let title_contains_match = rule.title_contains.as_ref().map_or(true, |contains| {
                title.map_or(false, |t| t.contains(contains))
            });
            let title_regex_match = rule.title_regex.as_ref().map_or(true, |regex| {
                title.map_or(false, |t| regex.is_match(t))
            });
            let workspace_match = rule.workspace.map_or(true, |ws| workspace_id == Some(ws));

            if app_match && title_match && title_contains_match && title_regex_match && workspace_match {
                return true;
            }
        }
        false
    }

    pub fn notifications_enabled(&self) -> bool {
        self.notifications.enabled
    }

    pub fn notifications_app_map(&self, app_id: &str) -> Option<&str> {
        self.notifications.map_app_ids.get(app_id).map(String::as_str)
    }

    pub fn notifications_use_desktop_entry(&self) -> bool {
        self.notifications.use_desktop_entry
    }

    pub fn notifications_use_fuzzy_matching(&self) -> bool {
        self.notifications.use_fuzzy_matching
    }

    pub fn show_all_outputs(&self) -> bool {
        self.show_all_outputs
    }

    pub fn only_current_workspace(&self) -> bool {
        self.only_current_workspace
    }

    pub fn show_window_titles(&self) -> bool {
        self.show_window_titles
    }

    pub fn min_button_width(&self, output: Option<&str>) -> i32 {
        output
            .and_then(|name| self.dimensions_per_output.get(name))
            .and_then(|dims| dims.min_button_width)
            .unwrap_or(self.min_button_width)
    }

    pub fn max_button_width(&self, output: Option<&str>) -> i32 {
        output
            .and_then(|name| self.dimensions_per_output.get(name))
            .and_then(|dims| dims.max_button_width)
            .unwrap_or(self.max_button_width)
    }

    pub fn icon_size(&self) -> i32 {
        self.icon_size
    }

    pub fn icon_spacing(&self) -> i32 {
        self.icon_spacing
    }

    pub fn max_taskbar_width_for_output(&self, output: Option<&str>) -> i32 {
        output
            .and_then(|name| {
                self.dimensions_per_output.get(name)
                    .and_then(|dims| dims.max_taskbar_width)
                    .or_else(|| self.max_taskbar_width_per_output.get(name).copied())
            })
            .unwrap_or(self.max_taskbar_width)
    }

    pub fn scroll_arrow_left(&self) -> &str {
        &self.scroll_arrow_left
    }

    pub fn scroll_arrow_right(&self) -> &str {
        &self.scroll_arrow_right
    }

    pub fn context_menu(&self) -> &[ContextMenuItem] {
        &self.context_menu
    }

    pub fn multi_select_modifier(&self) -> ModifierKey {
        self.multi_select_modifier
    }

    pub fn multi_select_menu(&self) -> &[MultiSelectMenuItem] {
        &self.multi_select_menu
    }

    pub fn drag_style(&self) -> DragStyle {
        self.drag_style
    }

    pub fn drag_hover_focus(&self) -> bool {
        self.drag_hover_focus
    }

    pub fn drag_hover_focus_delay(&self) -> u32 {
        self.drag_hover_focus_delay
    }

    pub fn truncate_titles(&self) -> bool {
        self.truncate_titles
    }

    pub fn allow_title_linebreaks(&self) -> bool {
        self.allow_title_linebreaks
    }

    pub fn show_tooltip(&self) -> bool {
        self.show_tooltip
    }

    pub fn tooltip_delay(&self) -> u32 {
        self.tooltip_delay
    }

    pub fn button_alignment(&self) -> ButtonAlignment {
        self.button_alignment
    }

    pub fn left_click_focus_on_press(&self) -> bool {
        self.left_click_focus_on_press
    }

    pub fn audio_indicator(&self) -> &AudioIndicatorConfig {
        &self.audio_indicator
    }
}
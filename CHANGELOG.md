# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2026-03-17

### Added
- Browser-style drag reorder: grabbed button follows the cursor while neighboring buttons reposition around it
- `drag_style` setting to choose between `"browser"` (new default) and `"classic"` (original GTK drag-and-drop)
- Audio indicator with click-to-mute for windows playing audio
- `audio_indicator` configuration block (`enabled`, `playing_icon`, `muted_icon`, `clickable`)

### Fixed
- Click actions no longer fire after completing a drag reorder
- Drag reorder no longer flickers when hovering near button boundaries
- Audio reconnection after PulseAudio/PipeWire restart

### Changed
- Smoother drag reorder rendering (no layout recalculation during drag)

## [0.3.5] - 2026-02-20

### Added
- Configurable `button_alignment` option (left, center, right)
- `left_click_focus_on_press` option

### Changed
- Middle and right click actions now fire on press instead of release

## [0.3.2] - 2026-01-07

### Fixed
- Scroll overflow now works with smooth scrolling (touchpads, modern mice)
- Mouse wheel on buttons scrolls taskbar when `scroll_up`/`scroll_down` are `"none"`
- Container no longer expands beyond `max_taskbar_width`

### Changed
- Drag-and-drop repositioning uses `MoveColumnToIndex` for efficiency (single IPC call instead of multiple move commands)

## [0.3.1] - 2025-12-29

### Added
- Custom shell commands in click actions (not just menus)
  - Use `{ "command": "script.sh {window_id}" }` format for any click action
  - Backwards compatible: existing string format still works
- 8 new multi-select batch actions:
  - `move-to-monitor-left` / `move-to-monitor-right` / `move-to-monitor-up` / `move-to-monitor-down`
  - `maximize-columns`
  - `center-columns`
  - `consume-into-column`
  - `toggle-tabbed-display`

## [0.3.0] - 2025-12-29

### Added
- Multi-select windows with modifier key (Ctrl by default)
  - Configurable modifier via `multi_select_modifier` (ctrl, shift, alt, super)
  - Customizable multi-select context menu via `multi_select_menu`
  - Batch actions: close-windows, move-to-workspace-up/down, toggle-floating, fullscreen-windows
  - Custom scripts support with `{window_ids}` placeholder
- Separate click actions for focused vs unfocused windows:
  - `right_click_focused` / `right_click_unfocused`
  - `middle_click_focused` / `middle_click_unfocused`
- Scroll wheel actions (`scroll_up` / `scroll_down`)
- Custom shell commands in context menu items via `command` field
- 10 new window movement actions:
  - `move-column-left` / `move-column-right`
  - `move-column-to-first` / `move-column-to-last`
  - `move-window-up` / `move-window-down`
  - `move-window-up-or-to-workspace-up` / `move-window-down-or-to-workspace-down`
  - `move-column-left-or-to-monitor-left` / `move-column-right-or-to-monitor-right`

### Changed
- Modifier key detection no longer requires `input` group membership
- Selection clears automatically when window focus changes
- Click action config fields renamed for clarity (e.g., `right_click` split into `right_click_focused`/`right_click_unfocused`)

### Fixed
- Modifier key detection on Wayland layer-shell surfaces

## [0.2.0] - 2025-12-02

### Added
- Scrollable taskbar with arrow navigation when buttons exceed `max_taskbar_width`
- Configurable scroll arrow glyphs (`scroll_arrow_left` and `scroll_arrow_right`)
- Per-output max taskbar width configuration via `max_taskbar_width_per_output`
- Per-output dimension configuration via `dimensions_per_output` for fine-grained control of button sizes per monitor
- 14 new IPC window management actions:
  - `center-column` - Center the focused column on the screen
  - `center-window` - Center the window on the screen
  - `center-visible-columns` - Center all fully visible columns on the screen
  - `expand-column-to-available-width` - Expand column to fill available width
  - `consume-window-into-column` - Stack window into the adjacent column
  - `expel-window-from-column` - Unstack window from its column
  - `reset-window-height` - Reset window height to default
  - `switch-preset-column-width` - Cycle through preset column widths
  - `switch-preset-window-height` - Cycle through preset window heights
  - `move-window-to-workspace-down` - Move window to workspace below
  - `move-window-to-workspace-up` - Move window to workspace above
  - `move-window-to-monitor-left` - Move window to monitor on the left
  - `move-window-to-monitor-right` - Move window to monitor on the right
  - `toggle-column-tabbed-display` - Toggle tabbed display mode for column

### Changed
- Renamed all click actions to match niri IPC naming conventions for consistency
- Existing actions remain functional but now use standard niri terminology

### Fixed
- Workspace activation is now output-aware for proper multi-monitor support
- Per-output max taskbar width now applies correctly to each monitor
- Arrow visibility updates are deferred to prevent layout corruption
- Multi-monitor setups now properly show active workspace windows simultaneously
- Focus is restored after drag-and-drop to keep viewport at source position

## [0.1.0] - 2024-11-30

Initial release.

### Features
- Window buttons with application icons and optional title text
- Fully configurable click actions (left, right, middle, double-click)
- Configurable context menu
- Per-application click behavior and styling via regex title matching
- Advanced window filtering (by app, title, workspace)
- Drag and drop window reordering
- Dynamic button sizing with taskbar width limits
- Multi-monitor support
- Notification integration with urgency hints
- Custom CSS classes via pattern matching
- Shows active window in Niri overview

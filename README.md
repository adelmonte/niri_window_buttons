# niri_window_buttons

A Waybar module for displaying and managing traditional window buttons in the Niri compositor.

![screenshot](demo.png)

## Features

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

## Installation

```bash
cargo build --release
```

The compiled module will be at `target/release/libniri_window_buttons.so`.

## Configuration

### Basic Example

```jsonc
{
  "modules-left": ["cffi/niri_window_buttons"],
  "cffi/niri_window_buttons": {
    "module_path": "/path/to/libniri_window_buttons.so",
    "only_current_workspace": false,
    "show_window_titles": true,
    "icon_size": 24,
    "icon_spacing": 6,
    "min_button_width": 150,
    "max_button_width": 235,
    "max_taskbar_width": 1200,
    "click_actions": {
      "left_click_unfocused": "focus",
      "left_click_focused": "maximize-column",
      "double_click": "maximize-edges",
      "right_click": "menu",
      "middle_click": "close"
    },
    "context_menu": [
      {"label": "  Maximize Column", "action": "maximize-column"},
      {"label": "  Maximize to Edges", "action": "maximize-edges"},
      {"label": "󰉩  Toggle Floating", "action": "toggle-floating"},
      {"label": "  Close Window", "action": "close"}
    ],
    "ignore_rules": [],
    "notifications": {
      "enabled": true,
      "use_desktop_entry": true,
      "use_fuzzy_matching": false
    },
    "apps": {}
  }
}
```

### Display Options

- `show_all_outputs` - Show windows from all monitors (default: `false`)
- `only_current_workspace` - Show only current workspace windows (default: `false`)
- `show_window_titles` - Display window titles next to icons (default: `true`)

### Size Controls

- `min_button_width` - Minimum button width in pixels (default: `150`)
- `max_button_width` - Maximum button width in pixels (default: `235`)
- `max_taskbar_width` - Total taskbar width limit in pixels (default: `1200`)
- `icon_size` - Icon dimensions in pixels (default: `24`)
- `icon_spacing` - Space between icon and title in pixels (default: `6`)

### Click Actions

Configure what happens when you click buttons. All click types can be assigned any action, including the context menu:

```jsonc
"click_actions": {
  "left_click_unfocused": "focus",
  "left_click_focused": "maximize-column",
  "double_click": "maximize-edges",
  "right_click": "menu",
  "middle_click": "close"
}
```

**Available actions:**
- `"none"` - Do nothing
- `"focus"` - Focus/activate the window
- `"close"` - Close the window
- `"maximize-column"` - Maximize column width (respects gaps/borders)
- `"maximize-edges"` - Maximize window to screen edges (no gaps)
- `"fullscreen"` - Toggle fullscreen mode
- `"toggle-floating"` - Toggle between floating and tiled
- `"menu"` - Show context menu

### Context Menu

Customize which actions appear in the context menu and their order:

```jsonc
"context_menu": [
  {"label": "  Fullscreen", "action": "fullscreen"},
  {"label": "  Maximize Column", "action": "maximize-column"},
  {"label": "  Maximize to Edges", "action": "maximize-edges"},
  {"label": "󰉩  Toggle Floating", "action": "toggle-floating"},
  {"label": "  Close Window", "action": "close"}
]
```

The menu can be triggered via any click action by setting it to `"menu"`.

### Per-App Configuration

Override click actions and add CSS classes based on app ID and window title patterns:

```jsonc
"apps": {
  "firefox": [
    {
      "match": ".*Picture-in-Picture.*",
      "class": "pip",
      "click_actions": {
        "left_click_focused": "toggle-floating",
        "middle_click": "close"
      }
    },
    {
      "match": ".*",
      "click_actions": {
        "left_click_focused": "maximize-edges"
      }
    }
  ],
  "signal": [
    {
      "match": "\\([0-9]+\\)$",
      "class": "unread"
    }
  ]
}
```

**Per-app rule fields:**
- `"match"` - Regex pattern to match against window title (required)
- `"class"` - CSS class to apply when matched (optional)
- `"click_actions"` - Override click behavior for matching windows (optional)

Rules are evaluated in order. The first matching rule's settings are applied.

### Ignore Rules

Hide specific windows from the taskbar using flexible matching rules:

```jsonc
"ignore_rules": [
  {"app_id": "xpad"},
  {"app_id": "firefox", "title_contains": "Picture-in-Picture"},
  {"app_id": "steam", "title_regex": "^Friends List$"},
  {"workspace": 9},
  {"title": "Firefox — Sharing Indicator"}
]
```

**Available matchers:**
- `"app_id"` - Exact app ID match
- `"title"` - Exact window title match
- `"title_contains"` - Partial title match (substring)
- `"title_regex"` - Regex pattern against title
- `"workspace"` - Hide all windows on specific workspace number

All matchers in a single rule must match for the window to be ignored. Use multiple rules for OR logic.

### Notifications

Enable urgency hints when applications request attention:

```jsonc
"notifications": {
  "enabled": true,
  "use_desktop_entry": true,
  "use_fuzzy_matching": false,
  "map_app_ids": {
    "org.telegram.desktop": "telegram"
  }
}
```

- `enabled` - Enable notification monitoring (default: `true`)
- `use_desktop_entry` - Match via desktop entry if PID lookup fails (default: `true`)
- `use_fuzzy_matching` - Case-insensitive/partial app ID matching (default: `false`)
- `map_app_ids` - Translate notification app IDs to window app IDs (default: `{}`)

## Styling

Customize appearance using Waybar's GTK CSS. The module container uses class `.niri_window_buttons` and contains `button` elements.

**Available CSS Classes:**
- `.focused` - Currently focused window
- `.urgent` - Window with pending notification
- `.dragging` - Window being dragged
- `.drag-over` - Valid drop target during drag
- Custom classes from `apps` configuration

**Example:**

```css
#cffi\.niri_window_buttons button {
  padding: 4px 8px;
  border-radius: 4px;
  transition: background 200ms;
}

#cffi\.niri_window_buttons button.focused {
  background: rgba(255, 255, 255, 0.3);
  border-bottom: 3px solid #81a1c1;
}

#cffi\.niri_window_buttons button.urgent {
  background: rgba(191, 97, 106, 0.4);
}

#cffi\.niri_window_buttons button.unread {
  color: #ebcb8b;
}
```

## Finding App IDs and Titles

Use niri's IPC to inspect windows:

```bash
niri msg windows
```

Or with JSON output for scripting:
```bash
niri msg --json windows | jq '.[] | {app_id, title, workspace_id}'
```

## Limitations

- **Drag-and-drop reordering** works by sending multiple move-left/move-right commands to niri, as the IPC doesn't expose absolute window positions
- **Maximized-to-edges state** cannot be visually indicated because niri IPC doesn't expose this information

- **Smart viewport positioning** after drag-and-drop - currently niri auto-scrolls to the moved window, but there's no IPC command to center the view to show both the original and moved window together

## Wishlist / Future Ideas

- Per-workspace app rules (different click actions per workspace)
- Scroll wheel actions (scroll-up/scroll-down)
- Move window actions (move-left, move-right, move-to-workspace)
- Column actions (consume-into-column, expel-from-column)
- Toggle window title visibility per button
- Minimize/scratchpad support
- Window grouping by app
- Multi-select with modifier keys
- Right/middle_click_unfocused etc.
- Stacked tabs support

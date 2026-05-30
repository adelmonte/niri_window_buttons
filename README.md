# niri_window_buttons [![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

A Waybar module for displaying and managing traditional window buttons in the Niri compositor.

![screenshot](demo.png)

## Features

- Window buttons with application icons and optional title text
- Fully configurable click actions (left, right, middle, double-click, scroll wheel)
- Separate actions for focused vs unfocused windows
- Context menu with custom scripts support
- Multi-select windows with modifier keys
- Per-application click behavior and styling via regex title matching
- Advanced window filtering (by app, title, workspace)
- Drag and drop window reordering, including multi-select group drag, with hover-to-focus for external drags
- Dynamic button sizing with taskbar width limits and scroll overflow
- Multi-monitor support
- Audio indicator with click-to-mute for windows playing audio
- Notification integration with urgency hints
- Custom CSS classes via pattern matching
- Shows active window in Niri overview

## Installation

### From AUR (Arch Linux)

```bash
yay -S niri_window_buttons      # stable release
yay -S niri_window_buttons-git  # latest git version
```

The compiled module will be at `/usr/lib/waybar/libniri_window_buttons.so`.

### Manual Installation

```bash
cargo build --release
```

The compiled module will be at `target/release/libniri_window_buttons.so`.

## Configuration

### Example

```jsonc
{
  "modules-left": ["cffi/niri_window_buttons"],
  "cffi/niri_window_buttons": {
    "module_path": "/path/to/libniri_window_buttons.so",

    // Display
    "only_current_workspace": true,
    "show_window_titles": true,
    "truncate_titles": true,
    "allow_title_linebreaks": false,
    "button_alignment": "left",
    "show_tooltip": true,
    "tooltip_delay": 200,

    // Sizing
    "icon_size": 18,
    "icon_spacing": 6,
    "min_button_width": 10,
    "max_button_width": 190,
    "max_taskbar_width": 1210,
    "scroll_arrow_left": "",
    "scroll_arrow_right": "",

    // Per-output overrides
    "max_taskbar_width_per_output": {
      "DP-1": 1400,
      "eDP-1": 800
    },
    "dimensions_per_output": {
      "eDP-1": {
        "min_button_width": 10,
        "max_button_width": 150,
        "max_taskbar_width": 600
      }
    },

    // Drag reorder
    "drag_style": "browser",
    "left_click_focus_on_press": true,
    "drag_hover_focus": true,
    "drag_hover_focus_delay": 500,

    // Click actions
    "click_actions": {
      "left_click_unfocused": "focus-window",
      "left_click_focused": "maximize-column",
      "double_click": "maximize-window-to-edges",
      "right_click_unfocused": "menu",
      "right_click_focused": "menu",
      "middle_click_unfocused": "close-window",
      "middle_click_focused": "close-window",
      "scroll_up": "move-column-left",
      "scroll_down": "move-column-right"
    },

    // Context menu
    "context_menu": [
      {"label": "  Maximize Column", "action": "maximize-column"},
      {"label": "  Maximize to Edges", "action": "maximize-window-to-edges"},
      {"label": "  Center Column", "action": "center-window"},
      {"label": "󰉩  Toggle Floating", "action": "toggle-window-floating"},
      {"label": "  Move WS Up", "action": "move-window-to-workspace-up"},
      {"label": "  Move WS Down", "action": "move-window-to-workspace-down"},
      {"label": "  Run Script", "command": "notify-send '{app_id}'"},
      {"label": "  Close Window", "action": "close-window"}
    ],

    // Multi-select
    "multi_select_modifier": "shift",
    "multi_select_menu": [
      {"label": "  Move All Up", "action": "move-to-workspace-up"},
      {"label": "  Move All Down", "action": "move-to-workspace-down"},
      {"label": "  Maximize All", "action": "maximize-columns"},
      {"label": "  Close All", "action": "close-windows"}
    ],

    // Filtering
    "ignore_rules": [
      {"app_id": "xpad"},
      {"app_id": "launcher"},
      {"title_contains": "Picture-in-Picture"},
      {"app_id": "firefox", "title_regex": "^Extension:"}
    ],

    // Notifications
    "notifications": {
      "enabled": true,
      "use_desktop_entry": true,
      "use_fuzzy_matching": true,
      "map_app_ids": {
        "org.mozilla.firefox": "firefox"
      }
    },

    // Audio indicator
    "audio_indicator": {
      "enabled": true,
      "playing_icon": "󰕾",
      "muted_icon": "󰖁",
      "clickable": true
    },

    // Per-app CSS classes and click overrides
    "apps": {
      "signal": [
        {"match": "\\([0-9]+\\)$", "class": "unread"}
      ],
      "firefox": [
        {"match": ".*YouTube.*", "class": "youtube"},
        {"match": ".*", "click_actions": {
          "middle_click_unfocused": "none",
          "middle_click_focused": "none"
        }}
      ]
    }
  }
}
```

### Display Options

| Option | Description | Default |
|--------|-------------|---------|
| `show_all_outputs` | Show windows from all monitors | `false` |
| `only_current_workspace` | Show only current workspace windows | `false` |
| `show_window_titles` | Display window titles next to icons | `true` |
| `truncate_titles` | Truncate long titles with ellipsis | `true` |
| `allow_title_linebreaks` | Allow line breaks in window titles (expands button height) | `false` |
| `drag_style` | Drag reorder style: `"browser"` (grabbed button follows cursor) or `"classic"` (GTK drag-and-drop) | `"browser"` |
| `drag_hover_focus` | Focus window when external drag hovers over button | `true` |
| `drag_hover_focus_delay` | Delay in milliseconds before hover triggers focus | `500` |
| `show_tooltip` | Show window title tooltip on hover | `true` |
| `tooltip_delay` | Delay in milliseconds before tooltip appears | `300` |
| `button_alignment` | Align buttons within the taskbar: `"left"`, `"center"`, or `"right"` | `"left"` |
| `left_click_focus_on_press` | Focus unfocused windows on mouse-down instead of mouse-up. When `false`, dragging an unfocused window reorders it without stealing focus | `false` |

### Size Controls

| Option | Description | Default |
|--------|-------------|---------|
| `min_button_width` | Minimum button width in pixels | `150` |
| `max_button_width` | Maximum button width in pixels | `235` |
| `max_taskbar_width` | Total taskbar width limit in pixels | `1200` |
| `icon_size` | Icon dimensions in pixels | `24` |
| `icon_spacing` | Space between icon and title in pixels | `6` |

#### Per-Output Dimension Configuration

For more granular control, configure all button dimensions per output:
```jsonc
{
  "min_button_width": 150,
  "max_button_width": 235,
  "max_taskbar_width": 1200,
  "dimensions_per_output": {
    "eDP-1": {
      "min_button_width": 100,
      "max_button_width": 150,
      "max_taskbar_width": 800
    },
    "DP-1": {
      "min_button_width": 200,
      "max_button_width": 300,
      "max_taskbar_width": 1600
    }
  }
}
```

The top-level dimension settings are used as defaults. For each output, you can override any combination of `min_button_width`, `max_button_width`, and `max_taskbar_width`. Output names can be found using `niri msg outputs`.

#### Scroll Overflow Behavior

When window buttons exceed `max_taskbar_width`, the taskbar becomes scrollable. Arrow buttons appear at the edges for navigation.

**Scrolling methods:**

| Method | Behavior |
|--------|----------|
| Click arrow buttons | Scrolls taskbar by one page |
| Mouse wheel on buttons | Scrolls taskbar (when `scroll_up`/`scroll_down` are `"none"`) |

When `scroll_up`/`scroll_down` are set to a window action (e.g., `"move-column-left"`), that action executes instead of scrolling the taskbar. Set them to `"none"` to enable mouse wheel taskbar scrolling.

**Arrow customization:**

```jsonc
{
  "scroll_arrow_left": "←",
  "scroll_arrow_right": "→"
}
```

Defaults are `"◀"` and `"▶"`. You can use any unicode characters, emoji, or Nerd Font icons. The arrows can also be styled via CSS using the `.scroll-arrow-left` and `.scroll-arrow-right` classes.

### Click Actions

Configure what happens when you click buttons. All click types can be assigned any action, including the context menu. Right-click and middle-click support separate actions for focused vs unfocused windows:

```jsonc
"click_actions": {
  "left_click_unfocused": "focus-window",
  "left_click_focused": "maximize-column",
  "double_click": "maximize-window-to-edges",
  "right_click_unfocused": "menu",
  "right_click_focused": "menu",
  "middle_click_unfocused": "focus-window",
  "middle_click_focused": "close-window",
  "scroll_up": "move-column-left",
  "scroll_down": "move-column-right"
}
```

Click actions can also run custom shell commands using object syntax:

```jsonc
"click_actions": {
  "left_click_focused": { "command": "notify-send 'Clicked {app_id}'" },
  "middle_click_focused": { "command": "my-script.sh {window_id}" }
}
```

Placeholders: `{window_id}`, `{app_id}`, `{title}`

**Available actions:**

| Action | Description |
|--------|-------------|
| `none` | Do nothing (for `scroll_up`/`scroll_down`: enables taskbar scrolling) |
| `menu` | Show context menu |
| `focus-window` | Focus the window |
| `close-window` | Close the window |
| **Column/Window Sizing** | |
| `maximize-column` | Maximize column width |
| `maximize-window-to-edges` | Maximize window to screen edges |
| `expand-column-to-available-width` | Expand column to fill available space |
| `reset-window-height` | Reset window to default height |
| `switch-preset-column-width` | Cycle through preset column widths |
| `switch-preset-window-height` | Cycle through preset window heights |
| **Centering** | |
| `center-column` | Center column on screen |
| `center-window` | Center window on screen |
| `center-visible-columns` | Center all visible columns |
| **Display** | |
| `toggle-title` | Toggle title label visibility for this button |
| **Fullscreen/Floating** | |
| `fullscreen-window` | Toggle fullscreen |
| `toggle-windowed-fullscreen` | Toggle windowed fullscreen |
| `toggle-window-floating` | Toggle floating mode |
| **Column Stacking** | |
| `consume-window-into-column` | Stack window into adjacent column |
| `expel-window-from-column` | Unstack window from column |
| `toggle-column-tabbed-display` | Toggle tabbed display mode |
| **Movement** | |
| `move-column-left` | Move column left |
| `move-column-right` | Move column right |
| `move-column-to-first` | Move column to first position |
| `move-column-to-last` | Move column to last position |
| `move-window-up` | Move window up in column |
| `move-window-down` | Move window down in column |
| **Workspace/Monitor Movement** | |
| `move-window-to-workspace-up` | Move window to workspace above |
| `move-window-to-workspace-down` | Move window to workspace below |
| `move-window-up-or-to-workspace-up` | Move up, or to workspace above if at top |
| `move-window-down-or-to-workspace-down` | Move down, or to workspace below if at bottom |
| `move-window-to-monitor-left` | Move window to left monitor |
| `move-window-to-monitor-right` | Move window to right monitor |
| `move-column-left-or-to-monitor-left` | Move column left, or to left monitor |
| `move-column-right-or-to-monitor-right` | Move column right, or to right monitor |
| **Focus** | |
| `focus-workspace-previous` | Focus previously active workspace |

### Context Menu

Customize which actions appear in the context menu and their order:

```jsonc
"context_menu": [
  {"label": "  Fullscreen", "action": "fullscreen-window"},
  {"label": "  Maximize Column", "action": "maximize-column"},
  {"label": "  Maximize to Edges", "action": "maximize-window-to-edges"},
  {"label": "󰉩  Toggle Floating", "action": "toggle-window-floating"},
  {"label": "󱆃  Custom Script", "command": "my-script.sh {window_ids}"},
  {"label": "  Toggle Title", "action": "toggle-title"},
  {"label": "  Close Window", "action": "close-window"}
]
```

Menu items can also run custom shell commands with placeholders:
- `{window_id}` - Window ID
- `{app_id}` - Application ID
- `{title}` - Window title

Example: `{"label": "  Run Script", "command": "notify-send 'Window: {app_id}'"}`

The menu can be triggered via any click action by setting it to `"menu"`.

### Multi-Select

Select multiple windows using a modifier key, then perform batch actions via right-click menu:

```jsonc
{
  "multi_select_modifier": "ctrl",
  "multi_select_menu": [
    {"label": "  Move All Up", "action": "move-to-workspace-up"},
    {"label": "  Move All Down", "action": "move-to-workspace-down"},
    {"label": "󰉩  Float All", "action": "toggle-floating"},
    {"label": "  Fullscreen All", "action": "fullscreen-windows"},
    {"label": "󱆃  Custom Script", "command": "my-script.sh {window_ids}"},
    {"label": "  Close All", "action": "close-windows"}
  ]
}
```

**Modifier options:** `ctrl`, `shift`, `alt`, `super`

**Requirements:** User must be in the `input` group for modifier key detection on Wayland:
```bash
sudo usermod -aG input $USER
# Log out and back in for changes to take effect
```

**Multi-select actions:**

| Action | Description |
|--------|-------------|
| `close-windows` | Close all selected windows |
| `move-to-workspace-up` | Move all to workspace above |
| `move-to-workspace-down` | Move all to workspace below |
| `move-to-monitor-left` | Move all to left monitor |
| `move-to-monitor-right` | Move all to right monitor |
| `move-to-monitor-up` | Move all to upper monitor |
| `move-to-monitor-down` | Move all to lower monitor |
| `move-column-left` | Move column left (keeps stacked/tabbed windows together) |
| `move-column-right` | Move column right (keeps stacked/tabbed windows together) |
| `toggle-floating` | Toggle floating on all |
| `fullscreen-windows` | Fullscreen all selected |
| `maximize-columns` | Maximize all selected columns |
| `center-columns` | Center all selected columns |
| `consume-into-column` | Stack all selected into one column |
| `toggle-tabbed-display` | Toggle tabbed mode for all selected |

**Usage:**
- Hold modifier + left-click to select/deselect windows
- Right-click with selections to show multi-select menu
- Drag any selected window to move the whole group: members collapse into a contiguous block around the grabbed button and land adjacent on drop
- Click any window (or window button without modifier) clears selection

**Drag-and-drop with stacked/tabbed windows:**
- Normal drag: expels window from stack, moves it individually
- Modifier + drag: moves entire column together (keeps windows stacked)

The modifier is re-read at drop time, so you can build a selection with the modifier held, then release it before dropping to break stacks apart (or keep it held to preserve them).

Custom commands receive `{window_ids}` as a comma-separated list of window IDs.

### Per-App Configuration

Override click actions and add CSS classes based on app ID and window title patterns:

```jsonc
"apps": {
  "firefox": [
    {
      "match": ".*Picture-in-Picture.*",
      "class": "pip",
      "click_actions": {
        "left_click_focused": "toggle-window-floating",
        "middle_click_focused": "close-window",
        "middle_click_unfocused": "close-window"
      }
    },
    {
      "match": ".*",
      "click_actions": {
        "left_click_focused": "maximize-window-to-edges"
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

| Matcher | Description |
|---------|-------------|
| `app_id` | Exact app ID match |
| `title` | Exact window title match |
| `title_contains` | Partial title match (substring) |
| `title_regex` | Regex pattern against title |
| `workspace` | Hide all windows on specific workspace number |

All matchers in a single rule must match (AND logic). Use multiple rules for OR logic.

### Toggle Title Rules

Auto-start specific windows with their title hidden (icon-only) using the same matchers as ignore rules:

```jsonc
"toggle_title_rules": [
  {"app_id": "kitty"},
  {"app_id": "firefox", "title_contains": "YouTube"}
]
```

**Available matchers:**

| Matcher | Description |
|---------|-------------|
| `app_id` | Exact app ID match |
| `title` | Exact window title match |
| `title_contains` | Partial title match (substring) |
| `title_regex` | Regex pattern against title |

All matchers in a single rule must match (AND logic). Use multiple rules for OR logic.

The title can be toggled back on at any time using the `toggle-title` action.

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

| Option | Description | Default |
|--------|-------------|---------|
| `enabled` | Enable notification monitoring | `true` |
| `use_desktop_entry` | Match via desktop entry if PID lookup fails | `true` |
| `use_fuzzy_matching` | Case-insensitive/partial app ID matching | `false` |
| `map_app_ids` | Translate notification app IDs to window app IDs | `{}` |

### Audio Indicator

Shows a speaker icon on window buttons when the application is playing audio. Requires `libpulse` (provided by PulseAudio or PipeWire's PulseAudio compatibility layer).

Disabled by default. Enable with:

```jsonc
"audio_indicator": {
  "enabled": true
}
```

Full options:

```jsonc
"audio_indicator": {
  "enabled": true,
  "playing_icon": "󰕾",
  "muted_icon": "󰖁",
  "clickable": true
}
```

| Option | Description | Default |
|--------|-------------|---------|
| `enabled` | Enable the audio indicator | `false` |
| `playing_icon` | Icon shown when audio is playing | `󰕾` |
| `muted_icon` | Icon shown when audio is muted | `󰖁` |
| `clickable` | Click the icon to toggle mute | `true` |

The icons use [Nerd Fonts](https://www.nerdfonts.com/) codepoints. Make sure your Waybar font includes them, or set custom icons using any Unicode characters you prefer.

The indicator appears to the right of the application icon. For multi-process applications (Firefox, Chromium), when multiple windows share a process, the indicator is shown only on the focused window. Clicking the indicator toggles mute for all audio streams belonging to that process.

The indicator can be styled via CSS using the `.audio-indicator` class (see [Styling](#styling)).

## Styling

Customize appearance using Waybar's GTK CSS. The module container uses class `.niri_window_buttons` and contains `button` elements.

**Available CSS Classes:**

| Class | Description |
|-------|-------------|
| `.focused` | Currently focused window |
| `.selected` | Multi-selected window |
| `.urgent` | Window with pending notification |
| `.dragging` | Window being dragged (classic drag style) |
| `.drag-over` | Valid drop target during external drag |
| `.audio-indicator` | Audio indicator icon inside each button |
| Custom | Classes from `apps` configuration |

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

#cffi\.niri_window_buttons button.selected {
  background: rgba(136, 192, 208, 0.3);
  border: 1px dashed #88c0d0;
}

#cffi\.niri_window_buttons button.urgent {
  background: rgba(191, 97, 106, 0.4);
}

#cffi\.niri_window_buttons button.unread {
  color: #ebcb8b;
}
```

## Limitations

- **Maximized-to-edges state** cannot be visually indicated because niri IPC doesn't expose this information

## Wishlist / Future Ideas

- Per-workspace app rules (different click actions per workspace)
- Minimize/scratchpad support
- Window grouping by app
- Double stacked bar
- Dynamic sized buttons to reflect niri overview
- Window previews

# niri_window_buttons

A Waybar module for displaying and managing traditional window buttons for the Niri compositor.

![screenshot](demo.png)

## Features

- Window buttons with application icons and optional title text
- Drag and drop window reordering
- App ID filtering for hidden windows
- Click handling: activate, maximize, close windows
- Basic context menu
- Dynamic button sizing
- Workspace filtering (all/current)
- Multi-monitor support
- Notification integration with urgency hints
- Custom CSS styling
- Shows active window in Niri overview

## Configuration

Example configuration:
```jsonc
{
  "modules-left": ["cffi/niri-window-buttons"],
  "cffi/niri-window-buttons": {
    "module_path": "/path/to/libniri_window_buttons.so",
    "show_all_outputs": false,
    "only_current_workspace": false,
    "show_window_titles": true,
    "min_button_width": 150,
    "max_button_width": 235,
    "icon_size": 24,
    "icon_spacing": 6,
    "max_taskbar_width": 1200,
    "middle_click_close": true,
    "click_focused_maximizes": true,
    "ignore_app_ids": ["some-app-id"],
    "notifications": {
      "enabled": true,
      "use_desktop_entry": true,
      "use_fuzzy_matching": false,
      "map_app_ids": {
        "app.id": "different-app-id"
      }
    },
    "apps": {
      "app-id": [
        {
          "match": "\\([0-9]+\\)$",
          "class": "unread"
        }
      ]
    }
  }
}
```

## Building
```bash
cargo build --release
```

The compiled module will be at `/niri_window_buttons/target/release/libniri_window_buttons.so`.

### Available Settings

**Display Options:**
- `show_all_outputs` - Include windows from every monitor (default: `false`)
- `only_current_workspace` - Limit to active workspace windows (default: `false`)
- `show_window_titles` - Show title text alongside icons (default: `true`)

**Size Controls:**
- `min_button_width` - Smallest allowed button width in pixels (default: `150`)
- `max_button_width` - Largest allowed button width in pixels (default: `235`)
- `max_taskbar_width` - Maximum total width for all buttons in pixels (default: `1200`)
- `icon_size` - Icon dimension in pixels (default: `24`)
- `icon_spacing` - Pixels between icon and title (default: `6`)

**Mouse Actions:**
- `middle_click_close` - Close window on middle-click (default: `true`)
- `click_focused_maximizes` - Maximize when clicking focused window (default: `true`)

**Filtering:**
- `ignore_app_ids` - Application IDs to exclude from display (default: `[]`)

**Notification Settings:**
- `enabled` - Activate notification monitoring (default: `true`)
- `use_desktop_entry` - Try desktop entry matching when PID lookup fails (default: `true`)
- `use_fuzzy_matching` - Allow case-insensitive and partial ID matching (default: `false`)
- `map_app_ids` - Translate notification app IDs to window app IDs (default: `{}`)

## Default Functionality
- Left click, Switch to window.
- Left click active window, Maximize Column toggle
- Right click for context menu, including Maximize Column, Toggle Floating, and Close Window
- Middle click, Close
- Drag and Drop to reorder


## Styling

Customize appearance using Waybar's GTK CSS support. The module container uses the class `.niri-window-buttons` and contains `button` elements.

**Available CSS Classes:**
- `.focused` - Active window
- `.urgent` - Notification pending
- `.dragging` - During drag operation
- `.drag-over` - Valid drop target
- Custom classes from `apps` configuration

**Styling Example:**
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
  border-bottom: 3px solid #bf616a;
}

#cffi\.niri_window_buttons button.unread {
  color: #ebcb8b;
}
```

## Wishlist/Ideas

- Expand ignore rules, for example:
```
"ignore": {
    "app_id": ["xpad"],
    "title": [],
    "rules": []
}
"ignore_rules": [
       {"app_id": "firefox", "title_contains": "Picture-in-Picture"},
       {"workspace": 9},  // hide everything on workspace 9
       {"app_id": "steam", "title_regex": "^Friends List$"}
   ]
"ignore_titles": ["Picture-in-Picture", "Firefox — Sharing Indicator"]
```
- Menu option to hide/nirium hidden workspace? (scratchpad)
- Change default cursor
- Scroll buttons to change windows
- Button grouping
- ctrl+click multi select
- Modifier Keys
- Per app overrides
- Toggle or shrink label to icon
- Free reaarange instead of actual Niri workspace order

### Clicking Rules Ideas

```
Standard click types:
left-click (already does focus/activate)
right-click
middle-click
scroll-up
scroll-down

Window management:
focus - activate/focus the window (current default)
close - close the window
minimize (scratchpad) - minimize window
fullscreen - toggle fullscreen
toggle-floating - toggle floating state

Taskbar display:
toggle-title - show/hide the window title for just that button

Niri-specific:
move-to-workspace - move window to specific workspace
move-left / move-right - move window in layout
consume-into-column / expel-from-column - niri column actions

Other:
none - do nothing
menu - context menu
```

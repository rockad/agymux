# agymux Configuration Reference

`agymux` (also invoked as `ax`) is configured through a single TOML configuration file. It manages window ergonomics, tab strip placement, Monokai Pro Octagon styling, mouse click handlers, and bilingual keyboard shortcuts.

---

## Configuration File Locations

`agymux` checks configuration files in the following resolution order:

1. **Explicit flag / environment**: Config passed via CLI flags or `$AGYMUX_CONFIG`.
2. **XDG Standard Path**: `~/.config/agymux/agymux.toml` (or `$XDG_CONFIG_HOME/agymux/agymux.toml`).
3. **Internal Defaults**: If no file is present, `agymux` automatically falls back to built-in Monokai Pro Octagon defaults.

Generated tmux profiles and snippets are synchronized to:
- `~/.config/tmux/agymux.conf` (or custom profile target).

---

## Configuration Schema

```toml
[general]
projects_dir = "~/projects"
default_model = "Gemini 3.6 Flash (Medium)"
default_effort = "medium"
startup_action = "ask"
remote_control = false
single_remote_control_instance = true

[theme]
preset = "default"
rounded_pills = false
status_position = "top"

[theme.tabs]
position = "top"
layout = "horizontal"
sidebar_position = "left"
spacer_lines = 1
alignment = "left"
show_close_button = true
min_width = 8
standard_width = 22
max_width = 30

[keybindings]
prefix = "C-Space"
bilingual_cyrillic = true

[tmux]
isolated_socket = false
```

---

## Exhaustive Settings Reference

### `[general]` — General & Workflow Settings

| Key | Type | Default | Choices / Format | Description |
| :--- | :--- | :--- | :--- | :--- |
| `projects_dir` | `string` | `"~/projects"` | Filesystem path | Root directory recursively scanned by Project Switcher (`Ctrl+Space o` / `O` / `щ` / `Щ`) for git repositories and workspaces. |
| `default_model` | `string` | `"Gemini 3.6 Flash (Medium)"` | Any Antigravity model name | Default model preselected when creating new conversation tabs. |
| `default_effort` | `string` | `"medium"` | `"none"`, `"low"`, `"medium"`, `"high"` | Default thinking / reasoning effort level passed to Antigravity sessions. |
| `startup_action` | `string` | `"ask"` | `"new"`, `"resume_latest"`, `"picker"`, `"terminal"`, `"ask"` | Cockpit launch behavior when invoked without command arguments. |
| `remote_control` | `boolean` | `false` | `true`, `false` | Enable cross-device remote control mode (`--remote-control`). Windows display the `󰢹` indicator. |
| `single_remote_control_instance` | `boolean` | `true` | `true`, `false` | Restrict remote-control mode to a single active instance per machine to prevent conversation forking across competing remote sessions. |

#### Startup Action Choices:
- `"ask"` (default): Interactive interactive prompt offering New Tab, Resume Latest, or Switcher.
- `"new"`: Immediately spawn a new conversation tab.
- `"resume_latest"`: Automatically attach to the most recently active conversation in the project.
- `"picker"`: Instantly pop up the two-pane conversation switcher (`Ctrl+Space s`).
- `"terminal"`: Open a plain shell tab without starting an AI session.

---

### `[theme]` — Status Bar & Styling

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `preset` | `string` | `"default"` | `"default"`, `"octo"` | Color preset palette (Monokai Pro Octagon). |
| `rounded_pills` | `boolean` | `false` | `true`, `false` | Use Powerline rounded glyphs (`` / ``) instead of crisp rectangular flat highlighted tabs. |
| `status_position` | `string` | `"top"` | `"top"`, `"bottom"` | Global tmux status line position. |
| `status_style` | `string` (optional) | `None` | Tmux style string | Custom override for base status bar (`bg=#282a3a,fg=#eaf2f1`). |
| `status_left` | `string` | `None` | Tmux format string | Custom override for left status section (Project pill or Prefix indicator). |
| `status_right` | `string` | `None` | Tmux format string | Custom override for right status section (Running turn indicator, time, branding). |
| `window_status_format` | `string` | `None` | Tmux format string | Custom template for inactive tabs. |
| `window_status_current_format` | `string` | `None` | Tmux format string | Custom template for active tab. |

---

### `[theme.tabs]` — Tab Strip Ergonomics & Rail

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `position` | `string` | `"top"` | `"top"`, `"bottom"` | Position of the tab strip relative to the terminal viewport. |
| `layout` | `string` | `"horizontal"` | `"horizontal"`, `"vertical"` | Tab strip mode. `"horizontal"` renders the tmux status bar. `"vertical"` toggles the interactive IDE sidebar rail. |
| `sidebar_position` | `string` | `"left"` | `"left"`, `"right"` | Position of the interactive IDE sidebar when in vertical layout mode. |
| `spacer_lines` | `integer` | `1` | `0`, `1` | Number of empty breathing rows separating tabs from the active pane (`status 2` vs `status on`). |
| `alignment` | `string` | `"left"` | `"left"`, `"centre"`, `"center"`, `"right"` | Tab bar alignment along the horizontal status row. |
| `show_close_button` | `boolean` | `true` | `true`, `false` | Render the interactive close glyph `󰅖` on active and inactive tabs. |
| `min_width` | `integer` | `8` | Positive integer | Minimum tab width constraint for truncated window names. |
| `standard_width` | `integer` | `22` | Positive integer | Standard optimal tab width for window names. |
| `max_width` | `integer` | `30` | Positive integer | Maximum width limit before window names are truncated. |

---

### `[keybindings]` — Prefix & Keyboard Resilience

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `prefix` | `string` | `"C-Space"` | Valid tmux key | Primary prefix key for all agymux shortcuts. |
| `bilingual_cyrillic` | `boolean` | `true` | `true`, `false` | Automatically bind paired RussianWin Cyrillic keys (e.g. `s` / `ы`, `p` / `з`, `b` / `и`) so layout switching never breaks shortcuts. |

---

### `[tmux]` — Tmux Server Isolation

| Key | Type | Default | Choices | Description |
| :--- | :--- | :--- | :--- | :--- |
| `isolated_socket` | `boolean` | `false` | `true`, `false` | Run agymux sessions under a dedicated socket (`tmux -L agymux`) isolated from default user sessions. |

---

## Status Indicator Legend

`agymux` renders real-time visual indicators directly inside each tab's title:

| Glyph | Unicode | Color Context | Meaning |
| :---: | :---: | :--- | :--- |
| **`󰢹`** | `\u{F08B9}` | Sky Blue (`#78e7ff`) / Slate | **Remote Control Session**: Tab was launched with `--remote-control` active, enabling remote orchestration across devices. |
| **`󰛌`** | `\u{F06CC}` | Cyan Highlight (`#78e7ff`) | **External Sync / Worktree Sync**: Changes were synced or received externally. Automatically clears when the tab receives focus (`pane-focus-in`). |
| **`󰂞`** | `\u{F009E}` | Gold / Red Alert (`#ff657a`) | **Activity / Bell Alert**: Window activity flag or terminal bell triggered by a finished run or waiting background process. |
| **`󰅖`** | `\u{F0156}` | Subtle Grey (`#525666`) / Dark Ink | **Interactive Close Button**: Mouse-clickable zone bound via `#[range=user\|close]`. |

---

## Mouse Ergonomics & Protection

`agymux` features full OpenCode-style mouse handling with safety protections:

### 1. Left Click (`MouseDown1Status`)
- **Clicking the tab body:** Switches to the clicked tab instantly (`switch-client -t =`).
- **Clicking the close icon `󰅖`:**
  - If **2 or more tabs** are open: Immediately closes the tab (`kill-window -t =`).
  - If **only 1 tab** remains: Triggers a tmux confirmation prompt:
    ```text
    Close final tab <title>? (y/n)
    ```
    This prevents accidental window closure from dropping the entire session unexpectedly.

### 2. Middle Click (`MouseDown2Status`)
- Clicking anywhere on a tab with the middle mouse button executes a direct close.
- Protected by the same final-tab confirmation guard as Left Click on `󰅖`.

### 3. Right Click (`MouseDown3Status`)
- Opens an interactive context menu directly at the mouse position:
  - `Close Tab` (`x`)
  - `Rename Tab` (`r`)
  - `Fork Tab` (`f`)
  - `Reload Session` (`R`)

---

## Tab Bar Visual Layouts & ASCII Diagrams

### 1. Top Spaced (`position = "top"`, `spacer_lines = 1`) — Default OpenCode Parity
```text
┌────────────────────────────────────────────────────────────────────────┐
│ #S  󰘳  1 main 󰅖   2 fix-auth 󰢹 󰅖   3 + new 󰅖          ● RUNNING 12:45 ax │ <- Row 0: Tabs
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │ <- Row 1: Spacer
│ $ agy run --model "Gemini 3.6 Flash (Medium)"                          │ <- Row 2+: Terminal
│ Loaded 42 tools. Pair programming session ready.                       │
│                                                                        │
└────────────────────────────────────────────────────────────────────────┘
```

### 2. Top Compact (`position = "top"`, `spacer_lines = 0`)
```text
┌────────────────────────────────────────────────────────────────────────┐
│ #S  󰘳  1 main 󰅖   2 fix-auth 󰢹 󰅖   3 + new 󰅖          ● RUNNING 12:45 ax │ <- Row 0: Tabs
├────────────────────────────────────────────────────────────────────────┤
│ $ agy run --model "Gemini 3.6 Flash (Medium)"                          │ <- Row 1+: Terminal
│ Loaded 42 tools. Pair programming session ready.                       │
└────────────────────────────────────────────────────────────────────────┘
```

### 3. Bottom Spaced (`position = "bottom"`, `spacer_lines = 1`)
```text
┌────────────────────────────────────────────────────────────────────────┐
│ $ agy run --model "Gemini 3.6 Flash (Medium)"                          │ <- Rows 0..N-2
│ Loaded 42 tools. Pair programming session ready.                       │
├────────────────────────────────────────────────────────────────────────┤
│                                                                        │ <- Row N-1: Spacer
├────────────────────────────────────────────────────────────────────────┤
│ #S  󰘳  1 main 󰅖   2 fix-auth 󰢹 󰅖   3 + new 󰅖          ● RUNNING 12:45 ax │ <- Row N: Tabs
└────────────────────────────────────────────────────────────────────────┘
```

### 4. Vertical IDE Rail (`agymux layout` or `Ctrl+Space B` / `И`)
When activated, horizontal status tabs hide (`status off`) and a vertical 42-column sidebar opens on the left (or right):
```text
┌──────────────────────────────────────────┬─────────────────────────────┐
│ 󰘳 agymux Sessions                   [1] │ $ agy run                   │
│                                          │                             │
│ > 1  ● fix-search                        │ Running agent loop...       │
│      git: feature/search-revamp          │ Executed search_web: 4 hits │
│      turns: 12  tokens: 42.1k           │                             │
│                                          │                             │
│   2  󰢹 remote-control-tab                │                             │
│      git: main                           │                             │
│      turns: 3   tokens: 11.2k           │                             │
│                                          │                             │
│   3    plain-terminal                    │                             │
│      git: main                           │                             │
├──────────────────────────────────────────┤                             │
│ [Enter] Select  [x] Close  [q] Quit      │                             │
└──────────────────────────────────────────┴─────────────────────────────┘
```
On narrow viewports (< 106 columns), the rail dynamically switches to a compact 10-column icon bar.

---

## Pre-Configured Profile Recipes

### 1. OpenCode Parity (Default)
Top tabs, 1-line breathing spacer, flat highlighted Octagon theme, and protected mouse close:
```toml
# ~/.config/agymux/agymux.toml

[theme]
preset = "default"
rounded_pills = false
status_position = "top"

[theme.tabs]
position = "top"
layout = "horizontal"
spacer_lines = 1
alignment = "left"
show_close_button = true
standard_width = 22
```

### 2. Minimalist Compact
Zero spacers, bottom positioning, no close buttons for purely keyboard-driven workflows:
```toml
# ~/.config/agymux/agymux.toml

[theme]
status_position = "bottom"

[theme.tabs]
position = "bottom"
spacer_lines = 0
show_close_button = false
alignment = "left"
standard_width = 18
```

### 3. IDE Vertical Sidebar Cockpit
Vertical interactive sidebar rail positioned on the left side:
```toml
# ~/.config/agymux/agymux.toml

[theme.tabs]
layout = "vertical"
sidebar_position = "left"
```
Or toggle on-the-fly inside any session with:
- Shortcut: `Ctrl+Space B` (or `И` in RussianWin layout)
- Command Palette: `Ctrl+Space p` -> `Toggle IDE Sidebar Layout`
- CLI: `agymux layout`

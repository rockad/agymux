# agymux

**agymux** is a generic, project-agnostic terminal multiplexer that equips Google Antigravity (`agy`) with an **OpenCode-style tabbed multi-conversation UI** inside `tmux`.

---

## Features

- **OpenCode Top Tab Bar**: Visual tab strip positioned at the **top** of the screen (`status-position top`) styled in Monokai Pro Octagon (`[1: job-search]  [2: ssh-sessions]*  [+ 3: new]`).
- **Dynamic Auto-Title Renaming**: Automatically monitors Antigravity's SQLite database (`conversation_summaries.db`) and renames tmux tabs to match assigned conversation titles.
- **Floating Modal Switcher**: Press `Ctrl+Space s` (or run `agymux switch`) to open a centered, floating `fzf` modal listing all open tabs and past project conversations.
- **Direct Tab Switching**:
  - `Alt+1` .. `Alt+9`: Instant jump to tab index (no prefix chord needed).
  - `Alt+Left` / `Alt+Right`: Cycle adjacent tabs.
- **Project Agnostic**: Automatically scopes sessions by directory basename (`agy-<project>`), with explicit overrides via `--project <name>`.
- **Clean Lifecycle**: Zero-dependency `make install` and `make uninstall` scripts.

---

## Installation & Removal

### Install
```bash
make install
# or ./install.sh
```
Links executables to `~/.local/bin/` and sources `agymux-tabs.conf` in `~/.config/tmux/tmux.conf`.

### Uninstall
```bash
make uninstall
# or ./uninstall.sh
```
Removes all symlinks, strips configuration directives, and leaves the system in a pristine state.

---

## Keybindings Cheatsheet

| Shortcut | Action |
| :--- | :--- |
| `Alt+1` .. `Alt+9` | Direct jump to tab 1 through 9 |
| `Alt+Left` / `Alt+Right` | Cycle between previous / next conversation tab |
| `Ctrl+Space s` (or `agymux s`) | Open floating conversation picker (fzf popup modal) |
| `Ctrl+Space c` (or `agymux n`) | Open a new conversation tab (`+ new`) |
| `Ctrl+Space x` | Close current conversation tab |
| `Ctrl+\` | Instant detach (keeps background daemon & all tabs running) |

---

## CLI Usage

```bash
# Attach or start session for current directory
agymux

# Scope explicitly to a project
agymux --project doppelganger

# Open floating switcher
agymux switch

# Open new conversation tab
agymux new [optional-tab-name]

# List active agymux sessions
agymux ls

# Systemd daemon management
agymux daemon status
agymux daemon enable
agymux daemon start
agymux daemon stop
```

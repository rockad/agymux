# agymux v0.2

**agymux** is a generic, project-agnostic terminal multiplexer that equips Google Antigravity (`agy`) with an **OpenCode-style tabbed multi-conversation cockpit** inside `tmux`.

It combines OpenCode's visual delight and spatial workflow ergonomics (top tabs, two-pane switcher, command palette, git worktree isolation) with Antigravity's deep autonomous engine and 100% ToS-compliant local execution.

---

## Features

- **Monokai Pro Octagon Top Tabs**: Visual tab strip positioned at the **top** of the screen (`status-position top`) styled in `#282a3a` warm dark grey with rounded Powerline pill caps (`` / ``), unread indicators, and active turn spinners (`●`).
- **Two-Pane Conversation Switcher**: Press `Ctrl+Space s` (or `ы`) to open a centered, floating `fzf` modal with:
  - **Left Pane (45%)**: Active open tabs and historical project conversations. Supports `Tab` or `Ctrl+A` to toggle between **Local Project** and **Global Machine** search scope.
  - **Right Pane (55%)**: Instant live transcript preview ($< 15\text{ ms}$) showing token counts, initial user prompt, latest turn summary, and executed tool breakdowns.
- **Global Project Hopper**: Press `Ctrl+Space P` (or `З`) to search and switch between repositories across `~/projects/` and running `agy-*` sessions.
- **Quick Command Palette**: Press `Ctrl+Space p` (or `з`) to open the action menu (tab rename, fork, new worktree, model flags, cheatsheet).
- **Git Worktree Isolation**: Create on-demand git worktrees under `.worktrees/<branch>` via `Ctrl+Space w` (or `ц`) to isolate parallel agent write sets.
- **Dual-Access Architecture**: Every operation is executable **both** via direct keyboard shortcuts and from the Command Palette.
- **Bilingual Keymap Resilience**: Every letter shortcut features paired English and Cyrillic (RussianWin) bindings so CapsLock layout switching never produces dead keys.
- **Instant Prompt-Based Naming**: Tabs are named immediately from the first prompt argument (e.g. `dga "fix the auth bug"`), upgraded smoothly in the background when Antigravity writes semantic titles to SQLite.
- **Zero Quota Overhead**: Powered by pure UNIX CLI tools (`jq`, `fzf`, `sqlite3`). Spawns zero Python processes and consumes zero Gemini subscription tokens for UI navigation.

---

## Dual-Access Shortcuts Cheatsheet

| Action | Direct Shortcut | Command Palette Name |
| :--- | :--- | :--- |
| **Switch Tab / Session** | `Ctrl+Space s` / `ы` | `🗂️ Switch Tab / Session` |
| **Switch Project** | `Ctrl+Space P` / `З` | `🌐 Switch Project` |
| **Command Palette** | `Ctrl+Space p` / `з` | `⚙️ Open Command Palette` |
| **Keyboard Cheatsheet** | `Ctrl+Space ?` / `h` / `р` | `❓ Keyboard Shortcuts & Help` |
| **New Tab** | `Ctrl+Space c` / `с` | `➕ New Tab` |
| **New Worktree Tab** | `Ctrl+Space w` / `ц` | `📁 New Worktree Tab (Branch)` |
| **Rename Active Tab** | `Ctrl+Space r` / `к` | `📝 Rename Active Tab` |
| **Fork Conversation** | `Ctrl+Space f` / `а` | `🌿 Fork Conversation (-c)` |
| **Launch with Custom Flags** | `Ctrl+Space l` / `д` | `⚙️ Launch with Custom Flags` |
| **Close Tab** | `Ctrl+Space x` / `ч` | `🧹 Close Tab` |
| **Detach Session** | `Ctrl+Space d` / `в` | `🚪 Detach Session` |
| **Direct Tab Jump** | `Alt+1` .. `Alt+9` | N/A |
| **Cycle Adjacent Tabs** | `Alt+Left` / `Alt+Right` | N/A |

---

## Inside the Two-Pane Switcher (`Ctrl+Space s`)

- `Tab` or `Ctrl+A`: Toggle search scope between **Local Project** (current active tabs + repo history) and **Global Machine** (all repositories across `~/projects/`).
- `Enter`: Switch to selected tab or resume historical conversation.
- `Esc`: Cancel and return to terminal.

---

## Installation & Removal

### Install
```bash
make install
# or ./install.sh
```
Installs the single entrypoint binary `agymux` into `~/.local/bin/agymux` and wires `agymux-tabs.conf` into `~/.config/tmux/tmux.conf`.

### Verify & Test
```bash
make test
```

### Uninstall
```bash
make uninstall
# or ./uninstall.sh
```
Cleans `~/.local/bin/agymux`, strips tmux directives, and leaves dotfiles pristine.

---

## Architecture & Codebase Structure

```
agymux/
├── bin/
│   └── agymux                     # Single entrypoint CLI (all subcommands routed here)
├── lib/
│   ├── core/
│   │   ├── project.sh             # Projects discovery across ~/projects/ & active agy-* sessions
│   │   ├── session.sh             # SQLite queries against conversation_summaries.db
│   │   └── transcript.sh          # JSONL parser for rich turn preview & token counts (jq)
│   ├── ui/
│   │   ├── helper.sh              # Interactive shortcuts cheatsheet modal
│   │   ├── palette.sh             # Command palette modal & action handlers
│   │   ├── picker.sh              # Two-pane fzf modal (45% list / 55% preview)
│   │   ├── project-picker.sh      # Global project switcher modal
│   │   └── theme.sh               # Monokai Pro Octagon palette tokens
│   └── daemon/
│       ├── runner.sh              # Subprocess runner & instant prompt-based naming
│       └── watcher.sh             # Background SQLite title watcher
├── config/
│   └── tmux/
│       └── agymux-tabs.conf       # Top status line & bilingual keybindings
├── install.sh
├── uninstall.sh
├── Makefile
└── README.md
```

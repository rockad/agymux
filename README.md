# agymux (ax) — OpenCode-Style Cockpit for Google Antigravity

[![Crates.io](https://img.shields.io/crates/v/agymux.svg)](https://crates.io/crates/agymux)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

**agymux** (also installed as **`ax`**) is a high-performance, single-binary terminal multiplexer cockpit for Google Antigravity (`agy`) inside `tmux`.

It combines OpenCode's visual delight and spatial workflow ergonomics (top tab strip, two-pane switcher, command palette, live transcript previews, git worktree isolation) with Antigravity's deep autonomous engine — built in native Rust for sub-millisecond cold starts and zero runtime dependencies.

---

## Highlights

- **Sub-Millisecond Cold-Start (< 1 ms):** Written in Rust with Ratatui and bundled C-SQLite. Modals launch in ~700 µs inside `tmux display-popup`.
- **Zero External CLI Dependencies:** Bundled SQLite (`rusqlite`), streaming NDJSON parser (`serde_json`), and built-in fuzzy matching (`nucleo-matcher`). Does not require `fzf`, `jq`, or external `sqlite3`.
- **Dual Binaries (`agymux` & `ax`):** Both commands are built and installed identically. Use `agymux` for descriptive clarity and `ax` for ultra-fast 2-keystroke execution.
- **Monokai Pro Octagon Top Tabs:** Visual status strip positioned at the top of the terminal (`status-position top`) styled in `#282a3a` warm dark grey with Powerline rounded pills (`` / ``).
- **Two-Pane Conversation Switcher (`Ctrl+Space s` / `ы`):**
  - **Left Pane (45%):** Active open tabs and historical project conversations with fuzzy search. Supports `Tab` or `Ctrl+A` to toggle between **Local Project** and **Global Machine** search scope.
  - **Right Pane (55%):** Live rich transcript preview showing token counts, user prompt, latest turn excerpt, and executed tools.
- **Global Project Hopper (`Ctrl+Space P` / `З`):** Search and jump between repositories across `~/projects/` and active `agy-*` sessions.
- **Quick Command Palette (`Ctrl+Space p` / `з`):** Interactive action palette (tab rename, fork, new worktree, launch flags, cheatsheet).
- **Git Worktree Isolation (`Ctrl+Space w` / `ц`):** Create isolated git worktrees on demand under `.worktrees/<branch>`.
- **Bilingual Keymap Resilience:** Every shortcut features paired English and Cyrillic (RussianWin) bindings so CapsLock layout switching never produces dead keys.
- **Instant Prompt-Based Naming:** Windows are named immediately from the first prompt argument, updated dynamically in the background.

---

## Dual-Access Shortcuts Cheatsheet

| Action | Direct Shortcut | Command Palette Name |
| :--- | :--- | :--- |
| **Switch Tab / Session** | `Ctrl+Space s` / `ы` | `🗂️ Switch Tab / Session` |
| **Switch Project** | `Ctrl+Space P` / `З` | `🌐 Switch Project` |
| **Command Palette** | `Ctrl+Space p` / `з` | `⚙️ Open Command Palette` |
| **Keyboard Cheatsheet** | `Ctrl+Space h` / `р` / `?` | `❓ Keyboard Shortcuts & Help` |
| **New Tab** | `Ctrl+Space c` / `с` | `➕ New Tab` |
| **New Worktree Tab** | `Ctrl+Space w` / `ц` | `📁 New Worktree Tab (Branch)` |
| **Rename Active Tab** | `Ctrl+Space r` / `к` | `📝 Rename Active Tab` |
| **Fork Conversation** | `Ctrl+Space f` / `а` | `🌿 Fork Conversation (-c)` |
| **Launch with Custom Flags** | `Ctrl+Space l` / `д` | `⚙️ Launch with Custom Flags` |
| **Close Tab** | `Ctrl+Space x` / `ч` | `🧹 Close Tab` |
| **Detach Session** | `Ctrl+Space d` / `в` | `🚪 Detach Session` |
| **Direct Tab Jump** | `Ctrl+Space 1` .. `9` | N/A |
| **Cycle Adjacent Tabs** | `Ctrl+Space Left` / `Right` | N/A |

---

## Inside the Two-Pane Switcher (`Ctrl+Space s`)

- `Tab` or `Ctrl+A`: Toggle search scope between **Local Project** (current active tabs + repo history) and **Global Machine** (all repositories across `~/projects/`).
- `j` / `k` or `Up` / `Down`: Navigate list.
- `Enter`: Switch to selected tab or resume historical conversation.
- `Esc` / `q`: Cancel and return to terminal.

---

## Installation

### From Source (Cargo)
```bash
git clone https://github.com/rockad/agymux.git
cd agymux
cargo build --release

# Install both agymux and ax binaries
cp target/release/agymux ~/.local/bin/
cp target/release/ax ~/.local/bin/
```

### Via Crates.io
```bash
cargo install agymux
```

---

## CLI Usage

```text
Usage: agymux [OPTIONS] [COMMAND]
   or: ax [OPTIONS] [COMMAND]

Commands:
  attach   Attach to active session or start session in current directory (default)
  switch   Open two-pane conversation & tab picker modal
  project  Open project switcher modal
  palette  Open quick command palette modal
  helper   Display keyboard shortcuts cheatsheet modal
  new      Open a new conversation tab in current session
  run      Internal process runner with instant prompt-based window naming
  preview  Render rich preview for a conversation ID
  ls       List active agymux sessions
  daemon   Manage background systemd user daemon
  help     Print help

Options:
  -p, --project <PROJECT>  Override project session name
  -C, --dir <DIR>          Working directory (default: current directory)
  -d, --direct             Direct execution without tmux
  -V, --version            Print version
```

---

## License

MIT © Aleksandr Korolev

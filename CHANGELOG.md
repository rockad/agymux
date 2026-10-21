## [0.3.0] - 2026-10-21

### 🚀 Features

- *(remote-control)* Add remote-control flag forwarding, config option, and palette session reload action
- *(palette)* Add Kill Session option terminating pane processes and tmux session
- *(tmux)* Add turn-driven tab sync hook, width truncation, mouse close, and ide rail
- *(ui)* Disambiguate keybindings, add Alt tab navigation, and fix title swapping
- *(remote-control)* Enforce single active instance to prevent conversation forking
- *(session)* Auto-attach by directory over SSH and display clean project names
- *(ui)* Remap keybindings to mnemonic defaults and underline accelerators in palette
- *(cli)* Add -v/--version flags and semver release automation

### 🐛 Bug Fixes

- *(session)* Filter subagent and zero-step abandoned sessions from picker and discovery
- *(watcher)* Prevent duplicate tab naming by excluding all bound conversation IDs and expand title limit
- *(picker)* Ensure outside-tmux selections attach to agymux tmux session cleanly
- *(runner)* Pass --conversation instead of -c <id> when resuming session

### 🎨 Styling

- Apply rustfmt formatting across codebase

### 🧪 Testing

- *(session)* Add integration test suite for subagent and abandoned session filtering
- *(sync)* Add cross surface sync integration tests
## [0.2.1] - 2026-10-13

### ⚙️ Miscellaneous Tasks

- *(release)* Bump version to 0.2.1 and fix rustfmt in tmux profile
## [0.2.0] - 2026-10-13

### 🚀 Features

- Initial release of agymux tabbed terminal multiplexer
- Add --help / help flag and usage documentation
- Agymux v0.2 release with OpenCode ergonomics, two-pane switcher, command palette, and shortcut helper
- Rewrite in native Rust with sub-millisecond cold start and ax dual entrypoint
- *(keys)* Move direct tab navigation from root Alt to prefix chords (Ctrl+Space 1..9, Left/Right)
- *(test)* Implement comprehensive test suite, headless tui tests, benchmarks, and ci
- Release v0.2.0 with configurable startup lifecycle, palette expansions, and terminal color inheritance
- *(tmux)* Use ANSI terminal palette colors for tab row and status indicators
- *(tmux)* Add pipe separator and 3-state tab coloring with distinct elevated inactive shade
- *(tmux)* Place project name on the right with dot-separated time and date
- *(tmux)* Add 1-line bottom margin spacer beneath top tabs

### 🐛 Bug Fixes

- *(tmux)* Eliminate comma parsing artifact in status-left style tags
- *(tabs)* Spawn detached watcher process to sync window titles across exec()
- *(tabs)* Reduce title watcher interval to 500ms and exclude previous conversation
- *(attach)* Launch agymux run in initial tmux window to reach tui on first attach
- *(tmux)* Attach interactively with inherited tty and prefer existing session
- *(tmux)* Remove double braces in status-left format string and allow theme overrides
- *(tmux)* Split reverse and bold style tags to avoid comma in status-left ternary
- *(tmux)* Keep session name persistent and prepend prefix indicator on #{client_prefix}

### ⚡ Performance

- Eliminate input latency by removing forced --remote-control and using exec()
- *(tmux)* Enable synchronized output (mode 2026) and relax status interval
- *(tmux)* Set status-interval to 1s for real-time statusline updates

### 🚜 Refactor

- Consolidate into single agymux entrypoint binary
- Retire legacy daemon subcommand and cleanup references
- *(config)* Generalize agymux-tabs.conf to agymux.conf with compatibility symlink

### 🎨 Styling

- Format attach and driver code with cargo fmt
- *(tmux)* Use distinct white text on black for inactive tabs and white clock details
- *(tmux)* Keep brightblack background with white text for inactive tabs
- *(tmux)* Adopt 256-color palette pairing (colour73 teal active, colour237 dark slate inactive)

### ⚙️ Miscellaneous Tasks

- Update install.sh to build and link target/release binaries

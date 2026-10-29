use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::process::Command;

use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_PINK, COLOR_YELLOW,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteAction {
    RenameTab,
    NewTab,
    ForkConversation,
    NewWorktreeTab,
    LaunchCustomFlags,
    CloseTab,
    SwitchTab,
    SwitchProject,
    Shortcuts,
    Detach,
    QuitKeepTabs,
    KillSession,
    BindConversation,
    NewTerminalTab,
    SwitchToMain,
    ReloadActiveSession,
    SplitHorizontal,
    SplitVertical,
    TogglePaneZoom,
    BreakPane,
    CopyMode,
    ClosePane,
    NewClaudeSession,
    NewAntigravitySession,
    MoveTabLeft,
    MoveTabRight,
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub action: PaletteAction,
    pub title: &'static str,
    pub shortcut: &'static str,
    pub mnemonic: Option<char>,
}

impl PaletteItem {
    pub fn all() -> Vec<Self> {
        vec![
            PaletteItem {
                action: PaletteAction::SwitchTab,
                title: "Switch Tab / Session",
                shortcut: "Ctrl+Space s / ы",
                mnemonic: Some('s'),
            },
            PaletteItem {
                action: PaletteAction::SwitchProject,
                title: "Open Project",
                shortcut: "Ctrl+Space o / щ",
                mnemonic: Some('o'),
            },
            PaletteItem {
                action: PaletteAction::NewTab,
                title: "New Conversation Tab",
                shortcut: "Ctrl+Space c / с",
                mnemonic: Some('c'),
            },
            PaletteItem {
                action: PaletteAction::RenameTab,
                title: "Rename Tab",
                shortcut: "Ctrl+Space r / к",
                mnemonic: Some('r'),
            },
            PaletteItem {
                action: PaletteAction::CloseTab,
                title: "Close Tab",
                shortcut: "Ctrl+Space x / ч",
                mnemonic: Some('x'),
            },
            PaletteItem {
                action: PaletteAction::SplitHorizontal,
                title: "Split Window Horizontally",
                shortcut: "Ctrl+Space %",
                mnemonic: Some('h'),
            },
            PaletteItem {
                action: PaletteAction::SplitVertical,
                title: "Split Window Vertically",
                shortcut: "Ctrl+Space \"",
                mnemonic: Some('v'),
            },
            PaletteItem {
                action: PaletteAction::TogglePaneZoom,
                title: "Toggle Pane Zoom",
                shortcut: "Ctrl+Space z",
                mnemonic: Some('z'),
            },
            PaletteItem {
                action: PaletteAction::BreakPane,
                title: "Break Pane to New Tab",
                shortcut: "Ctrl+Space !",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::CopyMode,
                title: "Scroll / Copy Mode",
                shortcut: "Ctrl+Space [",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::ClosePane,
                title: "Close Current Pane",
                shortcut: "Ctrl+Space x",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::Detach,
                title: "Detach Session",
                shortcut: "Ctrl+Space d / в",
                mnemonic: Some('d'),
            },
            PaletteItem {
                action: PaletteAction::QuitKeepTabs,
                title: "Quit Cockpit (keep tabs)",
                shortcut: "Ctrl+Space d / в",
                mnemonic: Some('d'),
            },
            PaletteItem {
                action: PaletteAction::KillSession,
                title: "Kill Session",
                shortcut: "Ctrl+Space k / л",
                mnemonic: Some('k'),
            },
            PaletteItem {
                action: PaletteAction::Shortcuts,
                title: "Help & Shortcuts",
                shortcut: "Ctrl+Space h / р",
                mnemonic: Some('h'),
            },
            PaletteItem {
                action: PaletteAction::BindConversation,
                title: "Bind Conversation to Tab",
                shortcut: "Palette action",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::NewTerminalTab,
                title: "New Terminal Tab",
                shortcut: "Ctrl+Space t / е",
                mnemonic: Some('t'),
            },
            PaletteItem {
                action: PaletteAction::SwitchToMain,
                title: "Switch to Main Terminal",
                shortcut: "Ctrl+Space m / ь",
                mnemonic: Some('m'),
            },
            PaletteItem {
                action: PaletteAction::ForkConversation,
                title: "Fork Conversation",
                shortcut: "Ctrl+Space f / а",
                mnemonic: Some('f'),
            },
            PaletteItem {
                action: PaletteAction::NewWorktreeTab,
                title: "New Worktree Tab",
                shortcut: "Ctrl+Space w / ц",
                mnemonic: Some('w'),
            },
            PaletteItem {
                action: PaletteAction::LaunchCustomFlags,
                title: "Launch with Custom Flags",
                shortcut: "Ctrl+Space l / д",
                mnemonic: Some('l'),
            },
            PaletteItem {
                action: PaletteAction::ReloadActiveSession,
                title: "Reload Active Session",
                shortcut: "Palette",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::NewClaudeSession,
                title: "New Claude Code Session",
                shortcut: "Ctrl+Space c",
                mnemonic: Some('c'),
            },
            PaletteItem {
                action: PaletteAction::NewAntigravitySession,
                title: "New Antigravity Session",
                shortcut: "Ctrl+Space n",
                mnemonic: Some('n'),
            },
            PaletteItem {
                action: PaletteAction::MoveTabLeft,
                title: "Move Tab Left",
                shortcut: "Ctrl+Shift+Left",
                mnemonic: None,
            },
            PaletteItem {
                action: PaletteAction::MoveTabRight,
                title: "Move Tab Right",
                shortcut: "Ctrl+Shift+Right",
                mnemonic: None,
            },
        ]
    }
}

pub struct PaletteState {
    pub items: Vec<PaletteItem>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub query: String,
}

impl Default for PaletteState {
    fn default() -> Self {
        Self::new()
    }
}

impl PaletteState {
    pub fn new() -> Self {
        let items = PaletteItem::all();
        let filtered_indices = (0..items.len()).collect();
        Self {
            items,
            filtered_indices,
            selected: 0,
            query: String::new(),
        }
    }

    pub fn filter(&mut self) {
        let q = self.query.to_lowercase().trim().to_string();
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if q.is_empty() {
                    return true;
                }
                let title = item.title.to_lowercase();
                let shortcut = item.shortcut.to_lowercase();
                if title.contains(&q) || shortcut.contains(&q) {
                    return true;
                }
                if let Some(m) = item.mnemonic {
                    if q == m.to_string().to_lowercase() {
                        return true;
                    }
                }
                // Synonyms & keywords
                match item.action {
                    PaletteAction::QuitKeepTabs => {
                        "quit".contains(&q) || "exit".contains(&q) || q == "q" || q == "й"
                    }
                    PaletteAction::KillSession => {
                        "kill".contains(&q)
                            || "terminate".contains(&q)
                            || "stop".contains(&q)
                            || q == "k"
                            || q == "л"
                    }
                    PaletteAction::Detach => "detach".contains(&q) || q == "d" || q == "в",
                    PaletteAction::SwitchTab => {
                        "switch".contains(&q) || "tab".contains(&q) || q == "s" || q == "ы"
                    }
                    PaletteAction::SwitchProject => {
                        "project".contains(&q) || "open".contains(&q) || q == "o" || q == "щ"
                    }
                    PaletteAction::BindConversation => {
                        "bind".contains(&q) || "attach".contains(&q) || "connect".contains(&q)
                    }
                    PaletteAction::NewTerminalTab => {
                        "terminal".contains(&q)
                            || "term".contains(&q)
                            || "shell".contains(&q)
                            || "sh".contains(&q)
                            || q == "t"
                            || q == "е"
                    }
                    PaletteAction::SwitchToMain => {
                        "main".contains(&q)
                            || "terminal".contains(&q)
                            || "term".contains(&q)
                            || q == "m"
                            || q == "ь"
                    }
                    PaletteAction::ReloadActiveSession => {
                        "reload".contains(&q)
                            || "sync".contains(&q)
                            || "refresh".contains(&q)
                            || "resume".contains(&q)
                            || q == "r"
                            || q == "к"
                    }
                    PaletteAction::SplitHorizontal => {
                        "split".contains(&q)
                            || "horizontal".contains(&q)
                            || "pane".contains(&q)
                            || q == "h"
                            || q == "%"
                    }
                    PaletteAction::SplitVertical => {
                        "split".contains(&q)
                            || "vertical".contains(&q)
                            || "pane".contains(&q)
                            || q == "v"
                            || q == "\""
                    }
                    PaletteAction::TogglePaneZoom => {
                        "zoom".contains(&q)
                            || "maximize".contains(&q)
                            || "fullscreen".contains(&q)
                            || "pane".contains(&q)
                            || q == "z"
                            || q == "я"
                    }
                    PaletteAction::BreakPane => {
                        "break".contains(&q) || "pane".contains(&q) || q == "!"
                    }
                    PaletteAction::CopyMode => {
                        "copy".contains(&q)
                            || "scroll".contains(&q)
                            || "history".contains(&q)
                            || q == "["
                            || q == "х"
                    }
                    PaletteAction::ClosePane => {
                        "close".contains(&q)
                            || "pane".contains(&q)
                            || q == "kill-pane"
                            || q == "killpane"
                    }
                    PaletteAction::NewClaudeSession => {
                        "claude".contains(&q) || "new".contains(&q) || q == "c" || q == "с"
                    }
                    PaletteAction::NewAntigravitySession => {
                        "agy".contains(&q)
                            || "antigravity".contains(&q)
                            || "new".contains(&q)
                            || q == "n"
                            || q == "т"
                    }
                    PaletteAction::MoveTabLeft => {
                        "move".contains(&q)
                            || "left".contains(&q)
                            || "swap".contains(&q)
                            || q == "<"
                            || q == "б"
                    }
                    PaletteAction::MoveTabRight => {
                        "move".contains(&q)
                            || "right".contains(&q)
                            || "swap".contains(&q)
                            || q == ">"
                            || q == "ю"
                    }
                    _ => false,
                }
            })
            .map(|(idx, _)| idx)
            .collect();

        if self.selected >= self.filtered_indices.len() {
            self.selected = self.filtered_indices.len().saturating_sub(1);
        }
    }

    pub fn select_next(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.selected = (self.selected + 1) % self.filtered_indices.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.filtered_indices.is_empty() {
            if self.selected == 0 {
                self.selected = self.filtered_indices.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn selected_action(&self) -> Option<PaletteAction> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.items.get(idx))
            .map(|item| item.action)
    }
}

pub enum PaletteResult {
    Continue,
    Execute(PaletteAction),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut PaletteState) -> PaletteResult {
    match key.code {
        KeyCode::Esc => PaletteResult::Cancel,
        KeyCode::Enter => {
            if let Some(act) = state.selected_action() {
                PaletteResult::Execute(act)
            } else {
                PaletteResult::Cancel
            }
        }
        KeyCode::Up | KeyCode::Char('k')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            state.select_prev();
            PaletteResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j')
            if key
                .modifiers
                .contains(crossterm::event::KeyModifiers::CONTROL) =>
        {
            state.select_next();
            PaletteResult::Continue
        }
        KeyCode::Up => {
            state.select_prev();
            PaletteResult::Continue
        }
        KeyCode::Down => {
            state.select_next();
            PaletteResult::Continue
        }
        KeyCode::Backspace => {
            state.query.pop();
            state.filter();
            PaletteResult::Continue
        }
        KeyCode::Char(c) => {
            state.query.push(c);
            state.filter();
            PaletteResult::Continue
        }
        _ => PaletteResult::Continue,
    }
}

fn render_title_with_mnemonic(
    title: &str,
    mnemonic: Option<char>,
    is_selected: bool,
) -> Vec<Span<'static>> {
    let base_fg = if is_selected { COLOR_GREEN } else { COLOR_FG };
    let mut base_style = Style::default().fg(base_fg);
    if is_selected {
        base_style = base_style.add_modifier(Modifier::BOLD);
    }
    let mut mnemonic_style = Style::default()
        .fg(COLOR_YELLOW)
        .add_modifier(Modifier::UNDERLINED);
    if is_selected {
        mnemonic_style = mnemonic_style.add_modifier(Modifier::BOLD);
    }

    let total_len = title.chars().count();
    let padding_len = 30_usize.saturating_sub(total_len);
    let padding = " ".repeat(padding_len);

    if let Some(m) = mnemonic {
        if let Some((byte_idx, _)) = title
            .char_indices()
            .find(|(_, c)| c.eq_ignore_ascii_case(&m))
        {
            let before = &title[..byte_idx];
            let char_len = title[byte_idx..].chars().next().map_or(1, |c| c.len_utf8());
            let ch_str = &title[byte_idx..byte_idx + char_len];
            let after = &title[byte_idx + char_len..];

            return vec![
                Span::styled(before.to_string(), base_style),
                Span::styled(ch_str.to_string(), mnemonic_style),
                Span::styled(after.to_string(), base_style),
                Span::styled(padding, base_style),
            ];
        }
    }
    vec![
        Span::styled(title.to_string(), base_style),
        Span::styled(padding, base_style),
    ]
}

pub fn render(f: &mut Frame, area: Rect, state: &PaletteState) {
    let width = 74.min(area.width.saturating_sub(4));
    let height = 22.min(area.height.saturating_sub(2));

    let horiz_pad = (area.width.saturating_sub(width)) / 2;
    let vert_pad = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width,
        height,
    };

    f.render_widget(Clear, modal_area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search prompt input
            Constraint::Min(4),    // Action list
            Constraint::Length(1), // Footer hint
        ])
        .split(modal_area);

    // 1. Search Box
    let search_block = styled_block("Command Palette", true);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled(
            "⚡ Action > ",
            Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            &state.query,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("█", Style::default().fg(COLOR_PINK)),
    ]))
    .block(search_block)
    .style(base_style());
    f.render_widget(search_p, chunks[0]);

    // 2. Action List
    let items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &item_idx)| {
            let item = &state.items[item_idx];
            let is_selected = display_idx == state.selected;

            let mut spans = vec![Span::styled("  ", Style::default())];
            spans.extend(render_title_with_mnemonic(
                item.title,
                item.mnemonic,
                is_selected,
            ));
            spans.push(Span::styled(
                format!("({})", item.shortcut),
                if is_selected {
                    Style::default().fg(COLOR_YELLOW)
                } else {
                    muted_style()
                },
            ));
            let line = Line::from(spans);

            if is_selected {
                ListItem::new(line).style(highlight_style())
            } else {
                ListItem::new(line).style(base_style())
            }
        })
        .collect();

    let list_block = styled_block("Select Action", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // 3. Footer
    let footer_p = Paragraph::new(Line::from(vec![
        Span::styled(" Enter: execute  ", Style::default().fg(COLOR_GREEN)),
        Span::styled(" Esc: cancel  ", Style::default().fg(COLOR_PINK)),
        Span::styled(" ↑/↓/Ctrl+j/Ctrl+k: navigate ", muted_style()),
    ]))
    .style(base_style());
    f.render_widget(footer_p, chunks[2]);
}

/// Execute a palette action inside or outside tmux
pub fn execute_action(action: PaletteAction, project: Option<&str>) -> anyhow::Result<()> {
    let proj = project.unwrap_or("default");
    match action {
        PaletteAction::RenameTab => {
            // Display tmux rename-window prompt
            let _ = Command::new("tmux")
                .args(["command-prompt", "-I", "#W", "rename-window '%%'"])
                .status();
        }
        PaletteAction::NewTab => {
            let _ = Command::new("tmux")
                .args([
                    "new-window",
                    "-n",
                    "+ new",
                    &format!("agymux run --project '{}'", proj),
                ])
                .status();
        }
        PaletteAction::ForkConversation => {
            let conv_id = std::env::var("CONVERSATION_ID").unwrap_or_default();
            if !conv_id.is_empty() {
                let _ = Command::new("tmux")
                    .args([
                        "new-window",
                        "-n",
                        "[fork]",
                        &format!(
                            "agymux run --project '{}' --conversation '{}'",
                            proj, conv_id
                        ),
                    ])
                    .status();
            } else {
                let _ = Command::new("tmux")
                    .args([
                        "command-prompt",
                        "-p",
                        "Fork conversation ID: ",
                        &format!("new-window -n '[fork]' 'agymux run --project \"{}\" --conversation \"%%\"'", proj),
                    ])
                    .status();
            }
        }
        PaletteAction::NewWorktreeTab => {
            let _ = Command::new("tmux")
                .args([
                    "command-prompt",
                    "-p",
                    "New worktree branch: ",
                    &format!("new-window -n '[%%]' 'git worktree add -b %% .worktrees/%% && cd .worktrees/%% && agymux run --project \"{}\"'", proj),
                ])
                .status();
        }
        PaletteAction::LaunchCustomFlags => {
            let _ = Command::new("tmux")
                .args([
                    "command-prompt",
                    "-p",
                    "Flags (e.g. --model 'Gemini Pro' --effort high): ",
                    &format!(
                        "new-window -n 'agy [custom]' 'agymux run --project \"{}\" %%'",
                        proj
                    ),
                ])
                .status();
        }
        PaletteAction::CloseTab => {
            let _ = Command::new("tmux")
                .args([
                    "confirm-before",
                    "-p",
                    "kill-window #W? (y/n)",
                    "kill-window",
                ])
                .status();
        }
        PaletteAction::SwitchTab => {
            let _ = Command::new("tmux")
                .args([
                    "display-popup",
                    "-E",
                    "-w",
                    "85%",
                    "-h",
                    "80%",
                    "agymux switch",
                ])
                .status();
        }
        PaletteAction::SwitchProject => {
            let _ = Command::new("tmux")
                .args([
                    "display-popup",
                    "-E",
                    "-w",
                    "70%",
                    "-h",
                    "70%",
                    "agymux project",
                ])
                .status();
        }
        PaletteAction::Shortcuts => {
            let _ = Command::new("tmux")
                .args([
                    "display-popup",
                    "-E",
                    "-w",
                    "65%",
                    "-h",
                    "70%",
                    "agymux helper",
                ])
                .status();
        }
        PaletteAction::Detach => {
            let _ = Command::new("tmux").arg("detach-client").status();
        }
        PaletteAction::QuitKeepTabs => {
            let _ = Command::new("tmux").arg("detach-client").status();
        }
        PaletteAction::KillSession => {
            // Collect all pane PIDs across windows in the current session
            let mut pids = Vec::new();
            if let Ok(output) = Command::new("tmux")
                .args(["list-panes", "-s", "-F", "#{pane_pid}"])
                .output()
            {
                if output.status.success() {
                    let out_str = String::from_utf8_lossy(&output.stdout);
                    for line in out_str.lines() {
                        if let Ok(pid) = line.trim().parse::<i32>() {
                            if pid > 1 {
                                pids.push(pid);
                            }
                        }
                    }
                }
            }

            // Terminate child processes and their process groups (SIGTERM then SIGKILL)
            for &pid in &pids {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-pid, libc::SIGTERM);
                    libc::kill(pid, libc::SIGTERM);
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(150));

            for &pid in &pids {
                #[cfg(unix)]
                unsafe {
                    libc::kill(-pid, libc::SIGKILL);
                    libc::kill(pid, libc::SIGKILL);
                }
            }

            let _ = Command::new("tmux").arg("kill-session").status();
        }
        PaletteAction::BindConversation => {
            let _ = Command::new("tmux")
                .args([
                    "display-popup",
                    "-E",
                    "-w",
                    "75%",
                    "-h",
                    "60%",
                    "agymux bind",
                ])
                .status();
        }
        PaletteAction::NewTerminalTab => {
            let _ = Command::new("tmux")
                .args(["new-window", "-n", "term", "$SHELL"])
                .status();
        }
        PaletteAction::SwitchToMain => {
            let _ = Command::new("tmux")
                .args([
                    "run-shell",
                    "tmux new-session -d -s main 2>/dev/null; tmux switch-client -t main",
                ])
                .status();
        }
        PaletteAction::ReloadActiveSession => {
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args(["set-option", "-w", "@external_sync", "0"])
                    .status();
            }

            let cid_output = Command::new("tmux")
                .args(["display-message", "-p", "#{@conversation_id}"])
                .output();
            let current_cid = cid_output
                .ok()
                .and_then(|o| String::from_utf8(o.stdout).ok())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty() && s != "-")
                .or_else(|| std::env::var("CONVERSATION_ID").ok())
                .filter(|s| !s.is_empty())
                .or_else(|| {
                    let db = crate::db_path();
                    if db.exists() {
                        rusqlite::Connection::open(&db).ok().and_then(|conn| {
                            crate::core::session::latest_conversation_id(&conn, Some(proj))
                        })
                    } else {
                        None
                    }
                });

            if let Some(cid) = current_cid {
                let config = crate::config::Config::load().unwrap_or_default();
                let rc_flag = if config.general.remote_control {
                    " --remote-control"
                } else {
                    ""
                };
                let reload_cmd = format!(
                    "agymux run --project '{}' --conversation '{}'{}",
                    proj, cid, rc_flag
                );

                if std::env::var("TMUX").is_ok() {
                    let _ = Command::new("tmux")
                        .args(["respawn-pane", "-k", &reload_cmd])
                        .status();
                    let toast = format!("Reloaded session: {}", cid);
                    let _ = Command::new("tmux")
                        .args(["display-message", &toast])
                        .status();
                } else {
                    println!("Reloading session: {}", cid);
                    let mut cmd = Command::new("agy");
                    cmd.args(["--project", proj, "--conversation", &cid]);
                    if config.general.remote_control {
                        cmd.arg("--remote-control");
                    }
                    let _ = cmd.status();
                }
            } else if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args(["display-message", "No active conversation bound to reload"])
                    .status();
            } else {
                println!("No active conversation bound to reload");
            }
        }
        PaletteAction::SplitHorizontal => {
            let _ = Command::new("tmux")
                .args(["split-window", "-h", "-c", "#{pane_current_path}"])
                .status();
        }
        PaletteAction::SplitVertical => {
            let _ = Command::new("tmux")
                .args(["split-window", "-v", "-c", "#{pane_current_path}"])
                .status();
        }
        PaletteAction::TogglePaneZoom => {
            let _ = Command::new("tmux").args(["resize-pane", "-Z"]).status();
        }
        PaletteAction::BreakPane => {
            let _ = Command::new("tmux").arg("break-pane").status();
        }
        PaletteAction::CopyMode => {
            let _ = Command::new("tmux").arg("copy-mode").status();
        }
        PaletteAction::ClosePane => {
            let _ = Command::new("tmux")
                .args(["confirm-before", "-p", "kill-pane #P? (y/n)", "kill-pane"])
                .status();
        }
        PaletteAction::NewClaudeSession => {
            let _ = Command::new("tmux")
                .args([
                    "new-window",
                    "-n",
                    "+ new",
                    &format!("agymux run -H claude --project '{}'", proj),
                ])
                .status();
        }
        PaletteAction::NewAntigravitySession => {
            let _ = Command::new("tmux")
                .args([
                    "new-window",
                    "-n",
                    "+ new",
                    &format!("agymux run -H agy --project '{}'", proj),
                ])
                .status();
        }
        PaletteAction::MoveTabLeft => {
            let _ = Command::new("tmux")
                .args(["swap-window", "-d", "-t", "-1"])
                .status();
        }
        PaletteAction::MoveTabRight => {
            let _ = Command::new("tmux")
                .args(["swap-window", "-d", "-t", "+1"])
                .status();
        }
    }
    Ok(())
}

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
}

#[derive(Debug, Clone)]
pub struct PaletteItem {
    pub action: PaletteAction,
    pub icon: &'static str,
    pub title: &'static str,
    pub shortcut: &'static str,
}

impl PaletteItem {
    pub fn all() -> Vec<Self> {
        vec![
            PaletteItem {
                action: PaletteAction::RenameTab,
                icon: "📝",
                title: "Rename Tab",
                shortcut: "Ctrl+Space r / к",
            },
            PaletteItem {
                action: PaletteAction::NewTab,
                icon: "➕",
                title: "New Tab",
                shortcut: "Ctrl+Space c / с",
            },
            PaletteItem {
                action: PaletteAction::ForkConversation,
                icon: "🌿",
                title: "Fork Conversation",
                shortcut: "Ctrl+Space f / а",
            },
            PaletteItem {
                action: PaletteAction::NewWorktreeTab,
                icon: "📁",
                title: "New Worktree Tab",
                shortcut: "Ctrl+Space w / ц",
            },
            PaletteItem {
                action: PaletteAction::LaunchCustomFlags,
                icon: "⚙️ ",
                title: "Launch with Custom Flags",
                shortcut: "Ctrl+Space l / д",
            },
            PaletteItem {
                action: PaletteAction::CloseTab,
                icon: "🧹",
                title: "Close Tab",
                shortcut: "Ctrl+Space x / ч",
            },
            PaletteItem {
                action: PaletteAction::SwitchTab,
                icon: "🗂️ ",
                title: "Switch Tab / Session",
                shortcut: "Ctrl+Space s / ы",
            },
            PaletteItem {
                action: PaletteAction::SwitchProject,
                icon: "🌐",
                title: "Switch Project",
                shortcut: "Ctrl+Space P / З",
            },
            PaletteItem {
                action: PaletteAction::Shortcuts,
                icon: "❓",
                title: "Keyboard Shortcuts",
                shortcut: "Ctrl+Space ? / h / р",
            },
            PaletteItem {
                action: PaletteAction::Detach,
                icon: "🚪",
                title: "Detach Session",
                shortcut: "Ctrl+Space d / в",
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
        let q = self.query.to_lowercase();
        self.filtered_indices = self
            .items
            .iter()
            .enumerate()
            .filter(|(_, item)| {
                if q.is_empty() {
                    return true;
                }
                item.title.to_lowercase().contains(&q) || item.shortcut.to_lowercase().contains(&q)
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

pub fn render(f: &mut Frame, area: Rect, state: &PaletteState) {
    let width = 68.min(area.width.saturating_sub(4));
    let height = 18.min(area.height.saturating_sub(2));

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

            let line = Line::from(vec![
                Span::styled(format!(" {} ", item.icon), Style::default()),
                Span::styled(
                    format!("{:<26}", item.title),
                    if is_selected {
                        Style::default()
                            .fg(COLOR_GREEN)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                ),
                Span::styled(
                    format!("({})", item.shortcut),
                    if is_selected {
                        Style::default().fg(COLOR_YELLOW)
                    } else {
                        muted_style()
                    },
                ),
            ]);

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
    }
    Ok(())
}

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};

use crate::config::StartupAction;
use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_PINK, COLOR_PURPLE, COLOR_YELLOW,
};

pub struct StartupOption {
    pub action: StartupAction,
    pub key_label: &'static str,
    pub title: String,
    pub subtitle: String,
}

pub struct StartupState {
    pub options: Vec<StartupOption>,
    pub selected: usize,
    pub remember: bool,
    pub project: String,
}

impl StartupState {
    pub fn new(project: String, latest_summary: Option<(String, String)>) -> Self {
        let resume_desc = if let Some((title, time)) = latest_summary {
            format!("\"{}\" ({})", title, time)
        } else {
            "No prior conversations for this project".to_string()
        };

        let options = vec![
            StartupOption {
                action: StartupAction::New,
                key_label: "1",
                title: "➕ Start a new conversation".to_string(),
                subtitle: "Fresh tab (+ new) running Google Antigravity".to_string(),
            },
            StartupOption {
                action: StartupAction::ResumeLatest,
                key_label: "2",
                title: "🔄 Resume latest conversation".to_string(),
                subtitle: resume_desc,
            },
            StartupOption {
                action: StartupAction::Picker,
                key_label: "3",
                title: "🗂️ Browse conversation picker".to_string(),
                subtitle: "Search and preview historical project turns".to_string(),
            },
            StartupOption {
                action: StartupAction::Terminal,
                key_label: "4",
                title: "💻 Open plain terminal session (main)".to_string(),
                subtitle: "Standard shell without running agy".to_string(),
            },
        ];

        Self {
            options,
            selected: 1, // Default select ResumeLatest if available
            remember: false,
            project,
        }
    }

    pub fn select_next(&mut self) {
        if !self.options.is_empty() {
            self.selected = (self.selected + 1) % self.options.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.options.is_empty() {
            if self.selected == 0 {
                self.selected = self.options.len() - 1;
            } else {
                self.selected -= 1;
            }
        }
    }

    pub fn toggle_remember(&mut self) {
        self.remember = !self.remember;
    }

    pub fn selected_action(&self) -> StartupAction {
        self.options
            .get(self.selected)
            .map(|o| o.action)
            .unwrap_or(StartupAction::New)
    }
}

pub enum StartupResult {
    Continue,
    Confirm(StartupAction, bool),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut StartupState) -> StartupResult {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('й') => StartupResult::Cancel,
        KeyCode::Enter => StartupResult::Confirm(state.selected_action(), state.remember),
        KeyCode::Char('1') => {
            state.selected = 0;
            StartupResult::Confirm(StartupAction::New, state.remember)
        }
        KeyCode::Char('2') => {
            state.selected = 1;
            StartupResult::Confirm(StartupAction::ResumeLatest, state.remember)
        }
        KeyCode::Char('3') => {
            state.selected = 2;
            StartupResult::Confirm(StartupAction::Picker, state.remember)
        }
        KeyCode::Char('4') => {
            state.selected = 3;
            StartupResult::Confirm(StartupAction::Terminal, state.remember)
        }
        KeyCode::Char(' ')
        | KeyCode::Char('r')
        | KeyCode::Char('R')
        | KeyCode::Char('к')
        | KeyCode::Char('К') => {
            state.toggle_remember();
            StartupResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k')
            if key.modifiers.contains(KeyModifiers::CONTROL) || key.code == KeyCode::Up =>
        {
            state.select_prev();
            StartupResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j')
            if key.modifiers.contains(KeyModifiers::CONTROL) || key.code == KeyCode::Down =>
        {
            state.select_next();
            StartupResult::Continue
        }
        _ => StartupResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &StartupState) {
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
            Constraint::Length(3), // Header title
            Constraint::Min(8),    // Options list
            Constraint::Length(3), // Remember checkbox & hint
        ])
        .split(modal_area);

    // 1. Header
    let block = styled_block("agymux Startup Lifecycle", true);
    let title_p = Paragraph::new(Line::from(vec![
        Span::styled(" Project: ", Style::default().fg(COLOR_CYAN)),
        Span::styled(
            &state.project,
            Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD),
        ),
        Span::styled("  —  Select startup action", muted_style()),
    ]))
    .block(block)
    .style(base_style());
    f.render_widget(title_p, chunks[0]);

    // 2. Options List
    let items: Vec<ListItem> = state
        .options
        .iter()
        .enumerate()
        .map(|(idx, opt)| {
            let is_selected = idx == state.selected;
            let line1 = Line::from(vec![
                Span::styled(
                    format!(" [{}] ", opt.key_label),
                    if is_selected {
                        Style::default()
                            .fg(COLOR_YELLOW)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                ),
                Span::styled(
                    &opt.title,
                    if is_selected {
                        Style::default()
                            .fg(COLOR_GREEN)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                ),
            ]);
            let line2 = Line::from(vec![
                Span::raw("     "),
                Span::styled(
                    &opt.subtitle,
                    if is_selected {
                        Style::default().fg(COLOR_PURPLE)
                    } else {
                        muted_style()
                    },
                ),
            ]);

            if is_selected {
                ListItem::new(vec![line1, line2]).style(highlight_style())
            } else {
                ListItem::new(vec![line1, line2]).style(base_style())
            }
        })
        .collect();

    let list_block = styled_block("Choose Action (1-4)", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // 3. Remember checkbox & footer
    let checkbox = if state.remember { "[x]" } else { "[ ]" };
    let footer_p = Paragraph::new(vec![
        Line::from(vec![Span::styled(
            format!(
                "  {} Remember selection as default in ~/.config/agymux/config.toml (Space/r)",
                checkbox
            ),
            if state.remember {
                Style::default()
                    .fg(COLOR_GREEN)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_YELLOW)
            },
        )]),
        Line::from(vec![
            Span::styled("  Enter: confirm  ", Style::default().fg(COLOR_GREEN)),
            Span::styled("1-4: direct select  ", muted_style()),
            Span::styled("Esc: cancel", Style::default().fg(COLOR_PINK)),
        ]),
    ])
    .style(base_style());
    f.render_widget(footer_p, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_startup_state_selection_and_toggles() {
        let mut state = StartupState::new(
            "test-project".to_string(),
            Some(("Refactor auth".to_string(), "5m ago".to_string())),
        );

        assert_eq!(state.options.len(), 4);
        assert_eq!(state.selected, 1);
        assert!(!state.remember);
        assert_eq!(state.selected_action(), StartupAction::ResumeLatest);

        // Next / Prev
        state.select_next();
        assert_eq!(state.selected, 2);
        assert_eq!(state.selected_action(), StartupAction::Picker);

        state.select_next();
        assert_eq!(state.selected, 3);
        assert_eq!(state.selected_action(), StartupAction::Terminal);

        state.select_prev();
        assert_eq!(state.selected, 2);

        // Toggle remember
        state.toggle_remember();
        assert!(state.remember);
        state.toggle_remember();
        assert!(!state.remember);
    }

    #[test]
    fn test_startup_handle_key_direct_numbers() {
        let mut state = StartupState::new("test-project".to_string(), None);

        // Direct '1'
        match handle_key(
            KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE),
            &mut state,
        ) {
            StartupResult::Confirm(act, rem) => {
                assert_eq!(act, StartupAction::New);
                assert!(!rem);
            }
            _ => panic!("Expected Confirm"),
        }

        // Direct '4' with remember toggled
        state.remember = true;
        match handle_key(
            KeyEvent::new(KeyCode::Char('4'), KeyModifiers::NONE),
            &mut state,
        ) {
            StartupResult::Confirm(act, rem) => {
                assert_eq!(act, StartupAction::Terminal);
                assert!(rem);
            }
            _ => panic!("Expected Confirm"),
        }

        // Esc cancels
        match handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &mut state) {
            StartupResult::Cancel => {}
            _ => panic!("Expected Cancel"),
        }
    }
}

use anyhow::Result;
use crossterm::event::{Event, KeyCode, KeyEvent, MouseButton, MouseEvent, MouseEventKind};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::path::Path;
use std::process::Command;

use crate::ui::theme::{
    base_style, highlight_style, muted_style, COLOR_CYAN, COLOR_FG, COLOR_GREEN, COLOR_MUTED,
    COLOR_PINK, COLOR_PURPLE, COLOR_YELLOW,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RailWindowItem {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub active: bool,
    pub remote_control: bool,
    pub external_sync: bool,
    pub conversation_id: Option<String>,
    pub branch: Option<String>,
    pub step_count: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RailAction {
    SelectWindow(String),
    CloseWindow(String, usize),
    Quit,
}

pub struct RailState {
    pub items: Vec<RailWindowItem>,
    pub selected: usize,
}

impl Default for RailState {
    fn default() -> Self {
        Self::new()
    }
}

impl RailState {
    pub fn new() -> Self {
        let items = Self::fetch_windows();
        let selected = items.iter().position(|w| w.active).unwrap_or(0);
        Self { items, selected }
    }

    pub fn refresh(&mut self) {
        let fetched = Self::fetch_windows();
        if !fetched.is_empty() {
            if self.selected >= fetched.len() {
                self.selected = fetched.len().saturating_sub(1);
            }
            self.items = fetched;
        }
    }

    pub fn fetch_windows() -> Vec<RailWindowItem> {
        let output = Command::new("tmux")
            .args([
                "list-windows",
                "-F",
                "#{window_id}:#{window_index}:#{window_name}:#{window_active}:#{@remote_control}:#{@external_sync}:#{@conversation_id}:#{pane_current_path}",
            ])
            .output();

        let Ok(out) = output else {
            return Vec::new();
        };

        let db = crate::db_path();
        let conn = if db.exists() {
            rusqlite::Connection::open(&db).ok()
        } else {
            None
        };

        let mut items = Vec::new();
        let raw = String::from_utf8_lossy(&out.stdout);

        for line in raw.lines() {
            let parts: Vec<&str> = line.split(':').collect();
            if parts.len() >= 4 {
                let id = parts[0].to_string();
                let index = parts[1].parse::<usize>().unwrap_or(1);
                let name = parts[2].to_string();
                let active = parts[3] == "1";
                let rc = parts.get(4).copied().unwrap_or("") == "1";
                let sync = parts.get(5).copied().unwrap_or("") == "1";
                let cid = parts
                    .get(6)
                    .copied()
                    .filter(|s| !s.is_empty() && *s != "-")
                    .map(|s| s.to_string());
                let path = parts.get(7).copied().unwrap_or("");

                let branch = if !path.is_empty() {
                    get_git_branch(path)
                } else {
                    None
                };

                let step_count = if let (Some(ref c), Some(ref conn_ref)) = (&cid, &conn) {
                    conn_ref
                        .query_row(
                            "SELECT step_count FROM conversation_summaries WHERE conversation_id = ?1 LIMIT 1",
                            [c],
                            |r| r.get::<_, i64>(0),
                        )
                        .ok()
                } else {
                    None
                };

                items.push(RailWindowItem {
                    id,
                    index,
                    name,
                    active,
                    remote_control: rc,
                    external_sync: sync,
                    conversation_id: cid,
                    branch,
                    step_count,
                });
            }
        }

        items
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        } else if !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }

    pub fn move_down(&mut self) {
        if !self.items.is_empty() {
            if self.selected + 1 < self.items.len() {
                self.selected += 1;
            } else {
                self.selected = 0;
            }
        }
    }
}

fn get_git_branch(path_str: &str) -> Option<String> {
    let p = Path::new(path_str);
    let head = p.join(".git").join("HEAD");
    if head.exists() {
        if let Ok(content) = std::fs::read_to_string(head) {
            let trimmed = content.trim();
            if let Some(branch) = trimmed.strip_prefix("ref: refs/heads/") {
                return Some(branch.to_string());
            }
        }
    }
    None
}

pub fn handle_event(event: Event, state: &mut RailState) -> Result<Option<RailAction>> {
    match event {
        Event::Key(key) => handle_key(key, state),
        Event::Mouse(mouse) => handle_mouse(mouse, state),
        _ => Ok(None),
    }
}

pub fn handle_key(key: KeyEvent, state: &mut RailState) -> Result<Option<RailAction>> {
    match key.code {
        KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('й') => Ok(Some(RailAction::Quit)),
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => {
            state.move_up();
            Ok(None)
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => {
            state.move_down();
            Ok(None)
        }
        KeyCode::Enter => {
            if let Some(item) = state.items.get(state.selected) {
                Ok(Some(RailAction::SelectWindow(item.id.clone())))
            } else {
                Ok(None)
            }
        }
        KeyCode::Char('x') | KeyCode::Char('ч') => {
            if let Some(item) = state.items.get(state.selected) {
                let win_count = state.items.len();
                Ok(Some(RailAction::CloseWindow(item.id.clone(), win_count)))
            } else {
                Ok(None)
            }
        }
        KeyCode::Char(c) if c.is_ascii_digit() && c != '0' => {
            let digit = c.to_digit(10).unwrap_or(0) as usize;
            if let Some(item) = state.items.iter().find(|w| w.index == digit) {
                Ok(Some(RailAction::SelectWindow(item.id.clone())))
            } else {
                Ok(None)
            }
        }
        _ => Ok(None),
    }
}

pub fn handle_mouse(mouse: MouseEvent, state: &mut RailState) -> Result<Option<RailAction>> {
    if mouse.kind == MouseEventKind::Down(MouseButton::Left) {
        // Line 0 is header border
        let row = mouse.row as usize;
        let is_compact = mouse.column < 12;
        let item_height = if is_compact { 1 } else { 2 };

        if row >= 1 {
            let item_index = (row - 1) / item_height;
            if let Some(item) = state.items.get(item_index) {
                state.selected = item_index;
                // If close button clicked (near right edge)
                if mouse.column >= 38 {
                    let win_count = state.items.len();
                    return Ok(Some(RailAction::CloseWindow(item.id.clone(), win_count)));
                } else {
                    return Ok(Some(RailAction::SelectWindow(item.id.clone())));
                }
            }
        }
    }
    Ok(None)
}

pub fn render(f: &mut Frame, area: Rect, state: &RailState) {
    f.render_widget(Clear, area);

    if area.width < 12 {
        render_compact(f, area, state);
    } else {
        render_standard(f, area, state);
    }
}

fn render_compact(f: &mut Frame, area: Rect, state: &RailState) {
    let block = Block::default()
        .borders(Borders::RIGHT)
        .border_style(Style::default().fg(COLOR_MUTED))
        .style(base_style());

    let inner = block.inner(area);
    f.render_widget(block, area);

    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_sel = idx == state.selected;
            let dot = if item.active {
                Span::styled("●", Style::default().fg(COLOR_GREEN))
            } else if item.external_sync {
                Span::styled("󰛌", Style::default().fg(COLOR_CYAN))
            } else if item.remote_control {
                Span::styled("󰢹", Style::default().fg(COLOR_PURPLE))
            } else {
                Span::styled("·", Style::default().fg(COLOR_MUTED))
            };

            let line = Line::from(vec![
                Span::styled(
                    format!("{} ", item.index),
                    if is_sel {
                        Style::default()
                            .fg(COLOR_YELLOW)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                ),
                dot,
            ]);

            if is_sel {
                ListItem::new(line).style(highlight_style())
            } else {
                ListItem::new(line).style(base_style())
            }
        })
        .collect();

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(List::new(items), inner, &mut list_state);
}

fn render_standard(f: &mut Frame, area: Rect, state: &RailState) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header title
            Constraint::Min(4),    // Windows list
            Constraint::Length(1), // Footer hint
        ])
        .split(area);

    // 1. Header
    let header_block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(COLOR_CYAN))
        .style(base_style());
    let header_p = Paragraph::new(Line::from(vec![Span::styled(
        " ▌ AGYMUX SESSIONS",
        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD),
    )]))
    .block(header_block);
    f.render_widget(header_p, chunks[0]);

    // 2. Windows List
    let items: Vec<ListItem> = state
        .items
        .iter()
        .enumerate()
        .map(|(idx, item)| {
            let is_sel = idx == state.selected;

            // Indicator symbol
            let (indicator_glyph, indicator_style) = if item.active {
                ("● ", Style::default().fg(COLOR_GREEN))
            } else if item.external_sync {
                ("󰛌 ", Style::default().fg(COLOR_CYAN))
            } else {
                ("  ", muted_style())
            };

            let rc_span = if item.remote_control {
                Span::styled(" 󰢹", Style::default().fg(COLOR_PURPLE))
            } else {
                Span::raw("")
            };

            let close_span = Span::styled(
                " 󰅖",
                if is_sel {
                    Style::default().fg(COLOR_PINK)
                } else {
                    muted_style()
                },
            );

            let max_name_len = (area.width as usize).saturating_sub(14).max(6);
            let display_name = if item.name.len() > max_name_len {
                format!("{}…", &item.name[..max_name_len - 1])
            } else {
                item.name.clone()
            };

            let line1 = Line::from(vec![
                Span::styled(
                    format!("{:<2} ", item.index),
                    if is_sel {
                        Style::default()
                            .fg(COLOR_YELLOW)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                ),
                Span::styled(indicator_glyph, indicator_style),
                Span::styled(
                    display_name,
                    if is_sel {
                        Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                ),
                rc_span,
                close_span,
            ]);

            // Subline: branch and step count
            let branch_str = item.branch.as_deref().unwrap_or("main");
            let turns_str = if let Some(steps) = item.step_count {
                format!(" · {} turns", steps)
            } else {
                String::new()
            };

            let line2 = Line::from(vec![
                Span::styled("   ⎇ ", Style::default().fg(Color::Rgb(120, 120, 140))),
                Span::styled(
                    format!("{}{}", branch_str, turns_str),
                    Style::default().fg(COLOR_MUTED),
                ),
            ]);

            if is_sel {
                ListItem::new(vec![line1, line2]).style(highlight_style())
            } else {
                ListItem::new(vec![line1, line2]).style(base_style())
            }
        })
        .collect();

    let list_block = Block::default()
        .borders(Borders::LEFT | Borders::RIGHT)
        .border_style(Style::default().fg(COLOR_MUTED))
        .style(base_style());

    let list = List::new(items).block(list_block);
    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // 3. Footer Hint
    let footer_p = Paragraph::new(Line::from(vec![
        Span::styled(" 1..9: jump  ", Style::default().fg(COLOR_YELLOW)),
        Span::styled(" x: close  ", Style::default().fg(COLOR_PINK)),
        Span::styled(" q: exit", muted_style()),
    ]))
    .style(base_style());
    f.render_widget(footer_p, chunks[2]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rail_state_navigation() {
        let mut state = RailState {
            items: vec![
                RailWindowItem {
                    id: "@1".to_string(),
                    index: 1,
                    name: "main".to_string(),
                    active: true,
                    remote_control: false,
                    external_sync: false,
                    conversation_id: None,
                    branch: Some("main".to_string()),
                    step_count: Some(5),
                },
                RailWindowItem {
                    id: "@2".to_string(),
                    index: 2,
                    name: "feature".to_string(),
                    active: false,
                    remote_control: true,
                    external_sync: true,
                    conversation_id: Some("cid-test".to_string()),
                    branch: Some("feat/rail".to_string()),
                    step_count: Some(10),
                },
            ],
            selected: 0,
        };

        state.move_down();
        assert_eq!(state.selected, 1);
        state.move_down();
        assert_eq!(state.selected, 0); // wrap
        state.move_up();
        assert_eq!(state.selected, 1); // wrap up
    }

    #[test]
    fn test_rail_handle_keys() {
        let mut state = RailState {
            items: vec![
                RailWindowItem {
                    id: "@1".to_string(),
                    index: 1,
                    name: "tab1".to_string(),
                    active: true,
                    remote_control: false,
                    external_sync: false,
                    conversation_id: None,
                    branch: None,
                    step_count: None,
                },
                RailWindowItem {
                    id: "@2".to_string(),
                    index: 2,
                    name: "tab2".to_string(),
                    active: false,
                    remote_control: false,
                    external_sync: false,
                    conversation_id: None,
                    branch: None,
                    step_count: None,
                },
            ],
            selected: 0,
        };

        // Jump directly via digit 2
        let key2 = KeyEvent::new(KeyCode::Char('2'), crossterm::event::KeyModifiers::NONE);
        let action = handle_key(key2, &mut state).unwrap();
        assert_eq!(action, Some(RailAction::SelectWindow("@2".to_string())));

        // Enter selects current item
        let key_enter = KeyEvent::new(KeyCode::Enter, crossterm::event::KeyModifiers::NONE);
        let action = handle_key(key_enter, &mut state).unwrap();
        assert_eq!(action, Some(RailAction::SelectWindow("@1".to_string())));

        // 'x' triggers CloseWindow
        let key_x = KeyEvent::new(KeyCode::Char('x'), crossterm::event::KeyModifiers::NONE);
        let action = handle_key(key_x, &mut state).unwrap();
        assert_eq!(action, Some(RailAction::CloseWindow("@1".to_string(), 2)));

        // 'q' triggers Quit
        let key_q = KeyEvent::new(KeyCode::Char('q'), crossterm::event::KeyModifiers::NONE);
        let action = handle_key(key_q, &mut state).unwrap();
        assert_eq!(action, Some(RailAction::Quit));
    }
}

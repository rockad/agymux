use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
};
use walkdir::WalkDir;

use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_PINK,
};

#[derive(Debug, Clone)]
pub struct ProjectItem {
    pub name: String,
    pub path: PathBuf,
    pub is_active: bool,
    pub tab_count: usize,
    pub attached_clients: usize,
}

pub struct ProjectPickerState {
    pub projects: Vec<ProjectItem>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub query: String,
}

impl ProjectPickerState {
    pub fn new(base_dir: &Path) -> Self {
        let projects = discover_projects(base_dir);
        let filtered_indices = (0..projects.len()).collect();
        Self {
            projects,
            filtered_indices,
            selected: 0,
            query: String::new(),
        }
    }

    pub fn filter(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered_indices = (0..self.projects.len()).collect();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let mut query_buf = Vec::new();
            let needle = Utf32Str::new(&self.query, &mut query_buf);

            let mut scored = Vec::new();
            for (idx, p) in self.projects.iter().enumerate() {
                let haystack_str = format!("{} {}", p.name, p.path.display());
                let mut haystack_buf = Vec::new();
                let haystack = Utf32Str::new(&haystack_str, &mut haystack_buf);
                if let Some(score) = matcher.fuzzy_match(haystack, needle) {
                    scored.push((score, idx));
                }
            }

            scored.sort_by(|a, b| b.0.cmp(&a.0));
            self.filtered_indices = scored.into_iter().map(|(_, idx)| idx).collect();
        }

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

    pub fn selected_project(&self) -> Option<&ProjectItem> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.projects.get(idx))
    }
}

pub enum ProjectPickerResult {
    Continue,
    Select(ProjectItem),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut ProjectPickerState) -> ProjectPickerResult {
    match key.code {
        KeyCode::Esc => ProjectPickerResult::Cancel,
        KeyCode::Enter => {
            if let Some(p) = state.selected_project() {
                ProjectPickerResult::Select(p.clone())
            } else {
                ProjectPickerResult::Cancel
            }
        }
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_prev();
            ProjectPickerResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_next();
            ProjectPickerResult::Continue
        }
        KeyCode::Up => {
            state.select_prev();
            ProjectPickerResult::Continue
        }
        KeyCode::Down => {
            state.select_next();
            ProjectPickerResult::Continue
        }
        KeyCode::Backspace => {
            state.query.pop();
            state.filter();
            ProjectPickerResult::Continue
        }
        KeyCode::Char(c) => {
            state.query.push(c);
            state.filter();
            ProjectPickerResult::Continue
        }
        _ => ProjectPickerResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &ProjectPickerState) {
    let width = 76.min(area.width.saturating_sub(4));
    let height = 24.min(area.height.saturating_sub(2));

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
            Constraint::Length(3), // Search bar
            Constraint::Min(4),    // Project list
            Constraint::Length(1), // Footer hint
        ])
        .split(modal_area);

    // 1. Search Bar
    let search_block = styled_block("Global Project Switcher", true);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled("🌐 Switch Project > ", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(&state.query, Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(COLOR_PINK)),
    ]))
    .block(search_block)
    .style(base_style());
    f.render_widget(search_p, chunks[0]);

    // 2. Project List
    let items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &item_idx)| {
            let p = &state.projects[item_idx];
            let is_selected = display_idx == state.selected;

            let status_badge = if p.is_active {
                Span::styled(
                    format!("● active ({} tabs)", p.tab_count),
                    Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("○ inactive", muted_style())
            };

            let name_style = if is_selected {
                Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD)
            };

            let line = Line::from(vec![
                Span::styled(format!(" {:<24} ", p.name), name_style),
                status_badge,
                Span::raw("   "),
                Span::styled(p.path.display().to_string(), muted_style()),
            ]);

            if is_selected {
                ListItem::new(line).style(highlight_style())
            } else {
                ListItem::new(line).style(base_style())
            }
        })
        .collect();

    let list_block = styled_block("Projects", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, chunks[1], &mut list_state);

    // 3. Footer
    let footer_p = Paragraph::new(Line::from(vec![
        Span::styled(" Enter: switch to project  ", Style::default().fg(COLOR_GREEN)),
        Span::styled(" Esc: cancel  ", Style::default().fg(COLOR_PINK)),
        Span::styled(" ↑/↓: navigate ", muted_style()),
    ]))
    .style(base_style());
    f.render_widget(footer_p, chunks[2]);
}

/// Discover git repositories and query tmux active sessions
fn discover_projects(base_dir: &Path) -> Vec<ProjectItem> {
    let mut projects = Vec::new();
    let mut active_sessions: HashMap<String, (usize, usize)> = HashMap::new();

    // Query tmux active sessions starting with agy-
    if let Ok(output) = Command::new("tmux")
        .args(["list-sessions", "-F", "#{session_name}\t#{session_windows}\t#{session_attached}"])
        .output()
    {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 3 {
                let sname = parts[0];
                if let Some(pname) = sname.strip_prefix("agy-") {
                    let nwins: usize = parts[1].parse().unwrap_or(0);
                    let natt: usize = parts[2].parse().unwrap_or(0);
                    active_sessions.insert(pname.to_string(), (nwins, natt));
                }
            }
        }
    }

    // Discover git projects under base_dir (depth 2)
    if base_dir.exists() {
        for entry in WalkDir::new(base_dir)
            .min_depth(1)
            .max_depth(2)
            .into_iter()
            .filter_entry(|e| !e.file_name().to_string_lossy().starts_with('.'))
            .flatten()
        {
            if entry.file_type().is_dir() {
                let p = entry.path();
                if p.join(".git").exists() {
                    let name = p
                        .file_name()
                        .map(|n| n.to_string_lossy().to_string())
                        .unwrap_or_else(|| "unknown".to_string());

                    let (is_active, tab_count, attached_clients) =
                        if let Some(&(nwins, natt)) = active_sessions.get(&name) {
                            (true, nwins, natt)
                        } else {
                            (false, 0, 0)
                        };

                    projects.push(ProjectItem {
                        name,
                        path: p.to_path_buf(),
                        is_active,
                        tab_count,
                        attached_clients,
                    });
                }
            }
        }
    }

    // Add any active sessions that weren't discovered in the filesystem directory
    for (name, (nwins, natt)) in active_sessions {
        if !projects.iter().any(|p| p.name == name) {
            let path = dirs::home_dir()
                .unwrap_or_default()
                .join("projects")
                .join(&name);
            projects.push(ProjectItem {
                name,
                path,
                is_active: true,
                tab_count: nwins,
                attached_clients: natt,
            });
        }
    }

    // Sort: active projects first, then alphabetically by name
    projects.sort_by(|a, b| {
        match b.is_active.cmp(&a.is_active) {
            std::cmp::Ordering::Equal => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            other => other,
        }
    });

    projects
}

/// Switch to or create the selected project session
pub fn execute_project_switch(project: &ProjectItem) -> anyhow::Result<()> {
    let sess_name = format!("agy-{}", project.name);
    let in_tmux = std::env::var("TMUX").is_ok();

    if in_tmux {
        let has_session = Command::new("tmux")
            .args(["has-session", "-t", &format!("={}", sess_name)])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_session {
            let _ = Command::new("tmux")
                .args(["switch-client", "-t", &format!("={}", sess_name)])
                .status();
        } else {
            let target_dir = if project.path.exists() {
                project.path.display().to_string()
            } else {
                std::env::current_dir()?.display().to_string()
            };

            let _ = Command::new("tmux")
                .args([
                    "new-session",
                    "-d",
                    "-s",
                    &sess_name,
                    "-c",
                    &target_dir,
                    "-n",
                    "main",
                    &format!("agymux run --project '{}' -c", project.name),
                ])
                .status();

            let _ = Command::new("tmux")
                .args(["switch-client", "-t", &format!("={}", sess_name)])
                .status();
        }
    } else {
        let has_session = Command::new("tmux")
            .args(["has-session", "-t", &format!("={}", sess_name)])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        if has_session {
            let _ = Command::new("tmux")
                .args(["attach-session", "-t", &format!("={}", sess_name)])
                .status();
        } else {
            let target_dir = if project.path.exists() {
                project.path.display().to_string()
            } else {
                std::env::current_dir()?.display().to_string()
            };

            let _ = Command::new("tmux")
                .args([
                    "new-session",
                    "-s",
                    &sess_name,
                    "-c",
                    &target_dir,
                    "-n",
                    "main",
                    &format!("agymux run --project '{}' -c", project.name),
                ])
                .status();
        }
    }
    Ok(())
}

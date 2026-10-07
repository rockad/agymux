use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use nucleo_matcher::{Config, Matcher, Utf32Str};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};
use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::HashMap,
    fs::File,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::Command,
};

use crate::cli::ScopeArg;
use crate::ui::theme::{
    base_style, highlight_style, muted_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN,
    COLOR_MUTED, COLOR_PINK, COLOR_PURPLE, COLOR_YELLOW,
};

#[derive(Debug, Clone)]
pub enum ItemKind {
    NewTab,
    OpenTab {
        session: String,
        window_index: usize,
        window_name: String,
        is_active: bool,
        conversation_id: Option<String>,
        pane_path: String,
    },
    Historical {
        conversation_id: String,
        title: String,
        relative_time: String,
        project: String,
        step_count: i64,
    },
}

#[derive(Debug, Clone)]
pub struct PickerCandidate {
    pub display_label: String,
    pub match_text: String,
    pub kind: ItemKind,
}

#[derive(Debug, Default, Clone)]
pub struct TranscriptInfo {
    pub initial_prompt: Option<String>,
    pub latest_turn: Option<String>,
    pub latest_type: Option<String>,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_tokens: u64,
    pub tools_count: u64,
    pub tools_breakdown: HashMap<String, u64>,
}

pub struct PickerState {
    pub scope: ScopeArg,
    pub project_name: String,
    pub workdir: PathBuf,
    pub query: String,
    pub all_candidates: Vec<PickerCandidate>,
    pub filtered_indices: Vec<usize>,
    pub selected: usize,
    pub transcript_cache: HashMap<String, TranscriptInfo>,
}

impl PickerState {
    pub fn new(scope: ScopeArg, project_name: String, workdir: PathBuf) -> Self {
        let mut state = Self {
            scope,
            project_name,
            workdir,
            query: String::new(),
            all_candidates: Vec::new(),
            filtered_indices: Vec::new(),
            selected: 0,
            transcript_cache: HashMap::new(),
        };
        state.reload_candidates();
        state
    }

    pub fn toggle_scope(&mut self) {
        self.scope = match self.scope {
            ScopeArg::Local => ScopeArg::Global,
            ScopeArg::Global => ScopeArg::Local,
        };
        self.reload_candidates();
    }

    pub fn reload_candidates(&mut self) {
        let mut candidates = Vec::new();

        // 1. New Tab
        candidates.push(PickerCandidate {
            display_label: "➕ [New Conversation Tab]".to_string(),
            match_text: "New Conversation Tab new tab fresh session".to_string(),
            kind: ItemKind::NewTab,
        });

        // 2. Open tmux tabs
        let open_tabs = load_open_tabs(self.scope, &self.project_name);
        let mut open_convs = HashMap::new();
        for tab in open_tabs {
            if let ItemKind::OpenTab {
                ref session,
                window_index,
                ref window_name,
                is_active,
                ref conversation_id,
                ..
            } = tab.kind
            {
                if let Some(ref cid) = conversation_id {
                    open_convs.insert(cid.clone(), true);
                }
                let marker = if is_active { "▶ " } else { "  " };
                let turn_icon = if window_name.contains('●') { "● " } else { "  " };
                let label = match self.scope {
                    ScopeArg::Local => format!("{}[Tab {}] {}{:<24} (open)", marker, window_index, turn_icon, window_name),
                    ScopeArg::Global => format!("{}[{}:{}] {}{:<20} (open)", marker, session, window_index, turn_icon, window_name),
                };
                let match_text = format!("{} {} {}", session, window_name, conversation_id.as_deref().unwrap_or(""));
                candidates.push(PickerCandidate {
                    display_label: label,
                    match_text,
                    kind: tab.kind,
                });
            }
        }

        // 3. Historical conversations from SQLite
        let historical = load_historical_conversations(self.scope, &self.project_name, 50);
        for hist in historical {
            if let ItemKind::Historical {
                ref conversation_id,
                ref title,
                ref relative_time,
                ref project,
                step_count: _,
            } = hist.kind
            {
                // Skip if already open in a tab
                if open_convs.contains_key(conversation_id) {
                    continue;
                }
                let label = match self.scope {
                    ScopeArg::Local => format!("   [Hist]       {:<30} ({})", title, relative_time),
                    ScopeArg::Global => format!("   [{:<10}] {:<30} ({})", project, title, relative_time),
                };
                let match_text = format!("{} {} {} {}", title, project, relative_time, conversation_id);
                candidates.push(PickerCandidate {
                    display_label: label,
                    match_text,
                    kind: hist.kind,
                });
            }
        }

        self.all_candidates = candidates;
        self.filter();
    }

    pub fn filter(&mut self) {
        if self.query.trim().is_empty() {
            self.filtered_indices = (0..self.all_candidates.len()).collect();
        } else {
            let mut matcher = Matcher::new(Config::DEFAULT);
            let mut query_buf = Vec::new();
            let needle = Utf32Str::new(&self.query, &mut query_buf);

            let mut scored = Vec::new();
            for (idx, candidate) in self.all_candidates.iter().enumerate() {
                // NewTab is always matched or kept at top if relevant
                let mut haystack_buf = Vec::new();
                let haystack = Utf32Str::new(&candidate.match_text, &mut haystack_buf);
                if let Some(score) = matcher.fuzzy_match(haystack, needle) {
                    scored.push((score, idx));
                }
            }

            // Sort descending by match score
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

    pub fn selected_candidate(&self) -> Option<&PickerCandidate> {
        self.filtered_indices
            .get(self.selected)
            .and_then(|&idx| self.all_candidates.get(idx))
    }

    pub fn get_or_load_transcript(&mut self, conv_id: &str) -> TranscriptInfo {
        if let Some(info) = self.transcript_cache.get(conv_id) {
            return info.clone();
        }

        let info = load_transcript(conv_id);
        self.transcript_cache.insert(conv_id.to_string(), info.clone());
        info
    }
}

pub enum PickerResult {
    Continue,
    Select(PickerCandidate),
    Cancel,
}

pub fn handle_key(key: KeyEvent, state: &mut PickerState) -> PickerResult {
    match key.code {
        KeyCode::Esc => PickerResult::Cancel,
        KeyCode::Enter => {
            if let Some(candidate) = state.selected_candidate() {
                PickerResult::Select(candidate.clone())
            } else {
                PickerResult::Cancel
            }
        }
        KeyCode::Tab => {
            state.toggle_scope();
            PickerResult::Continue
        }
        KeyCode::Char('a') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.toggle_scope();
            PickerResult::Continue
        }
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_prev();
            PickerResult::Continue
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            state.select_next();
            PickerResult::Continue
        }
        KeyCode::Up => {
            state.select_prev();
            PickerResult::Continue
        }
        KeyCode::Down => {
            state.select_next();
            PickerResult::Continue
        }
        KeyCode::PageUp => {
            for _ in 0..10 {
                state.select_prev();
            }
            PickerResult::Continue
        }
        KeyCode::PageDown => {
            for _ in 0..10 {
                state.select_next();
            }
            PickerResult::Continue
        }
        KeyCode::Backspace => {
            state.query.pop();
            state.filter();
            PickerResult::Continue
        }
        KeyCode::Char(c) => {
            state.query.push(c);
            state.filter();
            PickerResult::Continue
        }
        _ => PickerResult::Continue,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &mut PickerState) {
    let width = area.width.saturating_sub(4).min(140);
    let height = area.height.saturating_sub(2).min(45);

    let horiz_pad = (area.width.saturating_sub(width)) / 2;
    let vert_pad = (area.height.saturating_sub(height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width,
        height,
    };

    f.render_widget(Clear, modal_area);

    // Two-pane horizontal split: 45% left, 55% right
    let main_panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(modal_area);

    // --- LEFT PANE (45%) ---
    let left_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search prompt input
            Constraint::Min(4),    // List of candidates
            Constraint::Length(1), // Shortcut hint footer
        ])
        .split(main_panes[0]);

    // Search bar
    let scope_tag = match state.scope {
        ScopeArg::Local => "Local",
        ScopeArg::Global => "Global Search",
    };
    let prompt_label = format!("⚡ [{}] > ", scope_tag);
    let search_block = styled_block("Switcher", true);
    let search_p = Paragraph::new(Line::from(vec![
        Span::styled(prompt_label, Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(&state.query, Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD)),
        Span::styled("█", Style::default().fg(COLOR_PINK)),
    ]))
    .block(search_block)
    .style(base_style());
    f.render_widget(search_p, left_chunks[0]);

    // Candidate List
    let items: Vec<ListItem> = state
        .filtered_indices
        .iter()
        .enumerate()
        .map(|(display_idx, &item_idx)| {
            let candidate = &state.all_candidates[item_idx];
            let is_selected = display_idx == state.selected;

            let style = if is_selected {
                highlight_style()
            } else {
                base_style()
            };

            let line = match &candidate.kind {
                ItemKind::NewTab => Line::from(vec![
                    Span::styled("➕ ", Style::default().fg(COLOR_PURPLE)),
                    Span::styled(
                        &candidate.display_label,
                        if is_selected {
                            Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(COLOR_PURPLE)
                        },
                    ),
                ]),
                ItemKind::OpenTab { .. } => Line::from(vec![Span::styled(
                    &candidate.display_label,
                    if is_selected {
                        Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(COLOR_FG)
                    },
                )]),
                ItemKind::Historical { .. } => Line::from(vec![Span::styled(
                    &candidate.display_label,
                    if is_selected {
                        Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD)
                    } else {
                        muted_style()
                    },
                )]),
            };

            ListItem::new(line).style(style)
        })
        .collect();

    let list_block = styled_block("Tabs & Conversations", false);
    let list = List::new(items)
        .block(list_block)
        .highlight_style(highlight_style());

    let mut list_state = ListState::default();
    list_state.select(Some(state.selected));
    f.render_stateful_widget(list, left_chunks[1], &mut list_state);

    // Left Footer
    let left_footer = Paragraph::new(Line::from(vec![
        Span::styled("Tab/Ctrl+A: ", Style::default().fg(COLOR_YELLOW)),
        Span::styled("scope | ", muted_style()),
        Span::styled("Enter: ", Style::default().fg(COLOR_GREEN)),
        Span::styled("select | ", muted_style()),
        Span::styled("Esc: ", Style::default().fg(COLOR_PINK)),
        Span::styled("cancel", muted_style()),
    ]))
    .style(base_style());
    f.render_widget(left_footer, left_chunks[2]);

    // --- RIGHT PANE (55%) Live Rich Transcript Preview ---
    let right_block = styled_block("Transcript Preview", false);
    let preview_inner = right_block.inner(main_panes[1]);
    f.render_widget(right_block, main_panes[1]);

    let selected_candidate = state.selected_candidate().cloned();
    if let Some(candidate) = selected_candidate {
        render_preview_content(f, preview_inner, &candidate, state);
    } else {
        let empty_p = Paragraph::new(Line::from(vec![
            Span::styled("No item selected.", muted_style()),
        ]))
        .alignment(Alignment::Center);
        f.render_widget(empty_p, preview_inner);
    }
}

fn render_preview_content(
    f: &mut Frame,
    area: Rect,
    candidate: &PickerCandidate,
    state: &mut PickerState,
) {
    match &candidate.kind {
        ItemKind::NewTab => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![
                Span::styled(
                    "➕ Launch New Conversation",
                    Style::default().fg(COLOR_PURPLE).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::raw(""));
            lines.push(meta_line("Project:", &state.project_name));
            lines.push(meta_line("Working Dir:", &state.workdir.display().to_string()));
            lines.push(meta_line("Engine:", "Google Antigravity CLI (agy)"));
            lines.push(meta_line("Shortcut:", "Ctrl+Space c / с"));
            lines.push(Line::raw(""));
            lines.push(Line::from(vec![
                Span::styled("┌── BEHAVIOR ──────────────────────────────────────────┐", Style::default().fg(COLOR_MUTED)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("│  Spawns a fresh conversation in a new top tab.       │", Style::default().fg(COLOR_FG)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("│  Title names immediately from your initial prompt.   │", Style::default().fg(COLOR_FG)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("│  Running turn status displays live in status bar (●). │", Style::default().fg(COLOR_FG)),
            ]));
            lines.push(Line::from(vec![
                Span::styled("└──────────────────────────────────────────────────────┘", Style::default().fg(COLOR_MUTED)),
            ]));

            let p = Paragraph::new(lines).style(base_style()).wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }

        ItemKind::OpenTab {
            session,
            window_index,
            window_name,
            conversation_id,
            pane_path,
            ..
        } => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("🗂️  Tab #{} {}", window_index, window_name),
                    Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::raw(""));
            lines.push(meta_line("Session:", session));
            lines.push(meta_line("Window Index:", &window_index.to_string()));
            lines.push(meta_line("Path:", pane_path));

            if let Some(ref cid) = conversation_id {
                lines.push(meta_line("Conversation ID:", cid));
                let t_info = state.get_or_load_transcript(cid);
                append_transcript_lines(&mut lines, &t_info, cid);
            } else {
                lines.push(meta_line("Status:", "Active Tab (no bound conversation ID)"));
            }

            let p = Paragraph::new(lines).style(base_style()).wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }

        ItemKind::Historical {
            conversation_id,
            title,
            relative_time,
            project,
            step_count,
        } => {
            let mut lines = Vec::new();
            lines.push(Line::from(vec![
                Span::styled(
                    format!("💬 {}", title),
                    Style::default().fg(COLOR_PURPLE).add_modifier(Modifier::BOLD),
                ),
            ]));
            lines.push(Line::raw(""));
            lines.push(meta_line("Conversation ID:", conversation_id));
            lines.push(meta_line("Project:", project));
            lines.push(meta_line("Last Active:", relative_time));
            lines.push(meta_line("Step Count:", &format!("{} steps", step_count)));

            let t_info = state.get_or_load_transcript(conversation_id);
            append_transcript_lines(&mut lines, &t_info, conversation_id);

            let p = Paragraph::new(lines).style(base_style()).wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }
    }
}

fn append_transcript_lines(lines: &mut Vec<Line<'static>>, info: &TranscriptInfo, _cid: &str) {
    lines.push(Line::raw(""));
    let total_tokens = info.input_tokens + info.output_tokens + info.cache_tokens;
    lines.push(Line::from(vec![
        Span::styled("Tokens: ", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("Input: {} ", format_number(info.input_tokens)), Style::default().fg(COLOR_GREEN)),
        Span::styled(format!("| Output: {} ", format_number(info.output_tokens)), Style::default().fg(COLOR_GREEN)),
        Span::styled(format!("| Cache: {} ", format_number(info.cache_tokens)), Style::default().fg(COLOR_GREEN)),
        Span::styled(format!("(Total: {})", format_number(total_tokens)), Style::default().fg(COLOR_YELLOW)),
    ]));
    lines.push(Line::from(vec![
        Span::styled("Tools: ", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
        Span::styled(format!("{} executed", info.tools_count), Style::default().fg(COLOR_FG)),
    ]));
    lines.push(Line::raw(""));

    // Initial Prompt block
    lines.push(Line::from(vec![
        Span::styled("━━━ Initial User Prompt ━━━━━━━━━━━━━━━━━━━━━━━━━", Style::default().fg(COLOR_CYAN)),
    ]));
    if let Some(ref prompt) = info.initial_prompt {
        let clean = prompt.replace('\n', " ");
        let truncated = if clean.len() > 300 {
            format!("{}...", &clean[..300])
        } else {
            clean
        };
        lines.push(Line::from(vec![Span::styled(truncated, Style::default().fg(COLOR_FG))]));
    } else {
        lines.push(Line::from(vec![Span::styled("(No initial prompt recorded)", muted_style())]));
    }
    lines.push(Line::raw(""));

    // Latest Turn block
    lines.push(Line::from(vec![
        Span::styled(
            format!("━━━ Latest Assistant Turn [{}] ━━━━━━━━━━━━━", info.latest_type.as_deref().unwrap_or("RESPONSE")),
            Style::default().fg(COLOR_CYAN),
        ),
    ]));
    if let Some(ref turn) = info.latest_turn {
        let clean = turn.replace('\n', " ");
        let truncated = if clean.len() > 400 {
            format!("{}...", &clean[..400])
        } else {
            clean
        };
        lines.push(Line::from(vec![Span::styled(truncated, Style::default().fg(COLOR_FG))]));
    } else {
        lines.push(Line::from(vec![Span::styled("(No assistant response yet)", muted_style())]));
    }
    lines.push(Line::raw(""));

    // Executed Tools Breakdown
    if !info.tools_breakdown.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("━━━ Executed Tools Breakdown ━━━━━━━━━━━━━━━━━━━━", Style::default().fg(COLOR_CYAN)),
        ]));
        let mut sorted_tools: Vec<(&String, &u64)> = info.tools_breakdown.iter().collect();
        sorted_tools.sort_by(|a, b| b.1.cmp(a.1));
        for (name, count) in sorted_tools.into_iter().take(8) {
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(COLOR_GREEN)),
                Span::styled(format!("{:<20}", name), Style::default().fg(COLOR_FG).add_modifier(Modifier::BOLD)),
                Span::styled(format!(": {}", count), Style::default().fg(COLOR_YELLOW)),
            ]));
        }
    }
}

fn meta_line(label: &str, value: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("  {:<18}", label), Style::default().fg(COLOR_CYAN)),
        Span::styled(value.to_string(), Style::default().fg(COLOR_FG)),
    ])
}

fn format_number(num: u64) -> String {
    if num >= 1_000_000 {
        format!("{:.1}M", num as f64 / 1_000_000.0)
    } else if num >= 1_000 {
        format!("{:.1}k", num as f64 / 1_000.0)
    } else {
        num.to_string()
    }
}

/// Query active tmux windows
fn load_open_tabs(scope: ScopeArg, _project: &str) -> Vec<PickerCandidate> {
    let mut items = Vec::new();
    let format = "#{session_name}\t#{window_index}\t#{window_name}\t#{@conversation_id}\t#{window_active}\t#{pane_current_path}";

    let output = match scope {
        ScopeArg::Global => Command::new("tmux")
            .args(["list-windows", "-a", "-F", format])
            .output(),
        ScopeArg::Local => Command::new("tmux")
            .args(["list-windows", "-F", format])
            .output(),
    };

    if let Ok(out) = output {
        let stdout = String::from_utf8_lossy(&out.stdout);
        for line in stdout.lines() {
            let parts: Vec<&str> = line.split('\t').collect();
            if parts.len() >= 6 {
                let session = parts[0].to_string();
                let win_idx: usize = parts[1].parse().unwrap_or(0);
                let win_name = parts[2].to_string();
                let conv_id = if parts[3].is_empty() || parts[3] == "-" {
                    None
                } else {
                    Some(parts[3].to_string())
                };
                let is_active = parts[4] == "1";
                let pane_path = parts[5].to_string();

                if scope == ScopeArg::Global && !session.starts_with("agy-") {
                    continue;
                }

                items.push(PickerCandidate {
                    display_label: String::new(),
                    match_text: String::new(),
                    kind: ItemKind::OpenTab {
                        session,
                        window_index: win_idx,
                        window_name: win_name,
                        is_active,
                        conversation_id: conv_id,
                        pane_path,
                    },
                });
            }
        }
    }
    items
}

/// Query historical conversations from SQLite
fn load_historical_conversations(
    scope: ScopeArg,
    project: &str,
    limit: usize,
) -> Vec<PickerCandidate> {
    let mut items = Vec::new();
    let db_path = dirs::home_dir()
        .unwrap_or_default()
        .join(".gemini/antigravity-cli/conversation_summaries.db");

    if !db_path.exists() {
        return items;
    }

    if let Ok(conn) = Connection::open(&db_path) {
        let sql = match scope {
            ScopeArg::Global => {
                "SELECT conversation_id,
                        COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled'),
                        strftime('%m-%d %H:%M', last_modified_time),
                        COALESCE(NULLIF(project_id, ''), 'main'),
                        step_count
                 FROM conversation_summaries
                 ORDER BY last_modified_time DESC
                 LIMIT ?;"
            }
            ScopeArg::Local => {
                "SELECT conversation_id,
                        COALESCE(NULLIF(title, ''), NULLIF(preview, ''), 'Untitled'),
                        strftime('%m-%d %H:%M', last_modified_time),
                        COALESCE(NULLIF(project_id, ''), 'main'),
                        step_count
                 FROM conversation_summaries
                 WHERE workspace_uris LIKE ? OR project_id = ? OR ? = ''
                 ORDER BY last_modified_time DESC
                 LIMIT ?;"
            }
        };

        if scope == ScopeArg::Global {
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows = stmt.query_map([limit as i64], |row| {
                    Ok(ItemKind::Historical {
                        conversation_id: row.get(0)?,
                        title: row.get(1)?,
                        relative_time: row.get(2)?,
                        project: row.get(3)?,
                        step_count: row.get(4)?,
                    })
                });
                if let Ok(mapped) = rows {
                    for r in mapped.flatten() {
                        items.push(PickerCandidate {
                            display_label: String::new(),
                            match_text: String::new(),
                            kind: r,
                        });
                    }
                }
            }
        } else {
            let filter = format!("%{}%", project);
            if let Ok(mut stmt) = conn.prepare(sql) {
                let rows = stmt.query_map(
                    rusqlite::params![filter, project, project, limit as i64],
                    |row| {
                        Ok(ItemKind::Historical {
                            conversation_id: row.get(0)?,
                            title: row.get(1)?,
                            relative_time: row.get(2)?,
                            project: row.get(3)?,
                            step_count: row.get(4)?,
                        })
                    },
                );
                if let Ok(mapped) = rows {
                    for r in mapped.flatten() {
                        items.push(PickerCandidate {
                            display_label: String::new(),
                            match_text: String::new(),
                            kind: r,
                        });
                    }
                }
            }
        }
    }

    items
}

/// Parse transcript JSONL from ~/.gemini/antigravity-cli/brain/<id>/.system_generated/logs/transcript.jsonl
fn load_transcript(conv_id: &str) -> TranscriptInfo {
    let mut info = TranscriptInfo::default();
    let brain_dir = dirs::home_dir()
        .unwrap_or_default()
        .join(".gemini/antigravity-cli/brain");

    let p1 = brain_dir.join(conv_id).join(".system_generated/logs/transcript.jsonl");
    let p2 = brain_dir.join(conv_id).join(".system_generated/logs/transcript_full.jsonl");

    let path = if p1.exists() {
        p1
    } else if p2.exists() {
        p2
    } else {
        return info;
    };

    if let Ok(file) = File::open(path) {
        let reader = BufReader::new(file);
        for line in reader.lines().flatten() {
            if let Ok(v) = serde_json::from_str::<Value>(&line) {
                let item_type = v.get("type").and_then(|t| t.as_str()).unwrap_or("");
                let source = v.get("source").and_then(|s| s.as_str()).unwrap_or("");

                // Initial prompt
                if info.initial_prompt.is_none() && (item_type == "USER_INPUT" || source == "USER_EXPLICIT") {
                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                        let cleaned = clean_prompt(content);
                        if !cleaned.is_empty() {
                            info.initial_prompt = Some(cleaned);
                        }
                    }
                }

                // Latest turn
                if item_type == "PLANNER_RESPONSE" {
                    info.latest_type = Some("PLANNER".to_string());
                    if let Some(content) = v.get("content").and_then(|c| c.as_str()) {
                        if !content.trim().is_empty() {
                            info.latest_turn = Some(content.to_string());
                        }
                    } else if let Some(thinking) = v.get("thinking").and_then(|th| th.as_str()) {
                        if !thinking.trim().is_empty() {
                            info.latest_turn = Some(thinking.to_string());
                        }
                    }
                }

                // Tokens
                if let Some(inp) = v.get("input_tokens").and_then(|t| t.as_u64()) {
                    info.input_tokens += inp;
                }
                if let Some(out) = v.get("output_tokens").and_then(|t| t.as_u64()) {
                    info.output_tokens += out;
                }
                if let Some(cch) = v.get("cache_read_tokens").and_then(|t| t.as_u64()) {
                    info.cache_tokens += cch;
                }

                // Tool calls
                if let Some(calls) = v.get("tool_calls").and_then(|c| c.as_array()) {
                    for call in calls {
                        info.tools_count += 1;
                        let t_name = call
                            .get("name")
                            .or_else(|| call.get("toolAction"))
                            .and_then(|n| n.as_str())
                            .unwrap_or("tool");
                        *info.tools_breakdown.entry(t_name.to_string()).or_insert(0) += 1;
                    }
                }
            }
        }
    }

    info
}

fn clean_prompt(raw: &str) -> String {
    let mut s = raw.to_string();
    if let Some(start) = s.find("<USER_REQUEST>") {
        s = s[start + "<USER_REQUEST>".len()..].to_string();
    }
    if let Some(end) = s.find("</USER_REQUEST>") {
        s = s[..end].to_string();
    }
    s.trim().to_string()
}

/// Execute selection outside or inside tmux
pub fn execute_selection(
    candidate: &PickerCandidate,
    project: &str,
    workdir: &std::path::Path,
) -> anyhow::Result<()> {
    match &candidate.kind {
        ItemKind::NewTab => {
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args([
                        "new-window",
                        "-c",
                        &workdir.display().to_string(),
                        "-n",
                        "+ new",
                        &format!("agymux run --project '{}'", project),
                    ])
                    .status();
            } else {
                let _ = Command::new("agy")
                    .args(["--project", project, "--remote-control"])
                    .status();
            }
        }
        ItemKind::OpenTab {
            session,
            window_index,
            ..
        } => {
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args(["switch-client", "-t", &format!("{}:{}", session, window_index)])
                    .status();
                let _ = Command::new("tmux")
                    .args(["select-window", "-t", &format!("{}:{}", session, window_index)])
                    .status();
            }
        }
        ItemKind::Historical {
            conversation_id,
            project: target_project,
            ..
        } => {
            let effective_project = if target_project.is_empty() || target_project == "main" {
                project
            } else {
                target_project
            };
            if std::env::var("TMUX").is_ok() {
                let _ = Command::new("tmux")
                    .args([
                        "new-window",
                        "-c",
                        &workdir.display().to_string(),
                        "-n",
                        "conv",
                        &format!(
                            "agymux run --project '{}' --conversation '{}'",
                            effective_project, conversation_id
                        ),
                    ])
                    .status();
            } else {
                let _ = Command::new("agy")
                    .args(["--project", effective_project, "-c", conversation_id])
                    .status();
            }
        }
    }
    Ok(())
}

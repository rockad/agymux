use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Clear, Paragraph, Wrap},
    Frame,
};

use crate::ui::theme::{
    base_style, styled_block, COLOR_CYAN, COLOR_FG, COLOR_GREEN, COLOR_MUTED, COLOR_PINK,
    COLOR_PURPLE, COLOR_YELLOW,
};

#[derive(Default)]
pub struct HelperState {
    pub scroll: u16,
}

impl HelperState {
    pub fn new() -> Self {
        Self { scroll: 0 }
    }

    pub fn scroll_up(&mut self) {
        if self.scroll > 0 {
            self.scroll -= 1;
        }
    }

    pub fn scroll_down(&mut self) {
        self.scroll = self.scroll.saturating_add(1);
    }
}

/// Returns true if the cheatsheet modal should close
pub fn handle_key(key: KeyEvent, state: &mut HelperState) -> bool {
    match key.code {
        KeyCode::Esc
        | KeyCode::Enter
        | KeyCode::Char('q')
        | KeyCode::Char('й')
        | KeyCode::Char(' ') => true,
        KeyCode::Up | KeyCode::Char('k') | KeyCode::Char('л') => {
            state.scroll_up();
            false
        }
        KeyCode::Down | KeyCode::Char('j') | KeyCode::Char('о') => {
            state.scroll_down();
            false
        }
        _ => false,
    }
}

pub fn render(f: &mut Frame, area: Rect, state: &HelperState) {
    let modal_width = 78.min(area.width.saturating_sub(4));
    let modal_height = 32.min(area.height.saturating_sub(2));

    let horiz_pad = (area.width.saturating_sub(modal_width)) / 2;
    let vert_pad = (area.height.saturating_sub(modal_height)) / 2;

    let modal_area = Rect {
        x: horiz_pad,
        y: vert_pad,
        width: modal_width,
        height: modal_height,
    };

    f.render_widget(Clear, modal_area);

    let mut text_lines = Vec::new();

    // Title line
    text_lines.push(Line::from(vec![
        Span::styled(
            "agymux v0.2 — Keyboard Shortcuts & Commands",
            Style::default()
                .fg(COLOR_PURPLE)
                .add_modifier(Modifier::BOLD),
        ),
    ]));
    text_lines.push(Line::raw(""));

    // Section 1: Navigation
    text_lines.push(Line::from(vec![
        Span::styled("▌ TAB NAVIGATION & MOVEMENT", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
    ]));
    text_lines.push(shortcut_line("Alt + 1..9", "Jump directly to tab 1..9 (layout-independent)"));
    text_lines.push(shortcut_line("Alt + Left / Right", "Cycle to previous / next tab"));
    text_lines.push(shortcut_line("Ctrl+Space s  /  ы", "Two-pane tab & conversation switcher"));
    text_lines.push(Line::raw(""));

    // Section 2: Conversation & Worktree Lifecycle
    text_lines.push(Line::from(vec![
        Span::styled("▌ CONVERSATION & WORKTREE LIFECYCLE", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
    ]));
    text_lines.push(shortcut_line("Ctrl+Space c  /  с", "New conversation tab in current workspace"));
    text_lines.push(shortcut_line("Ctrl+Space w  /  ц", "New git worktree tab (.worktrees/<branch>)"));
    text_lines.push(shortcut_line("Ctrl+Space r  /  к", "Rename active tab"));
    text_lines.push(shortcut_line("Ctrl+Space f  /  а", "Fork current conversation into new tab"));
    text_lines.push(shortcut_line("Ctrl+Space l  /  д", "Launch new tab with custom model / effort flags"));
    text_lines.push(shortcut_line("Ctrl+Space x  /  ч", "Close active tab (with confirmation)"));
    text_lines.push(shortcut_line("Ctrl+Space d  /  в", "Detach tmux session (leaves agents running)"));
    text_lines.push(Line::raw(""));

    // Section 3: Modals & Global Controls
    text_lines.push(Line::from(vec![
        Span::styled("▌ MODALS & GLOBAL CONTROLS", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
    ]));
    text_lines.push(shortcut_line("Ctrl+Space p  /  з", "Quick Command Palette (all actions menu)"));
    text_lines.push(shortcut_line("Ctrl+Space P  /  З", "Global Project Switcher"));
    text_lines.push(shortcut_line("Ctrl+Space ?  /  h  /  р", "Display this shortcuts cheatsheet"));
    text_lines.push(Line::raw(""));

    // Section 4: Inside Picker
    text_lines.push(Line::from(vec![
        Span::styled("▌ INSIDE TWO-PANE PICKER", Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)),
    ]));
    text_lines.push(shortcut_line_yellow("Tab  or  Ctrl+A", "Toggle search scope (Local Project ⇄ Global Machine)"));
    text_lines.push(shortcut_line_yellow("Enter", "Select tab / resume historical conversation"));
    text_lines.push(shortcut_line_yellow("Esc", "Cancel / close modal"));
    text_lines.push(Line::raw(""));

    // Footer
    text_lines.push(Line::from(vec![
        Span::styled(
            "─".repeat(modal_width.saturating_sub(4) as usize),
            Style::default().fg(COLOR_MUTED),
        ),
    ]));
    text_lines.push(Line::from(vec![
        Span::styled(
            "All letter shortcuts feature paired English & Cyrillic (RussianWin) keys.",
            Style::default().fg(COLOR_MUTED),
        ),
    ]));
    text_lines.push(Line::from(vec![
        Span::styled(
            "Press Esc, q, or Enter to close modal.",
            Style::default().fg(COLOR_PINK).add_modifier(Modifier::BOLD),
        ),
    ]));

    let block = styled_block("Cheatsheet (Esc/Enter to exit)", true);
    let paragraph = Paragraph::new(text_lines)
        .block(block)
        .style(base_style())
        .wrap(Wrap { trim: false })
        .scroll((state.scroll, 0));

    f.render_widget(paragraph, modal_area);
}

fn shortcut_line(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<26}", key),
            Style::default().fg(COLOR_GREEN).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(COLOR_FG)),
    ])
}

fn shortcut_line_yellow(key: &str, desc: &str) -> Line<'static> {
    Line::from(vec![
        Span::styled(
            format!("  {:<26}", key),
            Style::default().fg(COLOR_YELLOW).add_modifier(Modifier::BOLD),
        ),
        Span::styled(desc.to_string(), Style::default().fg(COLOR_FG)),
    ])
}

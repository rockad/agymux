use ratatui::{
    layout::Alignment,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders},
};

/// Standard Terminal ANSI Color Tokens (inherits terminal emulator colors by default)
pub const COLOR_BG: Color = Color::Reset; // Inherit terminal background
pub const COLOR_BG_ALT: Color = Color::Reset; // Inherit terminal background
pub const COLOR_FG: Color = Color::Reset; // Primary text foreground
pub const COLOR_MUTED: Color = Color::DarkGray; // Dimmed metadata
pub const COLOR_GREEN: Color = Color::Green; // Active tab pill (green)
pub const COLOR_CYAN: Color = Color::Cyan; // Alternative active / highlight (cyan)
pub const COLOR_PINK: Color = Color::Magenta; // Prefix indicator / alert (pink)
pub const COLOR_ORANGE: Color = Color::LightRed; // Running turn / agent working (orange)
pub const COLOR_YELLOW: Color = Color::Yellow; // Yellow warning / match highlight
pub const COLOR_PURPLE: Color = Color::Magenta; // Purple accent / project badges

pub const PL_LEFT: &str = "";
pub const PL_RIGHT: &str = "";

/// Returns base application background and foreground style (inherits terminal colors)
pub fn base_style() -> Style {
    Style::default()
}

/// Returns style for inactive/dimmed text
pub fn muted_style() -> Style {
    Style::default().fg(COLOR_MUTED)
}

/// Returns header style
pub fn header_style() -> Style {
    Style::default()
        .fg(COLOR_PURPLE)
        .add_modifier(Modifier::BOLD)
}

/// Returns border style based on active state
pub fn active_border_style(is_active: bool) -> Style {
    if is_active {
        Style::default()
            .fg(COLOR_GREEN)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_MUTED)
    }
}

/// Returns highlighted row style for selections
pub fn highlight_style() -> Style {
    Style::default()
        .bg(COLOR_BG_ALT)
        .fg(COLOR_GREEN)
        .add_modifier(Modifier::BOLD)
}

/// Returns a styled block with rounded borders and optional active border highlighting
pub fn styled_block<'a>(title: &'a str, is_active: bool) -> Block<'a> {
    let border_color = if is_active { COLOR_GREEN } else { COLOR_MUTED };
    let title_style = if is_active {
        Style::default().fg(COLOR_CYAN).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(COLOR_FG)
    };

    Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border_color))
        .title(Span::styled(format!(" {} ", title), title_style))
        .title_alignment(Alignment::Left)
        .style(base_style())
}

/// Returns a styled pill Line with rounded powerline delimiters
pub fn pill<'a>(text: &'a str, pill_color: Color, text_color: Color) -> Line<'a> {
    Line::from(pill_spans(text, pill_color, text_color))
}

/// Returns vector of Spans representing a pill
pub fn pill_spans<'a>(text: &'a str, pill_color: Color, text_color: Color) -> Vec<Span<'a>> {
    vec![
        Span::styled(PL_LEFT, Style::default().fg(pill_color)),
        Span::styled(
            format!(" {} ", text),
            Style::default()
                .bg(pill_color)
                .fg(text_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(PL_RIGHT, Style::default().fg(pill_color)),
    ]
}

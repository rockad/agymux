use agymux::cli::ScopeArg;
use agymux::ui::helper::{self, HelperState};
use agymux::ui::palette::{self, PaletteAction, PaletteState};
use agymux::ui::picker::{self, PickerState};
use agymux::ui::theme::{self, COLOR_BG, COLOR_GREEN, PL_LEFT, PL_RIGHT};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::backend::TestBackend;
use ratatui::Terminal;
use std::path::PathBuf;

#[test]
fn test_theme_tokens_and_pills() {
    let p = theme::pill("active", COLOR_GREEN, COLOR_BG);
    assert_eq!(p.spans.len(), 3);
    assert_eq!(p.spans[0].content, PL_LEFT);
    assert_eq!(p.spans[1].content, " active ");
    assert_eq!(p.spans[2].content, PL_RIGHT);

    let block = theme::styled_block("Test", true);
    // Block is created without panics
    let _ = block;
}

#[test]
fn test_helper_modal_render_and_scroll() {
    let backend = TestBackend::new(80, 30);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = HelperState::new();

    terminal
        .draw(|f| helper::render(f, f.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();

    // Verify key sections and bilingual bindings are rendered
    assert!(rendered_text.contains("Keyboard Shortcuts"));
    assert!(rendered_text.contains("TAB NAVIGATION & MOVEMENT"));
    assert!(rendered_text.contains("CONVERSATION & WORKTREE LIFECYCLE"));
    assert!(rendered_text.contains("Ctrl+Space 1..9"));
    assert!(rendered_text.contains("Ctrl+Space c  /  с"));

    // Test scrolling logic
    assert_eq!(state.scroll, 0);
    let down_key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
    assert!(!helper::handle_key(down_key, &mut state));
    assert_eq!(state.scroll, 1);

    let up_key = KeyEvent::new(KeyCode::Up, KeyModifiers::NONE);
    assert!(!helper::handle_key(up_key, &mut state));
    assert_eq!(state.scroll, 0);

    // Esc closes helper
    let esc_key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
    assert!(helper::handle_key(esc_key, &mut state));
}

#[test]
fn test_palette_modal_render_and_fuzzy_filter() {
    let backend = TestBackend::new(70, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = PaletteState::new();

    assert_eq!(state.items.len(), 10);

    terminal
        .draw(|f| palette::render(f, f.area(), &state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text.contains("Command Palette"));
    assert!(rendered_text.contains("Action >"));

    // Test typing and filtering
    let char_r = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
    let char_e = KeyEvent::new(KeyCode::Char('e'), KeyModifiers::NONE);
    let char_n = KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE);
    palette::handle_key(char_r, &mut state);
    palette::handle_key(char_e, &mut state);
    palette::handle_key(char_n, &mut state);

    assert_eq!(state.query, "ren");
    assert!(!state.filtered_indices.is_empty());
    assert_eq!(state.selected_action(), Some(PaletteAction::RenameTab));
}

#[test]
fn test_picker_modal_render_and_scope_toggle() {
    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    let mut state = PickerState::new(ScopeArg::Local, "agymux".to_string(), PathBuf::from("/tmp"));

    terminal
        .draw(|f| picker::render(f, f.area(), &mut state))
        .unwrap();

    let buffer = terminal.backend().buffer();
    let rendered_text: String = buffer.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text.contains("Switcher"));
    assert!(rendered_text.contains("Tabs & Conversations"));
    assert!(rendered_text.contains("[Local]"));

    // Toggle scope to Global
    state.toggle_scope();
    assert_eq!(state.scope, ScopeArg::Global);

    terminal
        .draw(|f| picker::render(f, f.area(), &mut state))
        .unwrap();

    let buffer2 = terminal.backend().buffer();
    let rendered_text2: String = buffer2.content().iter().map(|c| c.symbol()).collect();
    assert!(rendered_text2.contains("[Global Search]"));
}

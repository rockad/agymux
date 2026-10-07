use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{self, Event, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    io,
    panic,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::cli::ScopeArg;
use crate::ui::{
    helper::{self, HelperState},
    palette::{self, PaletteResult, PaletteState},
    picker::{self, PickerResult, PickerState},
    project_picker::{self, ProjectPickerResult, ProjectPickerState},
};

/// RAII Terminal Guard ensuring clean terminal restoration on drop or panic
pub struct TerminalGuard {
    active: bool,
}

impl TerminalGuard {
    pub fn enter() -> Result<Self> {
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;

        // Set panic hook to restore terminal if a panic occurs
        let default_hook = panic::take_hook();
        panic::set_hook(Box::new(move |panic_info| {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
            default_hook(panic_info);
        }));

        Ok(Self { active: true })
    }

    pub fn leave(&mut self) {
        if self.active {
            let _ = disable_raw_mode();
            let _ = execute!(io::stdout(), LeaveAlternateScreen, Show);
            self.active = false;
        }
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        self.leave();
    }
}

/// Run two-pane tab & conversation picker
pub fn run_picker(scope: ScopeArg, project: String, workdir: PathBuf) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = PickerState::new(scope, project.clone(), workdir.clone());

    loop {
        terminal.draw(|f| {
            let area = f.area();
            picker::render(f, area, &mut state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match picker::handle_key(key, &mut state) {
                    PickerResult::Continue => {}
                    PickerResult::Select(candidate) => {
                        guard.leave();
                        picker::execute_selection(&candidate, &project, &workdir)?;
                        break;
                    }
                    PickerResult::Cancel => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run global project switcher modal
pub fn run_project_picker(base_dir: &Path) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = ProjectPickerState::new(base_dir);

    loop {
        terminal.draw(|f| {
            let area = f.area();
            project_picker::render(f, area, &state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match project_picker::handle_key(key, &mut state) {
                    ProjectPickerResult::Continue => {}
                    ProjectPickerResult::Select(project) => {
                        guard.leave();
                        project_picker::execute_project_switch(&project)?;
                        break;
                    }
                    ProjectPickerResult::Cancel => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run command palette modal
pub fn run_palette(project: Option<String>) -> Result<()> {
    let mut guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = PaletteState::new();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            palette::render(f, area, &state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                match palette::handle_key(key, &mut state) {
                    PaletteResult::Continue => {}
                    PaletteResult::Execute(action) => {
                        guard.leave();
                        palette::execute_action(action, project.as_deref())?;
                        break;
                    }
                    PaletteResult::Cancel => {
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}

/// Run keyboard shortcuts helper cheatsheet modal
pub fn run_helper() -> Result<()> {
    let _guard = TerminalGuard::enter()?;
    let backend = CrosstermBackend::new(io::stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut state = HelperState::new();

    loop {
        terminal.draw(|f| {
            let area = f.area();
            helper::render(f, area, &state);
        })?;

        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if helper::handle_key(key, &mut state) {
                    break;
                }
            }
        }
    }

    Ok(())
}

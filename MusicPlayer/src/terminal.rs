use std::io::{self, Stdout};
use anyhow::Result;
use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub type Tui = Terminal<CrosstermBackend<Stdout>>;

/// Initialize terminal in raw mode and alternate screen
pub fn init() -> Result<Tui> {
    enable_raw_mode()?;
    let mut standard_output = io::stdout();
    execute!(standard_output, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(standard_output);
    let terminal = Terminal::new(backend)?;
    Ok(terminal)
}

/// Restore terminal to its original state
pub fn restore(mut terminal: Tui) -> Result<()> {
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;
    Ok(())
}

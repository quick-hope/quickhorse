//! Terminal management for TUI

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal as RatatuiTerminal};
use std::io::{self, Stdout};

/// Terminal backend type
pub type Backend = CrosstermBackend<Stdout>;

/// Terminal type
pub type Terminal = RatatuiTerminal<Backend>;

/// Initialize the terminal
pub fn init() -> io::Result<Terminal> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = Backend::new(stdout);
    RatatuiTerminal::new(backend)
}

/// Restore the terminal
pub fn restore() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)
}
//! Terminal management for TUI (main screen mode - preserves content after exit)

use crossterm::{
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use ratatui::{backend::CrosstermBackend, Terminal as RatatuiTerminal};
use std::io::{self, Stdout};

/// Terminal backend type
pub type Backend = CrosstermBackend<Stdout>;

/// Terminal type
pub type Terminal = RatatuiTerminal<Backend>;

/// Initialize the terminal (main screen - content preserved after exit)
pub fn init() -> io::Result<Terminal> {
    enable_raw_mode()?;
    // No EnterAlternateScreen - use main screen so content is preserved
    let stdout = io::stdout();
    let backend = Backend::new(stdout);
    RatatuiTerminal::new(backend)
}

/// Restore the terminal (just disable raw mode - content stays in history)
pub fn restore() -> io::Result<()> {
    // No LeaveAlternateScreen - content preserved in terminal history
    disable_raw_mode()
}
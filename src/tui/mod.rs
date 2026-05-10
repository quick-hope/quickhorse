//! TUI module - Terminal User Interface using ratatui

mod app;
mod event;
mod terminal;
mod ui;

pub use app::App;
pub use event::{Event, EventHandler};
pub use terminal::{init as init_terminal, restore as restore_terminal};
pub use ui::render;

// Re-export TextEditor for testing purposes
#[allow(unused_imports)]
pub use app::TextEditor;
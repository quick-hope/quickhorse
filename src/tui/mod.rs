//! TUI module - Terminal User Interface using ratatui

mod app;
mod completion;
mod cursor;
mod event;
mod permission_dialog;
mod progress;
mod terminal;
mod ui;

pub use app::{App, PermissionChoiceResult};
pub use completion::{CommandCompleter, CompletionProvider, CompletionState, CompletionType, PathCompleter, PathEntry, PathEntryType, Suggestion};
pub use cursor::{Cursor, CursorPosition, DoublePressDetector, KillDirection, KillRing, MeasuredText, SelectionDirection};
pub use event::{Event, EventHandler};
pub use permission_dialog::{PermissionChoice, PermissionDialog, PermissionRequestWidget};
pub use progress::{ProgressBar, ProgressManager, Spinner, ToolProgress, ToolStatus};
pub use terminal::{init as init_terminal, restore as restore_terminal};
pub use ui::render;

// Re-export TextEditor for testing purposes
#[allow(unused_imports)]
pub use app::TextEditor;
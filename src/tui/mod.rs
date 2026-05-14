//! TUI module - Terminal User Interface using ratatui

mod app;
mod completion;
mod cursor;
mod event;
mod history;
mod history_cell;
mod permission_dialog;
mod progress;
mod scroll_state;
mod stream_state;
mod terminal;
mod transcript_cache;
mod ui;
mod widgets;

pub use app::{App, PermissionChoiceResult};
pub use completion::{CommandCompleter, CompletionProvider, CompletionState, CompletionType, PathCompleter, PathEntry, PathEntryType, Suggestion};
pub use cursor::{Cursor, CursorPosition, DoublePressDetector, KillDirection, KillRing, MeasuredText, SelectionDirection};
pub use event::{Event, EventHandler};
pub use history::{CommandHistory, HistoryEntry, HistoryStats};
pub use history_cell::{HistoryCell, ToolCell, USER_GLYPH, ASSISTANT_GLYPH, TRANSCRIPT_RAIL, REASONING_RAIL};
pub use permission_dialog::{PermissionChoice, PermissionDialog, PermissionRequestWidget};
pub use progress::{ProgressBar, ProgressManager, Spinner, ToolProgress, ToolStatus};
pub use scroll_state::{TranscriptScroll, ScrollDirection};
pub use stream_state::{StreamingState, ChunkingMode, AdaptiveChunkingPolicy, StreamChunker};
pub use terminal::{init as init_terminal, restore as restore_terminal};
pub use transcript_cache::{TranscriptViewCache, CachedCell};
pub use ui::render;
pub use widgets::{ChatWidget, ChatTheme, HeaderWidget, HeaderData, FooterWidget, FooterProps, ToolCardWidget, Renderable};

// Re-export TextEditor for testing purposes
#[allow(unused_imports)]
pub use app::TextEditor;
//! Widget system for TUI rendering.
//!
//! Widgets are pure render components that take pre-computed data
//! and render to a buffer. They do not own App state.

mod chat;
mod header;
mod footer;
mod tool_card;

pub use chat::{ChatWidget, ChatTheme};
pub use header::{HeaderWidget, HeaderData};
pub use footer::{FooterWidget, FooterProps};
pub use tool_card::{ToolCardWidget, ToolCardTheme, ToolStatusKind};

/// Trait for widgets that can render to a buffer.
pub trait Renderable {
    fn render(&self, area: ratatui::layout::Rect, buf: &mut ratatui::buffer::Buffer);
}
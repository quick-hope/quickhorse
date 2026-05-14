//! Transcript history cell types for TUI rendering.
//!
//! Each cell type has dedicated rendering with visual markers:
//! - User: `▎` (solid 1/4 block)
//! - Assistant: `●` (solid bullet)
//! - Thinking: `╎` (dashed rail)
//! - Tool: Compact card with expand affordance

use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use unicode_width::UnicodeWidthStr;

/// Visual marker for the user role at the start of their message line.
pub const USER_GLYPH: &str = "\u{258E}"; // ▎

/// Visual marker for the assistant role.
pub const ASSISTANT_GLYPH: &str = "\u{25CF}"; // ●

/// Transcript body left rail. Solid 1/8 block followed by a space.
pub const TRANSCRIPT_RAIL: &str = "\u{258F} "; // ▏ + space

/// Reasoning header opener. Used for thinking cells.
pub const REASONING_OPENER: &str = "\u{2026}"; // …

/// Reasoning body left rail. Dashed to visually separate from message body.
pub const REASONING_RAIL: &str = "\u{254E} "; // ╎ + space

/// Renderable history cell for user/assistant/system/thinking/tool entries.
#[derive(Debug, Clone)]
pub enum HistoryCell {
    /// User message.
    User {
        content: String,
    },
    /// Assistant response.
    Assistant {
        content: String,
        streaming: bool,
    },
    /// System message (usually hidden or special format).
    System {
        content: String,
    },
    /// Thinking/reasoning content (collapsed in live view).
    Thinking {
        content: String,
        streaming: bool,
        duration_secs: Option<f32>,
    },
    /// Tool execution card.
    Tool(ToolCell),
    /// Error message.
    Error {
        message: String,
    },
}

/// Tool execution cell for transcript.
#[derive(Debug, Clone)]
pub struct ToolCell {
    /// Tool name.
    pub name: String,
    /// Tool call ID.
    pub id: String,
    /// Tool input parameters (for display).
    pub input_summary: String,
    /// Tool output (truncated for display).
    pub output: String,
    /// Whether tool execution is complete.
    pub is_complete: bool,
    /// Whether tool execution failed.
    pub is_error: bool,
    /// Duration in seconds.
    pub duration_secs: Option<f32>,
}

impl HistoryCell {
    /// Render the cell into terminal lines.
    ///
    /// Width is used for text wrapping. Tool output is capped.
    pub fn lines(&self, width: u16) -> Vec<Line<'static>> {
        match self {
            HistoryCell::User { content } => {
                render_user_message(content, width)
            }
            HistoryCell::Assistant { content, streaming } => {
                render_assistant_message(content, *streaming, width)
            }
            HistoryCell::System { content } => {
                render_system_message(content, width)
            }
            HistoryCell::Thinking { content, streaming, duration_secs } => {
                render_thinking(content, *streaming, *duration_secs, width)
            }
            HistoryCell::Tool(tool) => {
                tool.render_lines(width)
            }
            HistoryCell::Error { message } => {
                render_error(message, width)
            }
        }
    }

    /// Check if this cell is streaming (for revision tracking).
    pub fn is_streaming(&self) -> bool {
        match self {
            HistoryCell::Assistant { streaming, .. } => *streaming,
            HistoryCell::Thinking { streaming, .. } => *streaming,
            HistoryCell::Tool(tool) => !tool.is_complete,
            _ => false,
        }
    }

    /// Check if this is a conversational cell (User/Assistant/Thinking).
    pub fn is_conversational(&self) -> bool {
        matches!(
            self,
            HistoryCell::User { .. }
                | HistoryCell::Assistant { .. }
                | HistoryCell::Thinking { .. }
        )
    }

    /// Check if this is empty (should be skipped in rendering).
    pub fn is_empty(&self) -> bool {
        match self {
            HistoryCell::User { content } => content.is_empty(),
            HistoryCell::Assistant { content, .. } => content.is_empty(),
            HistoryCell::System { content } => content.is_empty(),
            HistoryCell::Thinking { content, .. } => content.is_empty(),
            HistoryCell::Tool(tool) => tool.name.is_empty(),
            HistoryCell::Error { message } => message.is_empty(),
        }
    }
}

impl ToolCell {
    /// Render tool card lines.
    pub fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Header: glyph + name + status
        let status_glyph = if self.is_error {
            "❌"
        } else if self.is_complete {
            "✓"
        } else {
            "⋯" // Spinner placeholder
        };

        let duration_str = self.duration_secs
            .map(|d| format!(" {:.1}s", d))
            .unwrap_or_default();

        let header = format!(
            "{} {} {}{}",
            TRANSCRIPT_RAIL.trim(),
            status_glyph,
            self.name,
            duration_str
        );

        let header_style = if self.is_error {
            Style::default().fg(Color::Red)
        } else if self.is_complete {
            Style::default().fg(Color::Green)
        } else {
            Style::default().fg(Color::Yellow)
        };

        lines.push(Line::from(Span::styled(header, header_style)));

        // Input summary (if non-empty and fits)
        if !self.input_summary.is_empty() {
            let input_display = truncate_to_width(&self.input_summary, width.saturating_sub(4) as usize);
            lines.push(Line::from(Span::styled(
                format!("{} {}", TRANSCRIPT_RAIL, input_display),
                Style::default().fg(Color::DarkGray),
            )));
        }

        // Output (truncated)
        if !self.output.is_empty() {
            let output_display = truncate_to_width(&self.output, width.saturating_sub(4) as usize);
            let output_style = if self.is_error {
                Style::default().fg(Color::Red)
            } else {
                Style::default().fg(Color::Gray)
            };

            for line in output_display.lines().take(3) {
                lines.push(Line::from(Span::styled(
                    format!("{} {}", TRANSCRIPT_RAIL, line),
                    output_style,
                )));
            }
        }

        lines
    }
}

// === Helper functions ===

fn render_user_message(content: &str, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let wrap_width = width.saturating_sub(2) as usize; // Account for glyph

    let user_style = Style::default().fg(Color::Cyan);

    for line in wrap_text(content, wrap_width) {
        lines.push(Line::from(vec![
            Span::styled(USER_GLYPH, user_style.add_modifier(Modifier::BOLD)),
            Span::raw(" "),
            Span::styled(line, user_style),
        ]));
    }

    lines
}

fn render_assistant_message(content: &str, streaming: bool, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let wrap_width = width.saturating_sub(2) as usize;

    let base_style = Style::default().fg(Color::Green);
    let glyph_style = if streaming {
        base_style.add_modifier(Modifier::BOLD)
    } else {
        base_style
    };

    for line in wrap_text(content, wrap_width) {
        lines.push(Line::from(vec![
            Span::styled(ASSISTANT_GLYPH, glyph_style),
            Span::raw(" "),
            Span::styled(line, base_style),
        ]));
    }

    lines
}

fn render_system_message(content: &str, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let wrap_width = width.saturating_sub(2) as usize;

    let system_style = Style::default().fg(Color::Yellow);

    for line in wrap_text(content, wrap_width) {
        lines.push(Line::from(vec![
            Span::styled(TRANSCRIPT_RAIL.trim(), system_style),
            Span::styled(line, system_style),
        ]));
    }

    lines
}

fn render_thinking(content: &str, streaming: bool, duration_secs: Option<f32>, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let wrap_width = width.saturating_sub(2) as usize;

    let thinking_style = Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM | Modifier::ITALIC);

    // Header with duration
    let duration_str = duration_secs
        .map(|d| format!(" {:.1}s", d))
        .unwrap_or_default();

    let header = if streaming {
        format!("{} Thinking{}", REASONING_OPENER, duration_str)
    } else {
        format!("{} Thought{}", REASONING_OPENER, duration_str)
    };

    lines.push(Line::from(Span::styled(header, thinking_style)));

    // Content (collapsed to first few lines in live view)
    for line in wrap_text(content, wrap_width).into_iter().take(4) {
        lines.push(Line::from(vec![
            Span::styled(REASONING_RAIL.trim(), thinking_style),
            Span::styled(line, thinking_style),
        ]));
    }

    lines
}

fn render_error(message: &str, width: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    let wrap_width = width.saturating_sub(2) as usize;

    let error_style = Style::default().fg(Color::Red);

    lines.push(Line::from(Span::styled("❌ Error", error_style.add_modifier(Modifier::BOLD))));

    for line in wrap_text(message, wrap_width) {
        lines.push(Line::from(vec![
            Span::styled(TRANSCRIPT_RAIL.trim(), error_style),
            Span::styled(line, error_style),
        ]));
    }

    lines
}

/// Wrap text to fit within given width.
fn wrap_text(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.lines().map(|s| s.to_string()).collect();
    }

    let mut result = Vec::new();

    for line in text.lines() {
        if line.width() <= width {
            result.push(line.to_string());
        } else {
            // Break into multiple lines
            let mut current = String::new();
            let mut current_width = 0;

            for word in line.split_whitespace() {
                let word_width = word.width();

                if current_width == 0 {
                    current = word.to_string();
                    current_width = word_width;
                } else if current_width + 1 + word_width <= width {
                    current.push(' ');
                    current.push_str(word);
                    current_width += 1 + word_width;
                } else {
                    result.push(current);
                    current = word.to_string();
                    current_width = word_width;
                }
            }

            if !current.is_empty() {
                result.push(current);
            }
        }
    }

    result
}

/// Truncate text to fit within given display width.
fn truncate_to_width(text: &str, max_width: usize) -> String {
    if text.width() <= max_width {
        return text.to_string();
    }

    // Truncate by graphemes to respect display width
    let mut result = String::new();
    let mut width = 0;

    for g in unicode_segmentation::UnicodeSegmentation::graphemes(text, true) {
        let g_width = g.width();
        if width + g_width > max_width.saturating_sub(3) {
            result.push_str("...");
            break;
        }
        result.push_str(g);
        width += g_width;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_cell_lines() {
        let cell = HistoryCell::User { content: "Hello World".to_string() };
        let lines = cell.lines(80);

        assert!(!lines.is_empty());
        assert!(lines[0].spans[0].content.contains(USER_GLYPH));
    }

    #[test]
    fn test_assistant_cell_lines() {
        let cell = HistoryCell::Assistant {
            content: "Response text".to_string(),
            streaming: false,
        };
        let lines = cell.lines(80);

        assert!(!lines.is_empty());
        assert!(lines[0].spans[0].content.contains(ASSISTANT_GLYPH));
    }

    #[test]
    fn test_assistant_streaming() {
        let cell = HistoryCell::Assistant {
            content: "Streaming...".to_string(),
            streaming: true,
        };

        assert!(cell.is_streaming());
    }

    #[test]
    fn test_thinking_cell() {
        let cell = HistoryCell::Thinking {
            content: "Reasoning process".to_string(),
            streaming: true,
            duration_secs: Some(2.5),
        };

        assert!(cell.is_streaming());
        assert!(cell.is_conversational());

        let lines = cell.lines(80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_tool_cell_complete() {
        let tool = ToolCell {
            name: "BashTool".to_string(),
            id: "tool_123".to_string(),
            input_summary: "ls -la".to_string(),
            output: "file1 file2".to_string(),
            is_complete: true,
            is_error: false,
            duration_secs: Some(0.5),
        };

        let lines = tool.render_lines(80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_tool_cell_error() {
        let tool = ToolCell {
            name: "BashTool".to_string(),
            id: "tool_456".to_string(),
            input_summary: "rm -rf".to_string(),
            output: "Permission denied".to_string(),
            is_complete: true,
            is_error: true,
            duration_secs: None,
        };

        let lines = tool.render_lines(80);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_empty_cell() {
        let cell = HistoryCell::User { content: "".to_string() };
        assert!(cell.is_empty());
    }

    #[test]
    fn test_wrap_text() {
        let text = "This is a very long line that should be wrapped";
        let wrapped = wrap_text(text, 20);

        assert!(wrapped.len() > 1);
        for line in &wrapped {
            assert!(line.width() <= 20);
        }
    }

    #[test]
    fn test_truncate_to_width() {
        let text = "This is a very long string";
        let truncated = truncate_to_width(text, 10);

        assert!(truncated.width() <= 10);
        assert!(truncated.ends_with("..."));
    }

    #[test]
    fn test_conversational_check() {
        let user = HistoryCell::User { content: "test".to_string() };
        let assistant = HistoryCell::Assistant { content: "test".to_string(), streaming: false };
        let tool = HistoryCell::Tool(ToolCell {
            name: "test".to_string(),
            id: "1".to_string(),
            input_summary: "".to_string(),
            output: "".to_string(),
            is_complete: true,
            is_error: false,
            duration_secs: None,
        });

        assert!(user.is_conversational());
        assert!(assistant.is_conversational());
        assert!(!tool.is_conversational());
    }
}
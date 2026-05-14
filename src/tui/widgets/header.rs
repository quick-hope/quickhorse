//! HeaderWidget - Top bar displaying model, workspace, and status.
//!
//! Shows:
//! - Model name and provider
//! - Workspace/directory context
//! - Status indicator (streaming animation)
//! - Context utilization bar

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

/// HeaderWidget displays model and workspace info.
pub struct HeaderWidget {
    /// Data to render.
    data: HeaderData,
    /// Background color.
    background: Color,
    /// Border color.
    border: Color,
}

/// Data required to render the header.
#[derive(Debug, Clone)]
pub struct HeaderData {
    /// Model name (e.g., "gpt-4", "claude-3").
    pub model: String,
    /// Provider name (e.g., "openai", "anthropic").
    pub provider: String,
    /// Workspace/directory name.
    pub workspace: String,
    /// Whether streaming is active.
    pub is_streaming: bool,
    /// Status indicator frame (animated glyph).
    pub status_frame: Option<String>,
}

impl HeaderWidget {
    /// Create a new header widget.
    pub fn new(data: HeaderData) -> Self {
        Self {
            data,
            background: Color::Reset,
            border: Color::DarkGray,
        }
    }

    /// Set theme colors.
    pub fn with_colors(mut self, background: Color, border: Color) -> Self {
        self.background = background;
        self.border = border;
        self
    }
}

impl Widget for HeaderWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        // Build header line
        let mut spans = Vec::new();

        // Status indicator (if streaming)
        if let Some(frame) = &self.data.status_frame {
            spans.push(Span::styled(
                frame.clone(),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
            spans.push(Span::raw(" "));
        }

        // Provider chip
        spans.push(Span::styled(
            format!("[{}]", self.data.provider),
            Style::default().fg(Color::Magenta),
        ));
        spans.push(Span::raw(" "));

        // Model name
        spans.push(Span::styled(
            self.data.model.clone(),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));

        // Separator
        spans.push(Span::styled(
            "│",
            Style::default().fg(self.border),
        ));
        spans.push(Span::raw(" "));

        // Workspace (truncate if needed)
        let remaining_width = area.width as usize - spans.iter().map(|s| s.content.width()).sum::<usize>() - 1;
        let workspace_display = if self.data.workspace.width() > remaining_width {
            truncate_to_width(&self.data.workspace, remaining_width)
        } else {
            self.data.workspace.clone()
        };

        spans.push(Span::styled(
            workspace_display,
            Style::default().fg(Color::DarkGray),
        ));

        // Streaming indicator
        if self.data.is_streaming {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                "⏳",
                Style::default().fg(Color::Yellow),
            ));
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);

        paragraph.render(area, buf);

        // Bottom border line
        if area.height > 1 {
            let border_y = area.y + 1;
            for x in area.x..area.x + area.width {
                let cell = buf.get_mut(x, border_y);
                cell.set_char('─');
                cell.set_fg(self.border);
            }
        }
    }
}

/// Status indicator animation frames.
pub const STATUS_FRAMES_WHALE: &[&str] = &["🐳", "🐳.", "🐳..", "🐳...", "🐋", "🐋.", "🐋..", "🐋..."];

pub const STATUS_FRAMES_DOTS: &[&str] = &["●", "○", "●", "○", "●", "○"];

/// Get status frame based on elapsed time.
#[must_use]
pub fn get_status_frame(elapsed_ms: u64, frames: &[&str]) -> &'static str {
    if frames.is_empty() {
        return "";
    }
    let frame_duration = 420; // ms per frame
    let idx = (elapsed_ms / frame_duration) as usize % frames.len();
    // Return a static string from predefined frames
    match frames[idx] {
        "🐳" => "🐳",
        "🐳." => "🐳.",
        "🐳.." => "🐳..",
        "🐳..." => "🐳...",
        "🐋" => "🐋",
        "🐋." => "🐋.",
        "🐋.." => "🐋..",
        "🐋..." => "🐋...",
        "●" => "●",
        "○" => "○",
        _ => "●",
    }
}

/// Truncate string to fit width.
fn truncate_to_width(s: &str, max_width: usize) -> String {
    if s.width() <= max_width {
        return s.to_string();
    }

    let mut result = String::new();
    let mut width = 0;

    for g in unicode_segmentation::UnicodeSegmentation::graphemes(s, true) {
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
    fn test_header_basic() {
        let data = HeaderData {
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            workspace: "/Users/test/project".to_string(),
            is_streaming: false,
            status_frame: None,
        };

        let widget = HeaderWidget::new(data);
        assert_eq!(widget.data.model, "gpt-4");
    }

    #[test]
    fn test_header_streaming() {
        let data = HeaderData {
            model: "gpt-4".to_string(),
            provider: "openai".to_string(),
            workspace: "/Users/test".to_string(),
            is_streaming: true,
            status_frame: Some("●".to_string()),
        };

        let widget = HeaderWidget::new(data);
        assert!(widget.data.is_streaming);
    }

    #[test]
    fn test_status_frame() {
        let frame = get_status_frame(0, STATUS_FRAMES_DOTS);
        assert_eq!(frame, "●");

        let frame = get_status_frame(840, STATUS_FRAMES_DOTS); // 2 frames
        assert_eq!(frame, "●");
    }

    #[test]
    fn test_truncate_to_width() {
        let result = truncate_to_width("Hello World", 10);
        assert!(result.width() <= 10);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_truncate_short_string() {
        let result = truncate_to_width("Hi", 10);
        assert_eq!(result, "Hi");
    }
}
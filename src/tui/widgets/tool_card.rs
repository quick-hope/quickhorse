//! ToolCardWidget - Compact tool execution display.
//!
//! Shows:
//! - Tool name and status (running/success/failed)
//! - Input summary (truncated)
//! - Output preview (expandable)
//! - Duration timing

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

/// Tool execution status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)] // Used in tests and future integration
pub enum ToolStatusKind {
    /// Tool is currently running.
    Running,
    /// Tool completed successfully.
    Success,
    /// Tool failed with error.
    Failed,
    /// Tool waiting for approval.
    Pending,
}

/// ToolCardWidget displays a tool execution card.
#[allow(dead_code)] // Future integration with transcript
pub struct ToolCardWidget {
    /// Tool name.
    name: String,
    /// Tool ID (for matching).
    id: String,
    /// Execution status.
    status: ToolStatusKind,
    /// Input summary (first few chars).
    input_summary: String,
    /// Output preview (truncated).
    output_preview: String,
    /// Duration in seconds.
    duration_secs: Option<f32>,
    /// Whether output is expanded (full view).
    expanded: bool,
    /// Theme colors.
    theme: ToolCardTheme,
}

/// Theme colors for ToolCard.
#[derive(Debug, Clone)]
#[allow(dead_code)] // Future use
pub struct ToolCardTheme {
    pub running_color: Color,
    pub success_color: Color,
    pub failed_color: Color,
    pub pending_color: Color,
    pub text_muted: Color,
    pub border_color: Color,
    pub rail_color: Color,
}

impl Default for ToolCardTheme {
    fn default() -> Self {
        Self {
            running_color: Color::Yellow,
            success_color: Color::Green,
            failed_color: Color::Red,
            pending_color: Color::DarkGray,
            text_muted: Color::DarkGray,
            border_color: Color::Gray,
            rail_color: Color::DarkGray,
        }
    }
}

#[allow(dead_code)] // Future integration
impl ToolCardWidget {
    /// Create a new tool card.
    pub fn new(
        name: impl Into<String>,
        id: impl Into<String>,
        status: ToolStatusKind,
    ) -> Self {
        Self {
            name: name.into(),
            id: id.into(),
            status,
            input_summary: String::new(),
            output_preview: String::new(),
            duration_secs: None,
            expanded: false,
            theme: ToolCardTheme::default(),
        }
    }

    /// Set input summary.
    pub fn with_input(mut self, input: impl Into<String>) -> Self {
        self.input_summary = input.into();
        self
    }

    /// Set output preview.
    pub fn with_output(mut self, output: impl Into<String>) -> Self {
        self.output_preview = output.into();
        self
    }

    /// Set duration.
    pub fn with_duration(mut self, secs: f32) -> Self {
        self.duration_secs = Some(secs);
        self
    }

    /// Set expanded state.
    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    /// Set theme.
    pub fn with_theme(mut self, theme: ToolCardTheme) -> Self {
        self.theme = theme;
        self
    }

    /// Get status glyph.
    fn status_glyph(&self) -> &'static str {
        match self.status {
            ToolStatusKind::Running => "⋯",
            ToolStatusKind::Success => "✓",
            ToolStatusKind::Failed => "✗",
            ToolStatusKind::Pending => "⏸",
        }
    }

    /// Get status color.
    fn status_color(&self) -> Color {
        match self.status {
            ToolStatusKind::Running => self.theme.running_color,
            ToolStatusKind::Success => self.theme.success_color,
            ToolStatusKind::Failed => self.theme.failed_color,
            ToolStatusKind::Pending => self.theme.pending_color,
        }
    }

    /// Render as compact lines (for transcript).
    pub fn render_lines(&self, width: u16) -> Vec<Line<'static>> {
        let mut lines = Vec::new();
        let rail = "▏ ";
        let max_content_width = width.saturating_sub(rail.width() as u16) as usize;

        // Header line: status_glyph tool_name duration
        let status_glyph = self.status_glyph();
        let status_color = self.status_color();
        let duration_str = self.duration_secs
            .map(|d| format!(" {:.1}s", d))
            .unwrap_or_default();

        let header_content = format!("{} {}{}", status_glyph, self.name, duration_str);
        let header_display = truncate_to_width(&header_content, max_content_width);

        lines.push(Line::from(vec![
            Span::styled(rail, Style::default().fg(self.theme.rail_color)),
            Span::styled(header_display, Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
        ]));

        // Input summary (if non-empty)
        if !self.input_summary.is_empty() {
            let input_display = truncate_to_width(&self.input_summary, max_content_width);
            lines.push(Line::from(vec![
                Span::styled(rail, Style::default().fg(self.theme.rail_color)),
                Span::styled(input_display, Style::default().fg(self.theme.text_muted)),
            ]));
        }

        // Output preview (if non-empty and not expanded)
        if !self.output_preview.is_empty() && !self.expanded {
            let output_lines = self.output_preview.lines().take(3).collect::<Vec<_>>();
            let output_style = if self.status == ToolStatusKind::Failed {
                Style::default().fg(self.theme.failed_color)
            } else {
                Style::default().fg(self.theme.text_muted)
            };

            for output_line in output_lines {
                let output_display = truncate_to_width(output_line, max_content_width);
                lines.push(Line::from(vec![
                    Span::styled(rail, Style::default().fg(self.theme.rail_color)),
                    Span::styled(output_display, output_style),
                ]));
            }

            // Expand hint
            if self.output_preview.lines().count() > 3 {
                lines.push(Line::from(vec![
                    Span::styled(rail, Style::default().fg(self.theme.rail_color)),
                    Span::styled("(ctrl+o to expand)", Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        // Full output (if expanded)
        if self.expanded && !self.output_preview.is_empty() {
            let output_style = if self.status == ToolStatusKind::Failed {
                Style::default().fg(self.theme.failed_color)
            } else {
                Style::default().fg(self.theme.text_muted)
            };

            for output_line in self.output_preview.lines() {
                let output_display = truncate_to_width(output_line, max_content_width);
                lines.push(Line::from(vec![
                    Span::styled(rail, Style::default().fg(self.theme.rail_color)),
                    Span::styled(output_display, output_style),
                ]));
            }
        }

        lines
    }

    /// Check if card is running.
    #[must_use]
    pub fn is_running(&self) -> bool {
        self.status == ToolStatusKind::Running
    }

    /// Check if card is failed.
    #[must_use]
    pub fn is_failed(&self) -> bool {
        self.status == ToolStatusKind::Failed
    }
}

impl Widget for ToolCardWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let lines = self.render_lines(area.width);
        let paragraph = Paragraph::new(lines);
        paragraph.render(area, buf);
    }
}

/// Truncate string to fit width.
#[allow(dead_code)] // Used in tests
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
    fn test_tool_card_running() {
        let card = ToolCardWidget::new("BashTool", "tool_123", ToolStatusKind::Running);
        assert!(card.is_running());
        assert_eq!(card.status_glyph(), "⋯");
    }

    #[test]
    fn test_tool_card_success() {
        let card = ToolCardWidget::new("ReadTool", "tool_456", ToolStatusKind::Success)
            .with_duration(1.5);

        assert!(!card.is_running());
        assert_eq!(card.status_glyph(), "✓");
        assert_eq!(card.duration_secs, Some(1.5));
    }

    #[test]
    fn test_tool_card_failed() {
        let card = ToolCardWidget::new("WriteTool", "tool_789", ToolStatusKind::Failed)
            .with_output("Error: Permission denied");

        assert!(card.is_failed());
        assert_eq!(card.status_glyph(), "✗");
    }

    #[test]
    fn test_tool_card_render_lines() {
        let card = ToolCardWidget::new("BashTool", "tool_1", ToolStatusKind::Success)
            .with_input("ls -la")
            .with_output("file1\nfile2\nfile3")
            .with_duration(0.5);

        let lines = card.render_lines(80);
        assert!(!lines.is_empty());
        assert!(lines[0].spans[1].content.contains("BashTool"));
    }

    #[test]
    fn test_tool_card_expanded() {
        let card = ToolCardWidget::new("BashTool", "tool_1", ToolStatusKind::Success)
            .with_output("line1\nline2\nline3\nline4\nline5")
            .expanded(true);

        let lines = card.render_lines(80);
        // Expanded should show all 5 output lines
        let output_count = lines.iter().filter(|l| l.spans.len() > 1 && l.spans[1].content.starts_with("line")).count();
        assert_eq!(output_count, 5);
    }

    #[test]
    fn test_tool_card_expand_hint() {
        let card = ToolCardWidget::new("BashTool", "tool_1", ToolStatusKind::Success)
            .with_output("line1\nline2\nline3\nline4\nline5");

        let lines = card.render_lines(80);
        // Should have expand hint since output > 3 lines
        let has_hint = lines.iter().any(|l| l.spans.iter().any(|s| s.content.contains("expand")));
        assert!(has_hint);
    }

    #[test]
    fn test_truncate_to_width() {
        let result = truncate_to_width("Hello World this is a long string", 15);
        assert!(result.width() <= 15);
        assert!(result.ends_with("..."));
    }

    #[test]
    fn test_tool_status_kind() {
        assert_ne!(ToolStatusKind::Running, ToolStatusKind::Success);
        assert_ne!(ToolStatusKind::Success, ToolStatusKind::Failed);
    }
}
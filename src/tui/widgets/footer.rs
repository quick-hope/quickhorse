//! FooterWidget - Bottom bar displaying mode, status, and info chips.
//!
//! Shows:
//! - Mode indicator (agent/yolo/plan)
//! - Status label (ready/thinking/working)
//! - Info chips: tokens, cost, duration
//! - Working animation strip

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Widget},
};
use unicode_width::UnicodeWidthStr;

/// FooterWidget displays mode and status info.
pub struct FooterWidget {
    /// Pre-computed props.
    props: FooterProps,
}

/// Pre-computed data for footer rendering.
#[derive(Debug, Clone)]
pub struct FooterProps {
    /// Mode label ("agent", "yolo", "plan").
    pub mode_label: &'static str,
    /// Mode color.
    pub mode_color: Color,
    /// Status label ("ready", "thinking", "working").
    pub status_label: &'static str,
    /// Status color.
    pub status_color: Color,
    /// Token count (optional).
    pub tokens: Option<u32>,
    /// Session cost (optional).
    pub cost: Option<f64>,
    /// Turn duration in seconds (optional).
    pub duration_secs: Option<f32>,
    /// Tool count (if multiple running).
    pub tool_count: Option<usize>,
    /// Background color.
    pub background: Color,
    /// Text muted color.
    pub text_muted: Color,
}

impl FooterWidget {
    /// Create a new footer widget.
    pub fn new(props: FooterProps) -> Self {
        Self { props }
    }

    /// Create default footer (ready state).
    pub fn ready() -> Self {
        Self::new(FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "ready",
            status_color: Color::Green,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: None,
            background: Color::Reset,
            text_muted: Color::DarkGray,
        })
    }

    /// Create footer for streaming state.
    pub fn streaming(tool_count: usize) -> Self {
        Self::new(FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "thinking",
            status_color: Color::Yellow,
            tokens: None,
            cost: None,
            duration_secs: None,
            tool_count: Some(tool_count),
            background: Color::Reset,
            text_muted: Color::DarkGray,
        })
    }

    /// Create footer for working state (tools active).
    pub fn working(tool_count: usize, duration_secs: f32) -> Self {
        Self::new(FooterProps {
            mode_label: "agent",
            mode_color: Color::Cyan,
            status_label: "working",
            status_color: Color::Magenta,
            tokens: None,
            cost: None,
            duration_secs: Some(duration_secs),
            tool_count: Some(tool_count),
            background: Color::Reset,
            text_muted: Color::DarkGray,
        })
    }
}

impl Widget for FooterWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        if area.height == 0 {
            return;
        }

        // Build footer line
        let mut spans = Vec::new();

        // Mode chip
        spans.push(Span::styled(
            format!("[{}]", self.props.mode_label),
            Style::default().fg(self.props.mode_color).add_modifier(Modifier::BOLD),
        ));
        spans.push(Span::raw(" "));

        // Status label
        spans.push(Span::styled(
            self.props.status_label.to_string(),
            Style::default().fg(self.props.status_color),
        ));

        // Tool count (if active)
        if let Some(count) = self.props.tool_count {
            if count > 0 {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(
                    format!("({} tools)", count),
                    Style::default().fg(self.props.text_muted),
                ));
            }
        }

        // Duration (if working)
        if let Some(duration) = self.props.duration_secs {
            spans.push(Span::raw(" "));
            spans.push(Span::styled(
                format_duration(duration),
                Style::default().fg(self.props.text_muted),
            ));
        }

        // Right side: tokens and cost
        let left_width = spans.iter().map(|s| s.content.width()).sum::<usize>();
        let right_width = area.width as usize - left_width - 2;

        // Fill remaining space
        if right_width > 10 {
            spans.push(Span::raw(" ".repeat(right_width.saturating_sub(10))));

            // Cost (if available)
            if let Some(cost) = self.props.cost {
                if cost > 0.01 {
                    spans.push(Span::styled(
                        format!("${:.2}", cost),
                        Style::default().fg(Color::Green),
                    ));
                }
            }
        }

        let line = Line::from(spans);
        let paragraph = Paragraph::new(line);

        paragraph.render(area, buf);
    }
}

/// Format duration in human-readable form.
fn format_duration(secs: f32) -> String {
    if secs < 60.0 {
        format!("{:.0}s", secs)
    } else {
        let mins = (secs / 60.0).floor() as u32;
        let remaining_secs = secs - (mins as f32 * 60.0);
        format!("{}m {:.0}s", mins, remaining_secs)
    }
}

/// Wave animation glyphs for working state.
pub const WAVE_GLYPHS: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// Get wave glyph for animation frame.
#[must_use]
pub fn get_wave_glyph(frame: u64, col: usize, width: usize) -> char {
    let idx = (frame as usize + col) % WAVE_GLYPHS.len();
    WAVE_GLYPHS[idx]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_footer_ready() {
        let footer = FooterWidget::ready();
        assert_eq!(footer.props.status_label, "ready");
    }

    #[test]
    fn test_footer_streaming() {
        let footer = FooterWidget::streaming(0);
        assert_eq!(footer.props.status_label, "thinking");
    }

    #[test]
    fn test_footer_working() {
        let footer = FooterWidget::working(3, 45.0);
        assert_eq!(footer.props.status_label, "working");
        assert_eq!(footer.props.tool_count, Some(3));
        assert_eq!(footer.props.duration_secs, Some(45.0));
    }

    #[test]
    fn test_format_duration_short() {
        let result = format_duration(15.0);
        assert_eq!(result, "15s");
    }

    #[test]
    fn test_format_duration_minutes() {
        let result = format_duration(90.0);
        assert_eq!(result, "1m 30s");
    }

    #[test]
    fn test_format_duration_long() {
        let result = format_duration(185.0);
        assert_eq!(result, "3m 5s");
    }

    #[test]
    fn test_wave_glyphs() {
        let glyph = get_wave_glyph(0, 0, 10);
        assert_eq!(glyph, '▁');

        let glyph = get_wave_glyph(0, 7, 10);
        assert_eq!(glyph, '█');
    }
}
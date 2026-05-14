//! ChatWidget - Main transcript display widget.
//!
//! Renders conversation history with:
//! - Per-cell revision caching (performance optimization)
//! - Scrollbar when content exceeds viewport
//! - "Jump to latest" button when scrolled up
//! - Selection highlighting

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Style},
    text::Line,
    widgets::{Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget, Clear},
};

use super::super::transcript_cache::TranscriptViewCache;
use super::super::scroll_state::TranscriptScroll;
use super::super::history_cell::HistoryCell;

/// Jump to latest button dimensions
const JUMP_BUTTON_WIDTH: u16 = 3;
const JUMP_BUTTON_HEIGHT: u16 = 3;

/// ChatWidget renders the transcript with caching and scrolling.
pub struct ChatWidget {
    /// Content area for rendering.
    content_area: Rect,
    /// Lines to render (from cache).
    lines: Vec<Line<'static>>,
    /// Scrollbar state (if needed).
    scrollbar: Option<ScrollbarData>,
    /// Jump to latest button position (if scrolled up).
    jump_button: Option<Rect>,
    /// Background color.
    background: Color,
    /// Scroll track color.
    scroll_track: Color,
    /// Scroll thumb color.
    scroll_thumb: Color,
    /// Jump button border color.
    jump_border: Color,
    /// Jump button arrow color.
    jump_arrow: Color,
}

/// Scrollbar data for rendering.
#[derive(Debug, Clone, Copy)]
struct ScrollbarData {
    top: usize,
    visible: usize,
    total: usize,
}

impl ChatWidget {
    /// Create a new ChatWidget from app state.
    pub fn new(
        cells: &[HistoryCell],
        cell_revisions: &[u64],
        scroll: TranscriptScroll,
        width: u16,
        height: u16,
        theme: &ChatTheme,
    ) -> Self {
        let cache = build_cache(cells, cell_revisions, width);
        let total_lines = cache.total_lines();
        let visible_lines = height as usize;

        // Resolve scroll state
        let (resolved_scroll, top) = scroll.resolve_top(total_lines, visible_lines);

        // Get visible lines
        let lines = cache.visible_lines(top, visible_lines);

        // Pad if at tail
        let final_lines = if resolved_scroll.is_at_tail() && lines.len() < visible_lines {
            let padding = visible_lines - lines.len();
            let mut padded = lines;
            for _ in 0..padding {
                padded.push(Line::from(""));
            }
            padded
        } else {
            lines
        };

        // Scrollbar (if content exceeds viewport)
        let scrollbar = (total_lines > visible_lines && width > 1).then_some(ScrollbarData {
            top,
            visible: visible_lines,
            total: total_lines,
        });

        // Jump button (if scrolled up from tail)
        let jump_button = if !resolved_scroll.is_at_tail() && scrollbar.is_some() {
            Some(jump_button_rect(Rect::new(0, 0, width, height), true))
        } else {
            None
        };

        Self {
            content_area: Rect::new(0, 0, width, height),
            lines: final_lines,
            scrollbar,
            jump_button,
            background: theme.background,
            scroll_track: theme.scroll_track,
            scroll_thumb: theme.scroll_thumb,
            jump_border: theme.jump_border,
            jump_arrow: theme.jump_arrow,
        }
    }

    /// Create widget from pre-built lines (for testing).
    pub fn from_lines(lines: Vec<Line<'static>>, area: Rect, theme: &ChatTheme) -> Self {
        Self {
            content_area: area,
            lines,
            scrollbar: None,
            jump_button: None,
            background: theme.background,
            scroll_track: theme.scroll_track,
            scroll_thumb: theme.scroll_thumb,
            jump_border: theme.jump_border,
            jump_arrow: theme.jump_arrow,
        }
    }

    /// Check if widget has scrollbar.
    #[must_use]
    pub fn has_scrollbar(&self) -> bool {
        self.scrollbar.is_some()
    }

    /// Check if widget has jump button.
    #[must_use]
    pub fn has_jump_button(&self) -> bool {
        self.jump_button.is_some()
    }

    /// Get total lines count.
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.scrollbar.map(|s| s.total).unwrap_or(self.lines.len())
    }
}

/// Theme colors for ChatWidget.
#[derive(Debug, Clone)]
pub struct ChatTheme {
    pub background: Color,
    pub scroll_track: Color,
    pub scroll_thumb: Color,
    pub jump_border: Color,
    pub jump_arrow: Color,
}

impl Default for ChatTheme {
    fn default() -> Self {
        Self {
            background: Color::Reset,
            scroll_track: Color::DarkGray,
            scroll_thumb: Color::Gray,
            jump_border: Color::Gray,
            jump_arrow: Color::Cyan,
        }
    }
}

impl Widget for ChatWidget {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Render main content
        let content_area = Rect::new(
            area.x,
            area.y,
            area.width.saturating_sub(if self.scrollbar.is_some() { 1 } else { 0 }),
            area.height,
        );

        let paragraph = Paragraph::new(self.lines.clone());
        paragraph.render(content_area, buf);

        // Render scrollbar if needed
        if let Some(sb) = self.scrollbar {
            let scrollbar_area = Rect::new(
                area.x + area.width.saturating_sub(1),
                area.y,
                1,
                area.height,
            );

            let mut state = ScrollbarState::new(sb.total)
                .position(sb.top)
                .viewport_content_length(sb.visible);

            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .track_style(Style::default().fg(self.scroll_track))
                .thumb_style(Style::default().fg(self.scroll_thumb));

            scrollbar.render(scrollbar_area, buf, &mut state);
        }

        // Render jump to latest button if scrolled up
        if let Some(btn_area) = self.jump_button {
            render_jump_button(btn_area, buf, self.jump_border, self.jump_arrow);
        }
    }
}

/// Build cache from cells.
fn build_cache(cells: &[HistoryCell], revisions: &[u64], width: u16) -> TranscriptViewCache {
    let mut cache = TranscriptViewCache::new();

    // Convert cells to lines
    let cell_lines: Vec<Vec<Line<'static>>> = cells
        .iter()
        .filter(|c| !c.is_empty())
        .map(|c| c.lines(width))
        .collect();

    // Pad revisions to match cell count
    let padded_revs: Vec<u64> = if revisions.len() < cell_lines.len() {
        let mut revs = revisions.to_vec();
        for _ in revisions.len()..cell_lines.len() {
            revs.push(u64::MAX); // Force re-render for missing revisions
        }
        revs
    } else {
        revisions[..cell_lines.len()].to_vec()
    };

    cache.ensure(&cell_lines, &padded_revs, width);
    cache
}

/// Calculate jump button position.
fn jump_button_rect(area: Rect, has_scrollbar: bool) -> Rect {
    let x = area.x + area.width.saturating_sub(
        if has_scrollbar { JUMP_BUTTON_WIDTH + 1 } else { JUMP_BUTTON_WIDTH }
    );
    let y = area.y;

    Rect::new(x, y, JUMP_BUTTON_WIDTH, JUMP_BUTTON_HEIGHT)
}

/// Render jump to latest button.
fn render_jump_button(area: Rect, buf: &mut Buffer, border_color: Color, arrow_color: Color) {
    // Clear the area
    Clear.render(area, buf);

    // Draw border using get_mut (ratatui 0.26 returns &mut Cell directly)
    for y in area.y..area.y + area.height {
        for x in area.x..area.x + area.width {
            let cell = buf.get_mut(x, y);
            if y == area.y || y == area.y + area.height - 1 {
                cell.set_char('─');
                cell.set_fg(border_color);
            } else if x == area.x || x == area.x + area.width - 1 {
                cell.set_char('│');
                cell.set_fg(border_color);
            }
        }
    }

    // Draw arrow (↓)
    let arrow_x = area.x + area.width / 2;
    let arrow_y = area.y + area.height / 2;
    let cell = buf.get_mut(arrow_x, arrow_y);
    cell.set_char('↓');
    cell.set_fg(arrow_color);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_widget() {
        let theme = ChatTheme::default();
        let widget = ChatWidget::new(&[], &[], TranscriptScroll::to_bottom(), 80, 20, &theme);

        assert!(!widget.has_scrollbar());
        assert!(!widget.has_jump_button());
    }

    #[test]
    fn test_single_cell() {
        let cells = vec![HistoryCell::User { content: "Hello".to_string() }];
        let revisions = vec![1];
        let theme = ChatTheme::default();

        let widget = ChatWidget::new(&cells, &revisions, TranscriptScroll::to_bottom(), 80, 20, &theme);

        assert!(!widget.has_scrollbar()); // Fits in viewport
    }

    #[test]
    fn test_scrollbar_when_exceeds_viewport() {
        let cells = vec![
            HistoryCell::User { content: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5\nLine 6\nLine 7\nLine 8".to_string() },
        ];
        let revisions = vec![1];
        let theme = ChatTheme::default();

        let widget = ChatWidget::new(&cells, &revisions, TranscriptScroll::to_bottom(), 80, 5, &theme);

        assert!(widget.has_scrollbar());
    }

    #[test]
    fn test_jump_button_when_scrolled_up() {
        let cells = vec![
            HistoryCell::User { content: "Line 1\nLine 2\nLine 3\nLine 4\nLine 5".to_string() },
        ];
        let revisions = vec![1];
        let theme = ChatTheme::default();

        let scroll = TranscriptScroll::at_line(0); // Scrolled to top
        let widget = ChatWidget::new(&cells, &revisions, scroll, 80, 3, &theme);

        assert!(widget.has_scrollbar());
        assert!(widget.has_jump_button());
    }

    #[test]
    fn test_from_lines() {
        let lines = vec![Line::from("Test")];
        let area = Rect::new(0, 0, 80, 20);
        let theme = ChatTheme::default();

        let widget = ChatWidget::from_lines(lines, area, &theme);
        assert_eq!(widget.lines.len(), 1);
    }

    #[test]
    fn test_jump_button_rect() {
        let area = Rect::new(0, 0, 80, 20);
        let btn = jump_button_rect(area, true);

        assert_eq!(btn.width, JUMP_BUTTON_WIDTH);
        assert_eq!(btn.height, JUMP_BUTTON_HEIGHT);
    }
}
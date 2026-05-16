//! Scroll state tracking for transcript rendering.
//!
//! The transcript view uses a flat line-index scroll model: a single `offset`
//! into the rendered line buffer points at the top visible line, with
//! `usize::MAX` reserved as a sentinel meaning "stuck to the live tail."

#![allow(dead_code)] // Future use: full scroll integration

/// Sentinel offset meaning "stuck to live tail" — the renderer translates
/// this to `max_start` at draw time, so newly appended lines pull the view
/// down with them.
const TAIL_SENTINEL: usize = usize::MAX;

/// Flat line-offset scroll state for the transcript view.
///
/// Stores the index of the top visible line, or `TAIL_SENTINEL` to mean
/// "stuck to bottom."
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscriptScroll {
    offset: usize,
}

impl Default for TranscriptScroll {
    /// Default state is "stuck to live tail".
    fn default() -> Self {
        Self::to_bottom()
    }
}

impl TranscriptScroll {
    /// State that follows the live tail (default).
    #[must_use]
    pub const fn to_bottom() -> Self {
        Self { offset: TAIL_SENTINEL }
    }

    /// State pinned to a specific line index.
    #[must_use]
    pub const fn at_line(offset: usize) -> Self {
        Self { offset }
    }

    /// Returns true when the view is following the live tail.
    #[must_use]
    pub const fn is_at_tail(self) -> bool {
        self.offset == TAIL_SENTINEL
    }

    /// Resolve the scroll state to a concrete top line index.
    ///
    /// `max_start` is `total_lines.saturating_sub(visible_lines)`. The
    /// returned `Self` is the canonicalized state — if the resolved top
    /// reached the tail, we collapse to `to_bottom`.
    #[must_use]
    pub fn resolve_top(self, total_lines: usize, visible_lines: usize) -> (Self, usize) {
        if total_lines <= visible_lines {
            // Whole transcript fits; only "tail" is meaningful
            return (Self::to_bottom(), 0);
        }

        let max_start = total_lines.saturating_sub(visible_lines);

        if self.offset == TAIL_SENTINEL {
            return (Self::to_bottom(), max_start);
        }

        let top = self.offset.min(max_start);
        if top >= max_start {
            (Self::to_bottom(), max_start)
        } else {
            (Self::at_line(top), top)
        }
    }

    /// Apply a scroll delta and return the updated state.
    ///
    /// `delta_lines` is signed: negative scrolls up (toward the start),
    /// positive scrolls down (toward the tail). When the resolved offset
    /// hits `max_start` we snap to `to_bottom` so subsequent appended
    /// content pulls the view along.
    #[must_use]
    pub fn scrolled_by(self, delta_lines: i32, total_lines: usize, visible_lines: usize) -> Self {
        if delta_lines == 0 {
            return self;
        }

        if total_lines <= visible_lines {
            // Whole transcript fits; only "tail" is meaningful
            return Self::to_bottom();
        }

        let max_start = total_lines.saturating_sub(visible_lines);
        let current_top = if self.offset == TAIL_SENTINEL {
            max_start
        } else {
            self.offset.min(max_start)
        };

        let new_top = if delta_lines < 0 {
            current_top.saturating_sub(delta_lines.unsigned_abs() as usize)
        } else {
            let delta = usize::try_from(delta_lines).unwrap_or(usize::MAX);
            current_top.saturating_add(delta).min(max_start)
        };

        if new_top >= max_start {
            Self::to_bottom()
        } else {
            Self::at_line(new_top)
        }
    }

    /// Get the raw offset value (for debugging).
    #[must_use]
    pub fn raw_offset(self) -> usize {
        self.offset
    }
}

/// Direction for scroll input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrollDirection {
    Up,
    Down,
}

impl ScrollDirection {
    /// Convert direction to signed delta.
    #[must_use]
    pub fn delta(self, lines: usize) -> i32 {
        match self {
            ScrollDirection::Up => -(lines as i32),
            ScrollDirection::Down => lines as i32,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_is_tail() {
        let scroll = TranscriptScroll::default();
        assert!(scroll.is_at_tail());
    }

    #[test]
    fn test_to_bottom() {
        let scroll = TranscriptScroll::to_bottom();
        assert!(scroll.is_at_tail());
    }

    #[test]
    fn test_at_line() {
        let scroll = TranscriptScroll::at_line(5);
        assert!(!scroll.is_at_tail());
        assert_eq!(scroll.raw_offset(), 5);
    }

    #[test]
    fn test_resolve_tail_when_fits() {
        let scroll = TranscriptScroll::at_line(5);
        let (resolved, top) = scroll.resolve_top(10, 20); // Fits in one screen

        assert!(resolved.is_at_tail());
        assert_eq!(top, 0);
    }

    #[test]
    fn test_resolve_tail_sentinel() {
        let scroll = TranscriptScroll::to_bottom();
        let (resolved, top) = scroll.resolve_top(100, 20);

        assert!(resolved.is_at_tail());
        assert_eq!(top, 80); // 100 - 20 = 80
    }

    #[test]
    fn test_resolve_at_line_within_bounds() {
        let scroll = TranscriptScroll::at_line(10);
        let (resolved, top) = scroll.resolve_top(100, 20);

        assert!(!resolved.is_at_tail());
        assert_eq!(top, 10);
    }

    #[test]
    fn test_resolve_at_line_clamped() {
        let scroll = TranscriptScroll::at_line(90); // Beyond max_start (80)
        let (resolved, top) = scroll.resolve_top(100, 20);

        assert!(resolved.is_at_tail());
        assert_eq!(top, 80);
    }

    #[test]
    fn test_scroll_up() {
        let scroll = TranscriptScroll::to_bottom();
        let scrolled = scroll.scrolled_by(-5, 100, 20);

        assert!(!scrolled.is_at_tail());
        assert_eq!(scrolled.raw_offset(), 75); // 80 - 5 = 75
    }

    #[test]
    fn test_scroll_down() {
        let scroll = TranscriptScroll::at_line(70);
        let scrolled = scroll.scrolled_by(5, 100, 20);

        assert!(!scrolled.is_at_tail());
        assert_eq!(scrolled.raw_offset(), 75);
    }

    #[test]
    fn test_scroll_down_to_tail() {
        let scroll = TranscriptScroll::at_line(75);
        let scrolled = scroll.scrolled_by(10, 100, 20);

        // Would go to 85, but max_start is 80, so snap to tail
        assert!(scrolled.is_at_tail());
    }

    #[test]
    fn test_scroll_up_from_tail() {
        let scroll = TranscriptScroll::to_bottom();
        let scrolled = scroll.scrolled_by(-1, 100, 20);

        assert!(!scrolled.is_at_tail());
        assert_eq!(scrolled.raw_offset(), 79); // 80 - 1 = 79
    }

    #[test]
    fn test_scroll_direction_delta() {
        assert_eq!(ScrollDirection::Up.delta(3), -3);
        assert_eq!(ScrollDirection::Down.delta(3), 3);
    }

    #[test]
    fn test_no_scroll_when_fits() {
        let scroll = TranscriptScroll::at_line(5);
        let scrolled = scroll.scrolled_by(-3, 10, 20); // Fits

        assert!(scrolled.is_at_tail());
    }

    #[test]
    fn test_scroll_zero_no_change() {
        let scroll = TranscriptScroll::at_line(10);
        let scrolled = scroll.scrolled_by(0, 100, 20);

        assert_eq!(scroll, scrolled);
    }
}
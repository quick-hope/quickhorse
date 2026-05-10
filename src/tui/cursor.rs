//! Cursor and MeasuredText - Unicode text handling with proper grapheme/width support
//!
//! This module provides:
//! - MeasuredText: NFC normalized text with grapheme and display width info
//! - Cursor: Position management with proper Unicode handling
//! - KillRing: Emacs-style clipboard history
//!
//! Key concepts from OpenClaude:
//! - All text is normalized to NFC for consistent handling
//! - Grapheme clusters (like 👨‍👩‍👧‍👦) are treated as single units
//! - Display width is calculated for CJK characters (2 width each)
//! - Wrapped lines account for terminal width

use std::time::{Duration, Instant};
use unicode_normalization::UnicodeNormalization;
use unicode_segmentation::UnicodeSegmentation;
use unicode_width::UnicodeWidthStr;

/// Kill direction for accumulation
pub enum KillDirection {
    Prepend,
    Append,
}

/// Yank state tracking
struct YankState {
    last_start: usize,
    last_length: usize,
    was_yank: bool,
    index: usize,
}

impl Default for YankState {
    fn default() -> Self {
        Self {
            last_start: 0,
            last_length: 0,
            was_yank: false,
            index: 0,
        }
    }
}

/// Kill Ring - Emacs style clipboard history
///
/// Consecutive kills accumulate in the kill ring until the user types some
/// other key. Alt+Y cycles through previous kills after a yank.
pub struct KillRing {
    items: Vec<String>,
    max_size: usize,
    last_was_kill: bool,
    yank_state: YankState,
}

impl KillRing {
    /// Create a new kill ring with max size
    pub fn new(max_size: usize) -> Self {
        Self {
            items: Vec::new(),
            max_size,
            last_was_kill: false,
            yank_state: YankState::default(),
        }
    }

    /// Push text to kill ring
    pub fn push(&mut self, text: String, direction: KillDirection) {
        if text.is_empty() {
            return;
        }

        if self.last_was_kill && !self.items.is_empty() {
            // Accumulate with the most recent kill
            match direction {
                KillDirection::Prepend => {
                    self.items[0] = text + &self.items[0];
                }
                KillDirection::Append => {
                    self.items[0].push_str(&text);
                }
            }
        } else {
            // Add new entry to front of ring
            self.items.insert(0, text);
            if self.items.len() > self.max_size {
                self.items.pop();
            }
        }

        self.last_was_kill = true;
        self.yank_state.was_yank = false;
    }

    /// Get the last killed text
    pub fn get_last(&self) -> Option<&String> {
        self.items.first()
    }

    /// Get kill ring item at index
    pub fn get_item(&self, index: usize) -> Option<&String> {
        let len = self.items.len();
        if len == 0 {
            return None;
        }
        let normalized = ((index % len) + len) % len;
        self.items.get(normalized)
    }

    /// Get kill ring size
    pub fn size(&self) -> usize {
        self.items.len()
    }

    /// Reset kill accumulation
    pub fn reset_accumulation(&mut self) {
        self.last_was_kill = false;
    }

    /// Record a yank operation
    pub fn record_yank(&mut self, start: usize, length: usize) {
        self.yank_state.last_start = start;
        self.yank_state.last_length = length;
        self.yank_state.was_yank = true;
        self.yank_state.index = 0;
    }

    /// Check if yank-pop is available
    pub fn can_yank_pop(&self) -> bool {
        self.yank_state.was_yank && self.items.len() > 1
    }

    /// Yank-pop: cycle to next kill ring item
    pub fn yank_pop(&mut self) -> Option<(String, usize, usize)> {
        if !self.yank_state.was_yank || self.items.len() <= 1 {
            return None;
        }

        self.yank_state.index = (self.yank_state.index + 1) % self.items.len();
        let text = self.items.get(self.yank_state.index).cloned()?;
        Some((text, self.yank_state.last_start, self.yank_state.last_length))
    }

    /// Update yank length after replacement
    pub fn update_yank_length(&mut self, length: usize) {
        self.yank_state.last_length = length;
    }

    /// Reset yank state
    pub fn reset_yank_state(&mut self) {
        self.yank_state.was_yank = false;
    }

    /// Clear the kill ring
    pub fn clear(&mut self) {
        self.items.clear();
        self.last_was_kill = false;
        self.yank_state = YankState::default();
    }
}

/// Wrapped line information
#[derive(Debug, Clone)]
pub struct WrappedLine {
    /// Original line number (0-indexed)
    pub original_line: usize,
    /// Start byte offset in original text
    pub start_offset: usize,
    /// End byte offset in original text
    pub end_offset: usize,
    /// Display text (may be wrapped from original)
    pub display_text: String,
    /// Display width (columns)
    pub width: usize,
}

/// MeasuredText - NFC normalized text with grapheme and width info
///
/// Text processing flow:
/// 1. User input (raw text, potentially mixed NFD/NFC)
/// 2. MeasuredText normalizes to NFC + builds grapheme info
/// 3. All cursor operations use normalized text/offsets
/// 4. Display uses normalized text from wrapped_lines
pub struct MeasuredText {
    /// NFC normalized text
    text: String,
    /// Terminal columns (for wrapping)
    columns: usize,
    /// Wrapped lines (accounting for terminal width)
    wrapped_lines: Vec<WrappedLine>,
    /// Original lines (before wrapping)
    original_lines: Vec<String>,
}

impl MeasuredText {
    /// Create new MeasuredText with given terminal columns
    pub fn new(text: String, columns: usize) -> Self {
        // NFC normalization for consistent handling
        let normalized = text.nfc().collect::<String>();

        // Split into original lines
        let original_lines: Vec<String> = normalized.lines().map(|s| s.to_string()).collect();

        // Compute wrapped lines
        let wrapped_lines = Self::compute_wrapped_lines(&normalized, columns, &original_lines);

        Self {
            text: normalized,
            columns,
            wrapped_lines,
            original_lines,
        }
    }

    /// Get the normalized text
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Get text length in bytes
    pub fn len(&self) -> usize {
        self.text.len()
    }

    /// Check if text is empty
    pub fn is_empty(&self) -> bool {
        self.text.is_empty()
    }

    /// Get grapheme count
    pub fn grapheme_count(&self) -> usize {
        self.text.graphemes(true).count()
    }

    /// Get terminal columns
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Get wrapped lines
    pub fn wrapped_lines(&self) -> &[WrappedLine] {
        &self.wrapped_lines
    }

    /// Get original lines
    pub fn original_lines(&self) -> &[String] {
        &self.original_lines
    }

    /// Get total wrapped line count
    pub fn wrapped_line_count(&self) -> usize {
        self.wrapped_lines.len()
    }

    /// Get total original line count
    pub fn original_line_count(&self) -> usize {
        self.original_lines.len()
    }

    /// Compute wrapped lines accounting for terminal width
    fn compute_wrapped_lines(
        _text: &str,
        columns: usize,
        original_lines: &[String],
    ) -> Vec<WrappedLine> {
        if columns == 0 {
            // No wrapping needed
            return original_lines
                .iter()
                .enumerate()
                .map(|(i, line)| WrappedLine {
                    original_line: i,
                    start_offset: 0,
                    end_offset: line.len(),
                    display_text: line.clone(),
                    width: line.width(),
                })
                .collect();
        }

        let mut wrapped = Vec::new();
        let mut offset = 0;

        for (line_idx, line) in original_lines.iter().enumerate() {
            let line_start_offset = offset;
            let line_width = line.width();

            if line_width <= columns {
                // Line fits in terminal width
                wrapped.push(WrappedLine {
                    original_line: line_idx,
                    start_offset: line_start_offset,
                    end_offset: line_start_offset + line.len(),
                    display_text: line.clone(),
                    width: line_width,
                });
            } else {
                // Line needs wrapping
                let mut current_width = 0;
                let mut current_start = 0;
                let mut current_text = String::new();

                for grapheme in line.graphemes(true) {
                    let g_width = grapheme.width();

                    if current_width + g_width > columns {
                        // Wrap current segment
                        wrapped.push(WrappedLine {
                            original_line: line_idx,
                            start_offset: line_start_offset + current_start,
                            end_offset: line_start_offset + current_text.len(),
                            display_text: current_text.clone(),
                            width: current_width,
                        });

                        // Start new segment
                        current_start += current_text.len();
                        current_text = grapheme.to_string();
                        current_width = g_width;
                    } else {
                        current_text.push_str(grapheme);
                        current_width += g_width;
                    }
                }

                // Push remaining segment
                if !current_text.is_empty() {
                    wrapped.push(WrappedLine {
                        original_line: line_idx,
                        start_offset: line_start_offset + current_start,
                        end_offset: line_start_offset + line.len(),
                        display_text: current_text,
                        width: current_width,
                    });
                }
            }

            offset += line.len() + 1; // +1 for newline
        }

        wrapped
    }

    /// Find the wrapped line containing a given byte offset
    pub fn find_wrapped_line_at_offset(&self, offset: usize) -> Option<usize> {
        for (i, line) in self.wrapped_lines.iter().enumerate() {
            if offset >= line.start_offset && offset <= line.end_offset {
                return Some(i);
            }
        }
        None
    }

    /// Find the original line containing a given byte offset
    pub fn find_original_line_at_offset(&self, offset: usize) -> Option<usize> {
        let mut current_offset = 0;
        for (i, line) in self.original_lines.iter().enumerate() {
            if offset >= current_offset && offset <= current_offset + line.len() {
                return Some(i);
            }
            current_offset += line.len() + 1;
        }
        None
    }

    /// Get display column at byte offset
    pub fn get_display_column_at_offset(&self, offset: usize) -> usize {
        if let Some(wrapped_idx) = self.find_wrapped_line_at_offset(offset) {
            let line = &self.wrapped_lines[wrapped_idx];
            let relative_offset = offset - line.start_offset;
            let prefix = &line.display_text[..relative_offset.min(line.display_text.len())];
            prefix.width()
        } else {
            0
        }
    }

    /// Get grapheme at byte offset
    pub fn grapheme_at_offset(&self, offset: usize) -> Option<&str> {
        self.text[offset..].graphemes(true).next()
    }

    /// Get previous grapheme before byte offset
    pub fn prev_grapheme_at_offset(&self, offset: usize) -> Option<&str> {
        if offset == 0 {
            return None;
        }
        self.text[..offset].graphemes(true).rev().next()
    }

    /// Convert byte offset to grapheme offset
    pub fn byte_to_grapheme_offset(&self, byte_offset: usize) -> usize {
        self.text[..byte_offset.min(self.text.len())].graphemes(true).count()
    }

    /// Convert grapheme offset to byte offset
    pub fn grapheme_to_byte_offset(&self, grapheme_offset: usize) -> usize {
        let mut offset = 0;
        for (i, g) in self.text.graphemes(true).enumerate() {
            if i >= grapheme_offset {
                break;
            }
            offset += g.len();
        }
        offset
    }

    /// Find word boundaries around offset
    pub fn find_word_boundaries(&self, offset: usize) -> (usize, usize) {
        let words: Vec<_> = self.text.split_word_bounds().collect();

        for word in words.iter().filter(|w| !w.chars().all(|c| c.is_whitespace())) {
            // Find position of this word in text
            let start = self.text.find(word).unwrap_or(0);
            let end = start + word.len();
            if offset >= start && offset <= end {
                return (start, end);
            }
        }

        // Default: entire text
        (0, self.text.len())
    }

    /// Find previous word start before offset
    ///
    /// If cursor is inside a word (not at start), returns start of that word.
    /// If cursor is at word start or in whitespace, returns start of previous word.
    pub fn find_prev_word_start(&self, offset: usize) -> usize {
        let mut prev_word_starts: Vec<usize> = Vec::new();

        for (idx, word) in self.text.split_word_bound_indices() {
            if !word.chars().all(|c| c.is_whitespace()) {
                prev_word_starts.push(idx);
            }
        }

        // Find the word start position that is strictly less than offset
        for start in prev_word_starts.iter().rev() {
            if *start < offset {
                return *start;
            }
        }

        0
    }

    /// Find next word end after offset
    pub fn find_next_word_end(&self, offset: usize) -> usize {
        for (idx, word) in self.text.split_word_bound_indices() {
            if idx >= offset && !word.chars().all(|c| c.is_whitespace()) {
                return idx + word.len();
            }
        }
        self.text.len()
    }
}

/// Cursor position information
#[derive(Debug, Clone)]
pub struct CursorPosition {
    /// Original line number (0-indexed)
    pub line: usize,
    /// Column in original line (grapheme offset)
    pub column: usize,
    /// Wrapped line number (0-indexed)
    pub wrapped_line: usize,
    /// Display column (visual width)
    pub display_column: usize,
    /// Byte offset in text
    pub byte_offset: usize,
}

/// Cursor with selection support
pub struct Cursor {
    /// MeasuredText reference
    measured_text: MeasuredText,
    /// Byte offset in text
    offset: usize,
    /// Selection anchor (relative to offset, negative = backward)
    selection: usize,
}

impl Cursor {
    /// Create new cursor at offset 0
    pub fn new(measured_text: MeasuredText) -> Self {
        Self {
            measured_text,
            offset: 0,
            selection: 0,
        }
    }

    /// Create cursor at specific offset
    pub fn at_offset(measured_text: MeasuredText, offset: usize) -> Self {
        let offset = offset.min(measured_text.len());
        Self {
            measured_text,
            offset,
            selection: 0,
        }
    }

    /// Create cursor from text string
    pub fn from_text(text: String, columns: usize, offset: usize) -> Self {
        let mt = MeasuredText::new(text, columns);
        Self::at_offset(mt, offset)
    }

    /// Get current byte offset
    pub fn offset(&self) -> usize {
        self.offset
    }

    /// Get selection anchor
    pub fn selection(&self) -> usize {
        self.selection
    }

    /// Check if there's an active selection
    pub fn has_selection(&self) -> bool {
        self.selection != 0
    }

    /// Get selection range (start, end)
    pub fn selection_range(&self) -> Option<(usize, usize)> {
        if self.selection == 0 {
            return None;
        }
        let start = self.offset.saturating_sub(self.selection);
        let end = self.offset;
        Some((start.min(end), start.max(end)))
    }

    /// Get selected text
    pub fn selected_text(&self) -> Option<&str> {
        self.selection_range().map(|(start, end)| &self.measured_text.text()[start..end])
    }

    /// Get current position
    pub fn position(&self) -> CursorPosition {
        let wrapped_line = self.measured_text.find_wrapped_line_at_offset(self.offset).unwrap_or(0);
        let line = self.measured_text.find_original_line_at_offset(self.offset).unwrap_or(0);

        // Calculate column (grapheme offset in line)
        let line_start = if line < self.measured_text.original_lines().len() {
            self.measured_text.original_lines()[..line]
                .iter()
                .map(|l| l.len() + 1)
                .sum()
        } else {
            0
        };
        let line_text = &self.measured_text.text()[line_start..];
        let line_content = line_text.lines().next().unwrap_or("");
        let column = line_content[..(self.offset - line_start).min(line_content.len())]
            .graphemes(true)
            .count();

        // Display column
        let display_column = self.measured_text.get_display_column_at_offset(self.offset);

        CursorPosition {
            line,
            column,
            wrapped_line,
            display_column,
            byte_offset: self.offset,
        }
    }

    /// Move cursor to previous grapheme
    pub fn move_prev(&mut self) {
        if let Some(prev_g) = self.measured_text.prev_grapheme_at_offset(self.offset) {
            self.offset -= prev_g.len();
        }
        self.selection = 0;
    }

    /// Move cursor to next grapheme
    pub fn move_next(&mut self) {
        if let Some(next_g) = self.measured_text.grapheme_at_offset(self.offset) {
            self.offset += next_g.len();
            // Clamp to end of text
            self.offset = self.offset.min(self.measured_text.len());
        }
        self.selection = 0;
    }

    /// Move cursor to previous word
    pub fn move_prev_word(&mut self) {
        self.offset = self.measured_text.find_prev_word_start(self.offset);
        self.selection = 0;
    }

    /// Move cursor to next word
    pub fn move_next_word(&mut self) {
        self.offset = self.measured_text.find_next_word_end(self.offset);
        self.selection = 0;
    }

    /// Move cursor to line start
    pub fn move_line_start(&mut self) {
        let pos = self.position();
        let lines = self.measured_text.original_lines();

        if pos.line < lines.len() {
            let line_start: usize = lines[..pos.line].iter().map(|l| l.len() + 1).sum();
            self.offset = line_start;
        }
        self.selection = 0;
    }

    /// Move cursor to line end
    pub fn move_line_end(&mut self) {
        let pos = self.position();
        let lines = self.measured_text.original_lines();

        if pos.line < lines.len() {
            let line_start: usize = lines[..pos.line].iter().map(|l| l.len() + 1).sum();
            let line_len = lines[pos.line].len();
            self.offset = line_start + line_len;
        }
        self.selection = 0;
    }

    /// Move cursor up one line
    pub fn move_up(&mut self) {
        let pos = self.position();
        if pos.line > 0 {
            let prev_line = pos.line - 1;
            let lines = self.measured_text.original_lines();

            // Calculate target position
            let prev_line_start: usize = lines[..prev_line].iter().map(|l| l.len() + 1).sum();
            let prev_line_text = &lines[prev_line];

            // Try to maintain display column
            let target_col = pos.display_column;
            let mut current_width = 0;
            let mut target_offset = prev_line_start;

            for g in prev_line_text.graphemes(true) {
                let g_width = g.width();
                if current_width + g_width > target_col {
                    break;
                }
                target_offset += g.len();
                current_width += g_width;
            }

            self.offset = target_offset;
        }
        self.selection = 0;
    }

    /// Move cursor down one line
    pub fn move_down(&mut self) {
        let pos = self.position();
        let lines = self.measured_text.original_lines();

        if pos.line + 1 < lines.len() {
            let next_line = pos.line + 1;

            // Calculate target position
            let next_line_start: usize = lines[..next_line].iter().map(|l| l.len() + 1).sum();
            let next_line_text = &lines[next_line];

            // Try to maintain display column
            let target_col = pos.display_column;
            let mut current_width = 0;
            let mut target_offset = next_line_start;

            for g in next_line_text.graphemes(true) {
                let g_width = g.width();
                if current_width + g_width > target_col {
                    break;
                }
                target_offset += g.len();
                current_width += g_width;
            }

            self.offset = target_offset;
        }
        self.selection = 0;
    }

    /// Move cursor to text start
    pub fn move_start(&mut self) {
        self.offset = 0;
        self.selection = 0;
    }

    /// Move cursor to text end
    pub fn move_end(&mut self) {
        self.offset = self.measured_text.len();
        self.selection = 0;
    }

    /// Kill line from cursor to end (Ctrl+K)
    pub fn kill_line(&mut self, kill_ring: &mut KillRing) -> String {
        let pos = self.position();
        let lines = self.measured_text.original_lines();

        if pos.line < lines.len() {
            let line_start: usize = lines[..pos.line].iter().map(|l| l.len() + 1).sum();
            let line_end = line_start + lines[pos.line].len();

            let killed = self.measured_text.text()[self.offset..line_end].to_string();
            kill_ring.push(killed.clone(), KillDirection::Append);

            // Update internal text (need mutable access)
            // Note: In practice, this would be handled by the TextInput hook
            killed
        } else {
            String::new()
        }
    }

    /// Kill line from start to cursor (Ctrl+U)
    pub fn kill_line_backward(&mut self, kill_ring: &mut KillRing) -> String {
        let pos = self.position();
        let lines = self.measured_text.original_lines();

        if pos.line < lines.len() {
            let line_start: usize = lines[..pos.line].iter().map(|l| l.len() + 1).sum();

            let killed = self.measured_text.text()[line_start..self.offset].to_string();
            kill_ring.push(killed.clone(), KillDirection::Prepend);

            killed
        } else {
            String::new()
        }
    }

    /// Kill word backward (Ctrl+W)
    pub fn kill_word_backward(&mut self, kill_ring: &mut KillRing) -> String {
        let word_start = self.measured_text.find_prev_word_start(self.offset);

        let killed = self.measured_text.text()[word_start..self.offset].to_string();
        kill_ring.push(killed.clone(), KillDirection::Prepend);

        killed
    }

    /// Yank from kill ring (Ctrl+Y)
    pub fn yank(&mut self, kill_ring: &KillRing) -> Option<String> {
        kill_ring.get_last().cloned()
    }

    /// Yank pop - cycle kill ring (Alt+Y)
    pub fn yank_pop(&mut self, kill_ring: &mut KillRing) -> Option<String> {
        kill_ring.yank_pop().map(|(text, _, _)| text)
    }

    /// Set selection anchor
    pub fn start_selection(&mut self) {
        self.selection = 0;
    }

    /// Extend selection in direction
    pub fn extend_selection(&mut self, direction: SelectionDirection) {
        match direction {
            SelectionDirection::Backward => {
                if self.offset > 0 {
                    if let Some(prev_g) = self.measured_text.prev_grapheme_at_offset(self.offset) {
                        self.offset -= prev_g.len();
                        self.selection += prev_g.len();
                    }
                }
            }
            SelectionDirection::Forward => {
                if let Some(next_g) = self.measured_text.grapheme_at_offset(self.offset) {
                    self.offset += next_g.len();
                    self.selection += next_g.len();
                }
            }
        }
    }

    /// Clear selection
    pub fn clear_selection(&mut self) {
        self.selection = 0;
    }

    /// Update measured text (after edit)
    pub fn update_text(&mut self, text: String) {
        let columns = self.measured_text.columns();
        self.measured_text = MeasuredText::new(text, columns);
        self.offset = self.offset.min(self.measured_text.len());
        self.selection = 0;
    }

    /// Get display width at cursor
    pub fn display_width(&self) -> usize {
        self.position().display_column
    }

    /// Get wrapped line index
    pub fn wrapped_line(&self) -> usize {
        self.position().wrapped_line
    }

    /// Get viewport start line for max visible lines
    pub fn get_viewport_start_line(&self, max_visible_lines: usize) -> usize {
        if max_visible_lines == 0 {
            return 0;
        }

        let wrapped_line = self.wrapped_line();
        let total_lines = self.measured_text.wrapped_line_count();

        if total_lines <= max_visible_lines {
            return 0;
        }

        let half = max_visible_lines / 2;
        let start = wrapped_line.saturating_sub(half);
        let end = (start + max_visible_lines).min(total_lines);

        if end - start < max_visible_lines {
            return total_lines.saturating_sub(max_visible_lines);
        }

        start
    }

    /// Clone with new measured text
    pub fn clone_with_text(&self, text: String) -> Self {
        let columns = self.measured_text.columns();
        let mt = MeasuredText::new(text, columns);
        let len = mt.len();
        Self::at_offset(mt, self.offset.min(len))
    }
}

/// Selection direction
pub enum SelectionDirection {
    Backward,
    Forward,
}

/// Double press detector
///
/// Detects double press within timeout duration.
/// Used for Ctrl+C/Ctrl+D double press to exit and Escape double press to clear.
pub struct DoublePressDetector {
    last_press: Option<Instant>,
    timeout: Duration,
}

impl DoublePressDetector {
    /// Create new detector with timeout
    pub fn new(timeout: Duration) -> Self {
        Self {
            last_press: None,
            timeout,
        }
    }

    /// Check if this is a double press
    ///
    /// Returns true if this press is within timeout of the last press.
    /// Resets the timer after returning true.
    pub fn check(&mut self) -> bool {
        let now = Instant::now();

        if let Some(last) = self.last_press {
            if now.duration_since(last) < self.timeout {
                self.last_press = None;
                return true;
            }
        }

        self.last_press = Some(now);
        false
    }

    /// Reset the detector
    pub fn reset(&mut self) {
        self.last_press = None;
    }

    /// Check if pending (first press happened, waiting for second)
    pub fn is_pending(&self) -> bool {
        self.last_press.is_some()
    }
}

impl Default for DoublePressDetector {
    fn default() -> Self {
        Self::new(Duration::from_secs(2))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_measured_text_nfc_normalization() {
        // Test NFC normalization
        let mt = MeasuredText::new("cafe\u{301}".to_string(), 80); // café with combining accent
        // Should be normalized to single character
        assert!(mt.text().contains('é'));
    }

    #[test]
    fn test_measured_text_grapheme_count() {
        let mt = MeasuredText::new("你好世界".to_string(), 80);
        assert_eq!(mt.grapheme_count(), 4);

        // Emoji
        let mt2 = MeasuredText::new("👨‍👩‍👧‍👦".to_string(), 80);
        assert_eq!(mt2.grapheme_count(), 1);
    }

    #[test]
    fn test_measured_text_wrapping() {
        let mt = MeasuredText::new("hello world".to_string(), 5);

        // Should wrap into multiple lines
        assert!(mt.wrapped_line_count() > 1);
    }

    #[test]
    fn test_cursor_movement_unicode() {
        let mt = MeasuredText::new("a你b好c".to_string(), 80);
        let mut cursor = Cursor::new(mt);

        // Move through graphemes
        cursor.move_next(); // 'a'
        assert_eq!(cursor.offset(), 1);

        cursor.move_next(); // '你' (3 bytes)
        assert_eq!(cursor.offset(), 4);

        cursor.move_next(); // 'b'
        assert_eq!(cursor.offset(), 5);

        // Move back
        cursor.move_prev();
        assert_eq!(cursor.offset(), 4);
    }

    #[test]
    fn test_cursor_word_boundaries() {
        let mt = MeasuredText::new("hello world test".to_string(), 80);
        let mut cursor = Cursor::at_offset(mt, 7); // In "world"

        cursor.move_prev_word();
        assert_eq!(cursor.offset(), 6); // Start of "world"

        cursor.move_prev_word();
        assert_eq!(cursor.offset(), 0); // Start of "hello"
    }

    #[test]
    fn test_kill_ring() {
        let mut kr = KillRing::new(10);

        kr.push("hello".to_string(), KillDirection::Append);
        assert_eq!(kr.get_last(), Some(&"hello".to_string()));

        kr.push("world".to_string(), KillDirection::Append);
        assert_eq!(kr.get_last(), Some(&"helloworld".to_string()));

        kr.reset_accumulation();
        kr.push("new".to_string(), KillDirection::Append);
        assert_eq!(kr.get_last(), Some(&"new".to_string()));
    }

    #[test]
    fn test_kill_ring_yank_pop() {
        let mut kr = KillRing::new(10);

        kr.push("first".to_string(), KillDirection::Append);
        kr.reset_accumulation();
        kr.push("second".to_string(), KillDirection::Append);
        kr.reset_accumulation();

        kr.record_yank(0, 5);
        assert!(kr.can_yank_pop());

        let pop = kr.yank_pop();
        assert_eq!(pop.unwrap().0, "first".to_string());
    }

    #[test]
    fn test_double_press_detector() {
        let mut dp = DoublePressDetector::new(Duration::from_millis(500));

        // First press
        assert!(!dp.check());

        // Second press within timeout
        std::thread::sleep(Duration::from_millis(100));
        assert!(dp.check());

        // After true, should reset
        assert!(!dp.is_pending());
    }

    #[test]
    fn test_double_press_timeout_expired() {
        let mut dp = DoublePressDetector::new(Duration::from_millis(100));

        // First press
        assert!(!dp.check());

        // Wait for timeout
        std::thread::sleep(Duration::from_millis(150));

        // Should not trigger double press
        assert!(!dp.check());
    }

    #[test]
    fn test_cursor_display_width_cjk() {
        let mt = MeasuredText::new("你好".to_string(), 80);
        let mut cursor = Cursor::new(mt);

        cursor.move_next();
        cursor.move_next();

        // Each Chinese char has display width 2
        assert_eq!(cursor.display_width(), 4);
    }

    #[test]
    fn test_cursor_line_movement() {
        let mt = MeasuredText::new("line1\nline2\nline3".to_string(), 80);
        let mut cursor = Cursor::at_offset(mt, 10); // On line2

        cursor.move_up();
        assert_eq!(cursor.position().line, 0);

        cursor.move_down();
        cursor.move_down();
        assert_eq!(cursor.position().line, 2);
    }
}
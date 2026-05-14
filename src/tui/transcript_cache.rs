//! Cached transcript rendering for the TUI.
//!
//! ## Per-cell revision caching
//!
//! Naive caching invalidates the whole transcript whenever ANY cell mutates.
//! During streaming the assistant content cell mutates on every delta — that
//! would force a re-wrap of every cell on every chunk. We avoid this by
//! tracking a per-cell revision counter.
//!
//! Each cell index has a paired `revision: u64`. The cache stores
//! `Vec<CachedCell>` with `(cell_index, revision, lines)`. On
//! `ensure`, walk the cells; if a cell's current `revision` matches the cached
//! one (and width/options haven't changed), reuse the rendered lines.
//! Otherwise re-render that cell only and reassemble.

use ratatui::text::Line;
use std::sync::Arc;

/// Per-cell cached render output. Reused across `ensure` calls when the
/// upstream cell's revision counter hasn't changed.
///
/// Lines are stored behind an `Arc` so that cloning a `CachedCell` during
/// cache-ensure is O(1) rather than O(rendered_line_count).
#[derive(Debug, Clone)]
pub struct CachedCell {
    /// Revision the cell was at when the lines were rendered.
    pub revision: u64,
    /// Rendered lines for this cell, shared via Arc for cheap clone.
    pub lines: Arc<Vec<Line<'static>>>,
    /// Whether this cell's rendered output was empty.
    pub is_empty: bool,
}

/// Cache of rendered transcript lines for the current viewport.
#[derive(Debug)]
pub struct TranscriptViewCache {
    /// Width used for last render.
    width: u16,
    /// Per-cell rendered output, indexed by current cell position.
    per_cell: Vec<CachedCell>,
    /// Flattened lines reassembled from `per_cell` plus spacers.
    lines: Vec<Line<'static>>,
}

impl TranscriptViewCache {
    /// Create an empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self {
            width: 0,
            per_cell: Vec::new(),
            lines: Vec::new(),
        }
    }

    /// Ensure cached lines match the provided cells and per-cell revisions.
    ///
    /// Reuses rendered lines for cells whose `cell_revisions[i]` matches the
    /// previously cached revision. Width changes bust the entire cache.
    ///
    /// # Arguments
    /// * `cells` - Renderable cell lines (each cell returns Vec<Line>)
    /// * `cell_revisions` - Revision counters for each cell
    /// * `width` - Terminal width for wrapping
    pub fn ensure(
        &mut self,
        cells: &[Vec<Line<'static>>],
        cell_revisions: &[u64],
        width: u16,
    ) {
        let layout_changed = self.width != width;
        if layout_changed {
            self.per_cell.clear();
        }
        self.width = width;

        // Track whether anything actually changed
        let old_len = self.per_cell.len();
        let mut any_dirty = layout_changed || old_len != cells.len();
        let mut first_dirty: Option<usize> = if old_len != cells.len() {
            Some(old_len.min(cells.len()))
        } else {
            None
        };

        let mut new_per_cell: Vec<CachedCell> = Vec::with_capacity(cells.len());
        let revisions_match = cell_revisions.len() == cells.len();

        for (idx, cell_lines) in cells.iter().enumerate() {
            let current_rev = if revisions_match {
                cell_revisions[idx]
            } else {
                u64::MAX // No matching revisions — force re-render
            };

            // Reuse cached entry if revision matches and at same index
            // Rust 2021 compatible: use nested if instead of let chains
            let should_reuse = if !layout_changed && revisions_match {
                self.per_cell.get(idx)
                    .map(|prev| prev.revision == current_rev)
                    .unwrap_or(false)
            } else {
                false
            };

            if should_reuse {
                new_per_cell.push(self.per_cell[idx].clone());
                continue;
            }

            any_dirty = true;
            first_dirty = Some(first_dirty.map_or(idx, |current| current.min(idx)));
            let is_empty = cell_lines.is_empty();
            new_per_cell.push(CachedCell {
                revision: current_rev,
                lines: Arc::new(cell_lines.clone()),
                is_empty,
            });
        }

        self.per_cell = new_per_cell;

        // Reassemble flattened lines if anything changed
        if any_dirty {
            self.reflatten(first_dirty);
        }
    }

    /// Reassemble flattened lines from per_cell cache.
    fn reflatten(&mut self, first_dirty: Option<usize>) {
        // If we know the first dirty index, only reflatten from there
        let start = first_dirty.unwrap_or(0);

        if start == 0 {
            // Full reflatten
            self.lines.clear();
            for cached in &self.per_cell {
                if !cached.is_empty {
                    self.lines.extend(cached.lines.iter().cloned());
                }
            }
        } else {
            // Partial reflatten: truncate to first dirty, then append
            self.lines.truncate(
                self.per_cell[..start]
                    .iter()
                    .filter(|c| !c.is_empty)
                    .map(|c| c.lines.len())
                    .sum(),
            );
            for cached in &self.per_cell[start..] {
                if !cached.is_empty {
                    self.lines.extend(cached.lines.iter().cloned());
                }
            }
        }
    }

    /// Get the total number of rendered lines.
    #[must_use]
    pub fn total_lines(&self) -> usize {
        self.lines.len()
    }

    /// Get the cached lines (for rendering).
    #[must_use]
    pub fn lines(&self) -> &[Line<'static>] {
        &self.lines
    }

    /// Get a slice of lines for the visible viewport.
    #[must_use]
    pub fn visible_lines(&self, top: usize, visible_count: usize) -> Vec<Line<'static>> {
        let end = (top + visible_count).min(self.lines.len());
        if top >= self.lines.len() {
            Vec::new()
        } else {
            self.lines[top..end].to_vec()
        }
    }

    /// Clear the cache.
    pub fn clear(&mut self) {
        self.per_cell.clear();
        self.lines.clear();
    }
}

impl Default for TranscriptViewCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_lines(content: &str) -> Vec<Line<'static>> {
        content.lines().map(|l| Line::from(l.to_string())).collect()
    }

    #[test]
    fn test_empty_cache() {
        let cache = TranscriptViewCache::new();
        assert_eq!(cache.total_lines(), 0);
    }

    #[test]
    fn test_single_cell() {
        let mut cache = TranscriptViewCache::new();
        let cells = vec![make_lines("Hello World")];
        let revisions = vec![1];

        cache.ensure(&cells, &revisions, 80);

        assert_eq!(cache.total_lines(), 1);
        assert_eq!(cache.lines()[0].spans.len(), 1);
    }

    #[test]
    fn test_multiple_cells() {
        let mut cache = TranscriptViewCache::new();
        let cells = vec![
            make_lines("User message"),
            make_lines("Assistant reply"),
        ];
        let revisions = vec![1, 1];

        cache.ensure(&cells, &revisions, 80);

        assert_eq!(cache.total_lines(), 2);
    }

    #[test]
    fn test_reuse_on_same_revision() {
        let mut cache = TranscriptViewCache::new();

        // First render
        let cells = vec![make_lines("Hello")];
        let revisions = vec![1];
        cache.ensure(&cells, &revisions, 80);

        // Get the Arc pointer from first render
        let first_arc = cache.per_cell[0].lines.clone();

        // Second render with same revision - should reuse
        let cells2 = vec![make_lines("Hello")];
        let revisions2 = vec![1]; // Same revision
        cache.ensure(&cells2, &revisions2, 80);

        // The per_cell Arc should be ptr_eq (same pointer reused)
        assert!(Arc::ptr_eq(&cache.per_cell[0].lines, &first_arc));
    }

    #[test]
    fn test_dirty_on_revision_change() {
        let mut cache = TranscriptViewCache::new();

        // First render
        let cells = vec![make_lines("Hello")];
        let revisions = vec![1];
        cache.ensure(&cells, &revisions, 80);

        // Second render with different revision - should re-render
        let cells2 = vec![make_lines("Hello World")];
        let revisions2 = vec![2]; // Changed revision
        cache.ensure(&cells2, &revisions2, 80);

        assert_eq!(cache.total_lines(), 1);
        // Content should be updated
        let line = &cache.lines()[0];
        let span = &line.spans[0];
        assert_eq!(span.content, "Hello World");
    }

    #[test]
    fn test_width_change_busts_cache() {
        let mut cache = TranscriptViewCache::new();

        // First render at width 80
        let cells = vec![make_lines("Hello")];
        let revisions = vec![1];
        cache.ensure(&cells, &revisions, 80);

        // Width change should bust entire cache
        let cells2 = vec![make_lines("Hello")];
        let revisions2 = vec![1]; // Same revision
        cache.ensure(&cells2, &revisions2, 40); // Different width

        // Cache was cleared and rebuilt
        assert_eq!(cache.width, 40);
    }

    #[test]
    fn test_empty_cell_skipped() {
        let mut cache = TranscriptViewCache::new();
        let cells = vec![
            make_lines("Hello"),
            Vec::new(), // Empty cell
            make_lines("World"),
        ];
        let revisions = vec![1, 1, 1];

        cache.ensure(&cells, &revisions, 80);

        // Empty cell should be skipped in flattening
        assert_eq!(cache.total_lines(), 2);
    }

    #[test]
    fn test_visible_lines_slice() {
        let mut cache = TranscriptViewCache::new();
        let cells = vec![make_lines("Line1\nLine2\nLine3\nLine4\nLine5")];
        let revisions = vec![1];
        cache.ensure(&cells, &revisions, 80);

        assert_eq!(cache.total_lines(), 5);

        let visible = cache.visible_lines(1, 3);
        assert_eq!(visible.len(), 3);
    }
}
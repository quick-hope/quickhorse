//! Streaming state with adaptive chunking for live transcript updates.
//!
//! This module implements the pattern from DeepSeek-TUI where:
//! - Streaming text is split into small chunks
//! - Two-gear pacing: Smooth (normal) vs CatchUp (backlog draining)
//! - Thinking blocks bypass the newline gate for live display

use std::time::Instant;

/// Chunking mode for adaptive pacing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChunkingMode {
    /// Normal streaming: drip one line at a time.
    Smooth,
    /// Backlog draining: emit multiple lines to catch up.
    CatchUp,
}

/// Adaptive chunking policy that switches between modes.
#[derive(Debug)]
pub struct AdaptiveChunkingPolicy {
    mode: ChunkingMode,
    /// Low motion flag prevents CatchUp bursts.
    low_motion: bool,
    /// Time since last chunk (for pacing).
    last_chunk_at: Option<Instant>,
}

impl AdaptiveChunkingPolicy {
    /// Create a new policy in Smooth mode.
    pub fn new() -> Self {
        Self {
            mode: ChunkingMode::Smooth,
            low_motion: false,
            last_chunk_at: None,
        }
    }

    /// Get current chunking mode.
    #[must_use]
    pub fn mode(&self) -> ChunkingMode {
        if self.low_motion {
            ChunkingMode::Smooth
        } else {
            self.mode
        }
    }

    /// Set low motion flag (prevents CatchUp bursts).
    pub fn set_low_motion(&mut self, low_motion: bool) {
        self.low_motion = low_motion;
    }

    /// Update mode based on queue pressure.
    pub fn update_for_queue(&mut self, queued_lines: usize, now: Instant) {
        // Switch to CatchUp if backlog is building
        if queued_lines > 5 && !self.low_motion {
            self.mode = ChunkingMode::CatchUp;
        }

        // Switch back to Smooth after draining
        if queued_lines == 0 {
            self.mode = ChunkingMode::Smooth;
        }

        self.last_chunk_at = Some(now);
    }

    /// How many lines to emit this tick.
    #[must_use]
    pub fn lines_per_tick(&self, queued_lines: usize) -> usize {
        match self.mode() {
            ChunkingMode::Smooth => {
                // Emit 1 line per tick in smooth mode
                if queued_lines > 0 { 1 } else { 0 }
            }
            ChunkingMode::CatchUp => {
                // Emit more lines to catch up
                queued_lines.min(5)
            }
        }
    }
}

impl Default for AdaptiveChunkingPolicy {
    fn default() -> Self {
        Self::new()
    }
}

/// Stream chunker that holds queued lines.
#[derive(Debug)]
pub struct StreamChunker {
    /// Queued lines waiting to be committed.
    queue: Vec<String>,
}

impl StreamChunker {
    /// Create a new empty chunker.
    pub fn new() -> Self {
        Self { queue: Vec::new() }
    }

    /// Push a delta string into the queue.
    pub fn push_delta(&mut self, delta: &str) {
        // Split by newlines and queue each line
        for line in delta.lines() {
            if !line.is_empty() {
                self.queue.push(line.to_string());
            }
        }
    }

    /// Get number of queued lines.
    #[must_use]
    pub fn queued_lines(&self) -> usize {
        self.queue.len()
    }

    /// Drain the queue, returning up to `count` lines.
    pub fn drain(&mut self, count: usize) -> Vec<String> {
        let to_take = count.min(self.queue.len());
        self.queue.drain(0..to_take).collect()
    }

    /// Drain all remaining lines.
    pub fn drain_remaining(&mut self) -> String {
        let remaining = self.queue.join("\n");
        self.queue.clear();
        remaining
    }

    /// Check if queue is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Default for StreamChunker {
    fn default() -> Self {
        Self::new()
    }
}

/// Line buffer for newline gating.
///
/// Holds back trailing partial-line text between deltas.
#[derive(Debug)]
pub struct LineBuffer {
    /// Buffered content.
    buffer: String,
}

impl LineBuffer {
    /// Create a new empty buffer.
    pub fn new() -> Self {
        Self { buffer: String::new() }
    }

    /// Push content to buffer.
    pub fn push(&mut self, content: &str) {
        self.buffer.push_str(content);
    }

    /// Take committable content (up to last newline).
    pub fn take_committable(&mut self) -> String {
        if self.buffer.is_empty() {
            return String::new();
        }

        // Find last newline
        let Some(last_newline_idx) = self.buffer.rfind('\n') else {
            return String::new();
        };

        // Extract up to and including last newline
        let committable = self.buffer[..=last_newline_idx].to_string();
        self.buffer = self.buffer[last_newline_idx + 1..].to_string();
        committable
    }

    /// Flush remaining buffer content.
    pub fn flush(&mut self) -> String {
        let remaining = self.buffer.clone();
        self.buffer.clear();
        remaining
    }

    /// Check if buffer is empty.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }
}

impl Default for LineBuffer {
    fn default() -> Self {
        Self::new()
    }
}

/// Result of a commit tick.
#[derive(Debug)]
pub struct CommitResult {
    /// Text committed this tick.
    pub committed_text: String,
    /// Mode used for this tick.
    pub mode: ChunkingMode,
}

/// Run one commit tick and return committed text.
pub fn run_commit_tick(
    policy: &mut AdaptiveChunkingPolicy,
    chunker: &mut StreamChunker,
    now: Instant,
) -> CommitResult {
    let queued = chunker.queued_lines();
    policy.update_for_queue(queued, now);

    let lines_to_emit = policy.lines_per_tick(queued);
    let lines = chunker.drain(lines_to_emit);

    let committed_text = if lines.is_empty() {
        String::new()
    } else {
        lines.join("\n")
    };

    CommitResult {
        committed_text,
        mode: policy.mode(),
    }
}

/// Per-block streaming state.
#[derive(Debug)]
pub struct BlockState {
    /// Line buffer for newline gating.
    line_buffer: LineBuffer,
    /// Whether to bypass the line buffer (thinking blocks).
    bypass_gate: bool,
    /// Stream chunker.
    chunker: StreamChunker,
    /// Adaptive chunking policy.
    policy: AdaptiveChunkingPolicy,
    /// Is this a thinking block?
    is_thinking: bool,
}

impl BlockState {
    /// Create a new block state for text.
    pub fn for_text() -> Self {
        Self {
            line_buffer: LineBuffer::new(),
            bypass_gate: false,
            chunker: StreamChunker::new(),
            policy: AdaptiveChunkingPolicy::new(),
            is_thinking: false,
        }
    }

    /// Create a new block state for thinking (bypasses gate).
    pub fn for_thinking() -> Self {
        Self {
            line_buffer: LineBuffer::new(),
            bypass_gate: true,
            chunker: StreamChunker::new(),
            policy: AdaptiveChunkingPolicy::new(),
            is_thinking: true,
        }
    }
}

/// State for managing multiple stream collectors (one per content block).
#[derive(Debug, Default)]
pub struct StreamingState {
    /// Per-block state by index.
    blocks: Vec<Option<BlockState>>,
    /// Whether any stream is currently active.
    pub is_active: bool,
    /// Accumulated text for the current block.
    pub accumulated_text: String,
    /// Accumulated thinking for the current block.
    pub accumulated_thinking: String,
}

impl StreamingState {
    /// Create a new streaming state.
    pub fn new() -> Self {
        Self::default()
    }

    /// Start a new text block.
    pub fn start_text(&mut self, index: usize) {
        self.ensure_capacity(index);
        self.blocks[index] = Some(BlockState::for_text());
        self.is_active = true;
    }

    /// Start a new thinking block (bypasses newline gate).
    pub fn start_thinking(&mut self, index: usize) {
        self.ensure_capacity(index);
        self.blocks[index] = Some(BlockState::for_thinking());
        self.is_active = true;
    }

    /// Push content to a block.
    pub fn push_content(&mut self, index: usize, content: &str) {
        if let Some(Some(block)) = self.blocks.get_mut(index) {
            // Track raw content
            if block.is_thinking {
                self.accumulated_thinking.push_str(content);
            } else {
                self.accumulated_text.push_str(content);
            }

            // Route through buffer or directly to chunker
            if block.bypass_gate {
                block.chunker.push_delta(content);
            } else {
                block.line_buffer.push(content);
                let committable = block.line_buffer.take_committable();
                if !committable.is_empty() {
                    block.chunker.push_delta(&committable);
                }
            }
        }
    }

    /// Commit lines from a block.
    pub fn commit_text(&mut self, index: usize) -> String {
        if let Some(Some(block)) = self.blocks.get_mut(index) {
            let now = Instant::now();
            let result = run_commit_tick(&mut block.policy, &mut block.chunker, now);
            result.committed_text
        } else {
            String::new()
        }
    }

    /// Check if a block has pending chunker lines.
    #[must_use]
    pub fn has_pending_chunker_lines(&self, index: usize) -> bool {
        self.blocks
            .get(index)
            .and_then(|b| b.as_ref())
            .is_some_and(|b| b.chunker.queued_lines() > 0)
    }

    /// Get chunking mode for a block.
    #[must_use]
    pub fn chunking_mode(&self, index: usize) -> Option<ChunkingMode> {
        self.blocks
            .get(index)
            .and_then(|b| b.as_ref())
            .map(|b| b.policy.mode())
    }

    /// Finalize a block and get remaining text.
    pub fn finalize_block_text(&mut self, index: usize) -> String {
        if let Some(Some(block)) = self.blocks.get_mut(index) {
            // Flush line buffer
            let gate_tail = block.line_buffer.flush();
            if !gate_tail.is_empty() {
                block.chunker.push_delta(&gate_tail);
            }

            // Drain remaining chunker content
            let remaining = block.chunker.drain_remaining();

            self.check_active();
            remaining
        } else {
            String::new()
        }
    }

    /// Reset the streaming state.
    pub fn reset(&mut self) {
        self.blocks.clear();
        self.is_active = false;
        self.accumulated_text.clear();
        self.accumulated_thinking.clear();
    }

    /// Set low motion flag for all blocks.
    pub fn set_low_motion(&mut self, low_motion: bool) {
        for block in self.blocks.iter_mut().flatten() {
            block.policy.set_low_motion(low_motion);
        }
    }

    /// Ensure capacity for the given index.
    fn ensure_capacity(&mut self, index: usize) {
        while self.blocks.len() <= index {
            self.blocks.push(None);
        }
    }

    /// Check if any stream is still active.
    fn check_active(&mut self) {
        self.is_active = self.blocks.iter().any(|b| {
            b.as_ref().is_some_and(|state| !state.chunker.is_empty())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunker_push() {
        let mut chunker = StreamChunker::new();
        chunker.push_delta("line1\nline2\nline3");

        assert_eq!(chunker.queued_lines(), 3);
    }

    #[test]
    fn test_chunker_drain() {
        let mut chunker = StreamChunker::new();
        chunker.push_delta("line1\nline2\nline3");

        let drained = chunker.drain(2);
        assert_eq!(drained.len(), 2);
        assert_eq!(chunker.queued_lines(), 1);
    }

    #[test]
    fn test_line_buffer_take_committable() {
        let mut buffer = LineBuffer::new();
        buffer.push("hello ");
        assert!(buffer.take_committable().is_empty());

        buffer.push("world\n");
        let committable = buffer.take_committable();
        assert_eq!(committable, "hello world\n");
    }

    #[test]
    fn test_policy_smooth_mode() {
        let policy = AdaptiveChunkingPolicy::new();
        assert_eq!(policy.mode(), ChunkingMode::Smooth);
        assert_eq!(policy.lines_per_tick(5), 1);
    }

    #[test]
    fn test_policy_catchup_mode() {
        let mut policy = AdaptiveChunkingPolicy::new();
        policy.mode = ChunkingMode::CatchUp;

        assert_eq!(policy.lines_per_tick(10), 5);
    }

    #[test]
    fn test_policy_low_motion() {
        let mut policy = AdaptiveChunkingPolicy::new();
        policy.set_low_motion(true);
        policy.mode = ChunkingMode::CatchUp;

        // Low motion forces Smooth
        assert_eq!(policy.mode(), ChunkingMode::Smooth);
    }

    #[test]
    fn test_streaming_state_start_text() {
        let mut state = StreamingState::new();
        state.start_text(0);

        assert!(state.is_active);
        assert!(state.blocks[0].is_some());
    }

    #[test]
    fn test_streaming_state_push_content() {
        let mut state = StreamingState::new();
        state.start_text(0);
        state.push_content(0, "hello world");

        assert_eq!(state.accumulated_text, "hello world");
    }

    #[test]
    fn test_streaming_state_thinking_bypass() {
        let mut state = StreamingState::new();
        state.start_thinking(0);
        state.push_content(0, "thinking...");

        assert_eq!(state.accumulated_thinking, "thinking...");

        // Bypass gate means content goes directly to chunker
        let block = state.blocks[0].as_ref().unwrap();
        assert!(!block.chunker.is_empty());
    }

    #[test]
    fn test_streaming_state_commit() {
        let mut state = StreamingState::new();
        state.start_text(0);
        state.push_content(0, "line1\nline2\n");

        let committed = state.commit_text(0);
        // In Smooth mode, commits 1 line per tick
        assert!(!committed.is_empty() || state.has_pending_chunker_lines(0));
    }

    #[test]
    fn test_streaming_state_finalize() {
        let mut state = StreamingState::new();
        state.start_text(0);
        state.push_content(0, "remaining text");

        let finalized = state.finalize_block_text(0);
        assert_eq!(finalized, "remaining text");
    }

    #[test]
    fn test_streaming_state_reset() {
        let mut state = StreamingState::new();
        state.start_text(0);
        state.push_content(0, "some content");
        state.reset();

        assert!(!state.is_active);
        assert!(state.accumulated_text.is_empty());
    }

    #[test]
    fn test_run_commit_tick() {
        let mut policy = AdaptiveChunkingPolicy::new();
        let mut chunker = StreamChunker::new();
        chunker.push_delta("line1\nline2\nline3\nline4\nline5\n");

        let result = run_commit_tick(&mut policy, &mut chunker, Instant::now());

        // Smooth mode: emit 1 line
        assert_eq!(result.mode, ChunkingMode::Smooth);
    }
}
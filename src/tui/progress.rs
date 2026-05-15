//! Progress indicator components - Spinner, ProgressBar, ToolProgress
//!
//! Based on OpenClaude's Spinner.tsx and ProgressBar.tsx

#![allow(dead_code)] // Future use: progress UI integration

use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};
use std::time::{Duration, Instant};

/// Spinner frame characters (Unicode box drawing)
const SPINNER_FRAMES: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Progress bar blocks (like OpenClaude's ProgressBar)
const PROGRESS_BLOCKS: &[char] = &[' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];

/// Spinner verbs for loading messages
const SPINNER_VERBS: &[&str] = &[
    "Thinking",
    "Processing",
    "Computing",
    "Calculating",
    "Analyzing",
    "Generating",
    "Crafting",
    "Working",
    "Loading",
    "Preparing",
    "Executing",
    "Building",
    "Compiling",
    "Reading",
    "Writing",
    "Searching",
    "Fetching",
    "Parsing",
    "Transforming",
    "Optimizing",
];

/// Spinner state for animation
pub struct Spinner {
    /// Current frame index
    frame: usize,
    /// Start time
    start_time: Instant,
    /// Last update time
    last_update: Instant,
    /// Animation interval (ms)
    interval: Duration,
    /// Random verb
    verb: String,
}

impl Spinner {
    /// Create new spinner with random verb
    pub fn new() -> Self {
        Self::with_verb(Self::random_verb())
    }

    /// Create spinner with specific verb
    pub fn with_verb(verb: String) -> Self {
        Self {
            frame: 0,
            start_time: Instant::now(),
            last_update: Instant::now(),
            interval: Duration::from_millis(80),
            verb,
        }
    }

    /// Get a random spinner verb
    fn random_verb() -> String {
        use std::sync::atomic::{AtomicUsize, Ordering};
        static COUNTER: AtomicUsize = AtomicUsize::new(0);

        // Simple pseudo-random selection
        let idx = COUNTER.fetch_add(1, Ordering::Relaxed) % SPINNER_VERBS.len();
        SPINNER_VERBS[idx].to_string()
    }

    /// Update frame (returns true if frame changed)
    pub fn tick(&mut self) -> bool {
        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.interval {
            self.frame = (self.frame + 1) % SPINNER_FRAMES.len();
            self.last_update = now;
            true
        } else {
            false
        }
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get current frame character
    pub fn frame_char(&self) -> char {
        SPINNER_FRAMES[self.frame]
    }

    /// Get verb with ellipsis
    pub fn message(&self) -> String {
        format!("{}…", self.verb)
    }

    /// Set custom verb
    pub fn set_verb(&mut self, verb: String) {
        self.verb = verb;
    }

    /// Render as Line
    pub fn render(&self) -> Line<'static> {
        let elapsed_secs = self.elapsed().as_secs();
        let time_str = if elapsed_secs < 60 {
            format!("{}s", elapsed_secs)
        } else {
            format!("{}m{}s", elapsed_secs / 60, elapsed_secs % 60)
        };

        Line::from(vec![
            Span::styled(self.frame_char().to_string(), Style::default().fg(Color::Cyan)),
            Span::raw(" "),
            Span::styled(self.message(), Style::default().fg(Color::Yellow)),
            Span::raw(" "),
            Span::styled(format!("({})", time_str), Style::default().fg(Color::Gray)),
        ])
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}

/// Progress bar for showing completion ratio
pub struct ProgressBar {
    /// Progress ratio (0.0 - 1.0)
    ratio: f32,
    /// Width in characters
    width: usize,
    /// Fill color
    fill_color: Color,
    /// Empty color
    empty_color: Color,
}

impl ProgressBar {
    /// Create new progress bar
    pub fn new(ratio: f32, width: usize) -> Self {
        Self {
            ratio: ratio.clamp(0.0, 1.0),
            width,
            fill_color: Color::Green,
            empty_color: Color::Gray,
        }
    }

    /// Set colors
    pub fn with_colors(mut self, fill: Color, empty: Color) -> Self {
        self.fill_color = fill;
        self.empty_color = empty;
        self
    }

    /// Update progress
    pub fn set_ratio(&mut self, ratio: f32) {
        self.ratio = ratio.clamp(0.0, 1.0);
    }

    /// Render progress bar string
    fn render_string(&self) -> String {
        let filled = (self.ratio * self.width as f32) as usize;
        let remainder = self.ratio * self.width as f32 - filled as f32;
        let middle = (remainder * PROGRESS_BLOCKS.len() as f32) as usize;
        let empty = self.width.saturating_sub(filled).saturating_sub(1);

        let mut result = String::new();

        // Filled portion (full blocks)
        if filled > 0 {
            result.push_str(&PROGRESS_BLOCKS[PROGRESS_BLOCKS.len() - 1].to_string().repeat(filled));
        }

        // Middle block (partial) - only if not fully filled
        if filled < self.width && self.ratio < 1.0 {
            result.push(PROGRESS_BLOCKS[middle.min(PROGRESS_BLOCKS.len() - 1)]);
        }

        // Empty portion
        if empty > 0 && self.ratio < 1.0 {
            result.push_str(&PROGRESS_BLOCKS[0].to_string().repeat(empty));
        }

        result
    }

    /// Render as Line
    pub fn render(&self) -> Line<'static> {
        let bar_str = self.render_string();
        let pct = (self.ratio * 100.0) as usize;

        Line::from(vec![
            Span::styled(bar_str, Style::default().fg(self.fill_color).bg(self.empty_color)),
            Span::raw(" "),
            Span::styled(format!("{}%", pct), Style::default().fg(Color::White)),
        ])
    }
}

/// Tool execution progress state
pub struct ToolProgress {
    /// Tool name
    tool_name: String,
    /// Tool description/parameters summary
    description: String,
    /// Execution status
    status: ToolStatus,
    /// Start time
    start_time: Instant,
    /// Spinner for animation
    spinner: Spinner,
}

/// Tool execution status
#[derive(Debug, Clone, PartialEq, Copy)]
pub enum ToolStatus {
    /// Tool is starting
    Starting,
    /// Tool is executing
    Executing,
    /// Tool is waiting for user confirmation
    WaitingConfirmation,
    /// Tool completed successfully
    Success,
    /// Tool failed with error
    Error,
    /// Tool was cancelled
    Cancelled,
}

impl ToolProgress {
    /// Create new tool progress
    pub fn new(tool_name: String, description: String) -> Self {
        Self {
            tool_name: tool_name.clone(),
            description,
            status: ToolStatus::Starting,
            start_time: Instant::now(),
            spinner: Spinner::with_verb(format!("Running {}", tool_name)),
        }
    }

    /// Update status
    pub fn set_status(&mut self, status: ToolStatus) {
        self.status = status;
        match status {
            ToolStatus::Executing => {
                self.spinner.set_verb(format!("Running {}", self.tool_name));
            }
            ToolStatus::WaitingConfirmation => {
                self.spinner.set_verb("Waiting for confirmation".to_string());
            }
            ToolStatus::Success => {
                self.spinner.set_verb(format!("{} completed", self.tool_name));
            }
            ToolStatus::Error => {
                self.spinner.set_verb(format!("{} failed", self.tool_name));
            }
            ToolStatus::Cancelled => {
                self.spinner.set_verb(format!("{} cancelled", self.tool_name));
            }
            _ => {}
        }
    }

    /// Update description
    pub fn set_description(&mut self, description: String) {
        self.description = description;
    }

    /// Tick animation
    pub fn tick(&mut self) -> bool {
        self.spinner.tick()
    }

    /// Get elapsed time
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get status icon
    fn status_icon(&self) -> char {
        match self.status {
            ToolStatus::Starting => '○',
            ToolStatus::Executing => self.spinner.frame_char(),
            ToolStatus::WaitingConfirmation => '⏸',
            ToolStatus::Success => '✓',
            ToolStatus::Error => '✗',
            ToolStatus::Cancelled => '⊘',
        }
    }

    /// Get status color
    fn status_color(&self) -> Color {
        match self.status {
            ToolStatus::Starting => Color::Gray,
            ToolStatus::Executing => Color::Cyan,
            ToolStatus::WaitingConfirmation => Color::Yellow,
            ToolStatus::Success => Color::Green,
            ToolStatus::Error => Color::Red,
            ToolStatus::Cancelled => Color::Gray,
        }
    }

    /// Render as Lines
    pub fn render(&self, width: usize) -> Vec<Line<'static>> {
        let elapsed_secs = self.elapsed().as_secs();
        let time_str = if elapsed_secs < 60 {
            format!("{}s", elapsed_secs)
        } else {
            format!("{}m{}s", elapsed_secs / 60, elapsed_secs % 60)
        };

        let icon = self.status_icon();
        let color = self.status_color();

        // Truncate description if too long
        let max_desc_len = width.saturating_sub(self.tool_name.len() + 20);
        let desc_display = if self.description.len() > max_desc_len {
            format!("{}…", &self.description[..max_desc_len.saturating_sub(1)])
        } else {
            self.description.clone()
        };

        vec![
            Line::from(vec![
                Span::styled(icon.to_string(), Style::default().fg(color)),
                Span::raw(" "),
                Span::styled(self.tool_name.clone(), Style::default().fg(Color::White).add_modifier(Modifier::BOLD)),
                Span::raw(": "),
                Span::styled(desc_display, Style::default().fg(Color::Gray)),
                Span::raw(" "),
                Span::styled(format!("({})", time_str), Style::default().fg(Color::DarkGray)),
            ]),
        ]
    }
}

/// Progress manager - tracks multiple tool progresses
pub struct ProgressManager {
    /// Active tool progresses
    tools: Vec<ToolProgress>,
    /// Main spinner (for general operations)
    main_spinner: Option<Spinner>,
    /// Last render time
    last_render: Instant,
}

impl ProgressManager {
    /// Create new progress manager
    pub fn new() -> Self {
        Self {
            tools: Vec::new(),
            main_spinner: None,
            last_render: Instant::now(),
        }
    }

    /// Start main spinner
    pub fn start_main(&mut self, verb: Option<String>) {
        self.main_spinner = Some(match verb {
            Some(v) => Spinner::with_verb(v),
            None => Spinner::new(),
        });
    }

    /// Stop main spinner
    pub fn stop_main(&mut self) {
        self.main_spinner = None;
    }

    /// Add tool progress
    pub fn add_tool(&mut self, tool_name: String, description: String) -> usize {
        let progress = ToolProgress::new(tool_name, description);
        self.tools.push(progress);
        self.tools.len() - 1
    }

    /// Update tool status
    pub fn update_tool_status(&mut self, index: usize, status: ToolStatus) {
        if let Some(tool) = self.tools.get_mut(index) {
            tool.set_status(status);
        }
    }

    /// Remove completed tool
    pub fn remove_tool(&mut self, index: usize) {
        if index < self.tools.len() {
            self.tools.remove(index);
        }
    }

    /// Clear all completed tools
    pub fn clear_completed(&mut self) {
        self.tools.retain(|t| {
            t.status != ToolStatus::Success
                && t.status != ToolStatus::Error
                && t.status != ToolStatus::Cancelled
        });
    }

    /// Tick all animations
    pub fn tick(&mut self) -> bool {
        let mut changed = false;

        if let Some(spinner) = &mut self.main_spinner {
            if spinner.tick() {
                changed = true;
            }
        }

        for tool in &mut self.tools {
            if tool.tick() {
                changed = true;
            }
        }

        changed
    }

    /// Check if any active operations
    pub fn is_active(&self) -> bool {
        self.main_spinner.is_some()
            || self.tools.iter().any(|t| {
                t.status == ToolStatus::Starting
                    || t.status == ToolStatus::Executing
                    || t.status == ToolStatus::WaitingConfirmation
            })
    }

    /// Render all progress indicators
    pub fn render(&self, width: usize) -> Vec<Line<'static>> {
        let mut lines = Vec::new();

        // Main spinner
        if let Some(spinner) = &self.main_spinner {
            lines.push(spinner.render());
        }

        // Tool progresses
        for tool in &self.tools {
            lines.extend(tool.render(width));
        }

        lines
    }

    /// Get active tool count
    pub fn active_tool_count(&self) -> usize {
        self.tools
            .iter()
            .filter(|t| t.status == ToolStatus::Executing || t.status == ToolStatus::Starting)
            .count()
    }
}

impl Default for ProgressManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_spinner_frames() {
        let mut spinner = Spinner::new();

        // Initial frame
        assert!(spinner.frame < SPINNER_FRAMES.len());

        // Tick should NOT advance frame immediately (needs interval)
        spinner.tick();
        assert_eq!(spinner.frame, 0); // Still 0 because interval not elapsed

        // Wait for interval and tick again
        std::thread::sleep(Duration::from_millis(100));
        spinner.tick();
        assert_eq!(spinner.frame, 1); // Now advances
    }

    #[test]
    fn test_spinner_elapsed() {
        let spinner = Spinner::new();
        std::thread::sleep(Duration::from_millis(100));

        let elapsed = spinner.elapsed();
        assert!(elapsed.as_millis() >= 100);
    }

    #[test]
    fn test_spinner_message() {
        let spinner = Spinner::with_verb("Thinking".to_string());
        assert_eq!(spinner.message(), "Thinking…");
    }

    #[test]
    fn test_progress_bar_full() {
        let bar = ProgressBar::new(1.0, 10);
        // Just verify it doesn't panic
        let _line = bar.render();
    }

    #[test]
    fn test_progress_bar_half() {
        let bar = ProgressBar::new(0.5, 10);
        let line = bar.render();
        assert!(line.spans.len() >= 2);
    }

    #[test]
    fn test_progress_bar_zero() {
        let bar = ProgressBar::new(0.0, 10);
        let line = bar.render();
        assert!(line.spans.len() >= 2);
    }

    #[test]
    fn test_tool_progress_status() {
        let mut tp = ToolProgress::new("BashTool".to_string(), "ls -la".to_string());

        tp.set_status(ToolStatus::Executing);
        assert_eq!(tp.status, ToolStatus::Executing);

        tp.set_status(ToolStatus::Success);
        assert_eq!(tp.status, ToolStatus::Success);
    }

    #[test]
    fn test_tool_progress_elapsed() {
        let tp = ToolProgress::new("BashTool".to_string(), "test".to_string());
        std::thread::sleep(Duration::from_millis(50));

        let elapsed = tp.elapsed();
        assert!(elapsed.as_millis() >= 50);
    }

    #[test]
    fn test_progress_manager() {
        let mut pm = ProgressManager::new();

        // Start main spinner
        pm.start_main(Some("Loading".to_string()));
        assert!(pm.is_active());

        // Add tool
        let idx = pm.add_tool("BashTool".to_string(), "ls".to_string());
        assert_eq!(idx, 0);
        assert_eq!(pm.active_tool_count(), 1);

        // Update status
        pm.update_tool_status(0, ToolStatus::Success);
        assert_eq!(pm.active_tool_count(), 0);

        // Clear completed
        pm.clear_completed();
        assert_eq!(pm.tools.len(), 0);
    }

    #[test]
    fn test_progress_manager_tick() {
        let mut pm = ProgressManager::new();
        pm.start_main(None);

        // Tick immediately - should not change
        let changed = pm.tick();
        assert!(!changed);

        // Wait for interval
        std::thread::sleep(Duration::from_millis(100));
        let changed = pm.tick();
        assert!(changed);
    }
}
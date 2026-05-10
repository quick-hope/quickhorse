//! Application state management with multiline text editor

use crate::commands::{CommandRegistry, CommandContext};
use crate::config::Config;
use crate::provider::{ContentBlock, Message, Provider, StreamEvent, StreamReceiver};
use crate::tui::progress::{ProgressManager, ToolStatus};
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::{Arc, RwLock};
use unicode_width::UnicodeWidthStr;

/// Multiline text editor with proper cursor handling
#[derive(Debug, Clone)]
pub struct TextEditor {
    /// Lines of text
    lines: Vec<String>,
    /// Cursor row (0-indexed)
    cursor_row: usize,
    /// Cursor column (byte position, 0-indexed)
    cursor_col: usize,
}

impl TextEditor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_row: 0,
            cursor_col: 0,
        }
    }

    /// Get full text content
    pub fn text(&self) -> String {
        self.lines.join("\n")
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.lines.iter().all(|l| l.is_empty())
    }

    /// Clear all content
    pub fn clear(&mut self) {
        self.lines = vec![String::new()];
        self.cursor_row = 0;
        self.cursor_col = 0;
    }

    /// Get lines for display
    pub fn lines(&self) -> &[String] {
        &self.lines
    }

    /// Get cursor position (row, byte_col)
    pub fn cursor_position(&self) -> (usize, usize) {
        (self.cursor_row, self.cursor_col)
    }

    /// Get display width of current line up to cursor
    pub fn cursor_display_x(&self) -> usize {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            let slice = &line[..self.cursor_col.min(line.len())];
            slice.width()
        } else {
            0
        }
    }

    /// Insert a character at cursor
    pub fn insert_char(&mut self, c: char) {
        if self.cursor_row < self.lines.len() {
            self.lines[self.cursor_row].insert(self.cursor_col, c);
            self.cursor_col += c.len_utf8();
        }
    }

    /// Insert newline at cursor
    pub fn insert_newline(&mut self) {
        if self.cursor_row < self.lines.len() {
            let current_line = self.lines[self.cursor_row].clone();
            let before = current_line[..self.cursor_col].to_string();
            let after = current_line[self.cursor_col..].to_string();

            self.lines[self.cursor_row] = before;
            self.lines.insert(self.cursor_row + 1, after);
            self.cursor_row += 1;
            self.cursor_col = 0;
        }
    }

    /// Delete char before cursor (Backspace)
    pub fn backspace(&mut self) {
        if self.cursor_col > 0 {
            // Delete from current line
            let line = &mut self.lines[self.cursor_row];
            // Find byte position of previous char
            let prev_char_end = self.cursor_col;
            let prev_char_start = line[..prev_char_end]
                .char_indices()
                .rev()
                .next()
                .map(|(i, _)| i)
                .unwrap_or(0);
            line.remove(prev_char_start);
            self.cursor_col = prev_char_start;
        } else if self.cursor_row > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_row);
            let prev_line_len = self.lines[self.cursor_row - 1].len();
            self.lines[self.cursor_row - 1].push_str(&current_line);
            self.cursor_row -= 1;
            self.cursor_col = prev_line_len;
        }
    }

    /// Delete char at cursor (Delete)
    pub fn delete(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            if self.cursor_col < line.len() {
                self.lines[self.cursor_row].remove(self.cursor_col);
            } else if self.cursor_row + 1 < self.lines.len() {
                // Merge with next line
                let next_line = self.lines.remove(self.cursor_row + 1);
                self.lines[self.cursor_row].push_str(&next_line);
            }
        }
    }

    /// Move cursor left
    pub fn move_left(&mut self) {
        if self.cursor_col > 0 {
            let line = &self.lines[self.cursor_row];
            let prev_char = line[..self.cursor_col]
                .chars()
                .rev()
                .next()
                .unwrap();
            self.cursor_col -= prev_char.len_utf8();
        } else if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }

    /// Move cursor right
    pub fn move_right(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line = &self.lines[self.cursor_row];
            if self.cursor_col < line.len() {
                let next_char = line[self.cursor_col..]
                    .chars()
                    .next()
                    .unwrap();
                self.cursor_col += next_char.len_utf8();
            } else if self.cursor_row + 1 < self.lines.len() {
                self.cursor_row += 1;
                self.cursor_col = 0;
            }
        }
    }

    /// Move cursor up
    pub fn move_up(&mut self) {
        if self.cursor_row > 0 {
            self.cursor_row -= 1;
            self.adjust_cursor_col();
        }
    }

    /// Move cursor down
    pub fn move_down(&mut self) {
        if self.cursor_row + 1 < self.lines.len() {
            self.cursor_row += 1;
            self.adjust_cursor_col();
        }
    }

    /// Adjust cursor column to fit current line
    fn adjust_cursor_col(&mut self) {
        if self.cursor_row < self.lines.len() {
            let line_len = self.lines[self.cursor_row].len();
            self.cursor_col = self.cursor_col.min(line_len);
        }
    }

    /// Move cursor to start of line
    pub fn move_home(&mut self) {
        self.cursor_col = 0;
    }

    /// Move cursor to end of line
    pub fn move_end(&mut self) {
        if self.cursor_row < self.lines.len() {
            self.cursor_col = self.lines[self.cursor_row].len();
        }
    }
}

impl Default for TextEditor {
    fn default() -> Self {
        Self::new()
    }
}

/// Application state
pub struct App {
    /// Text editor for input
    pub editor: TextEditor,
    /// Chat messages
    pub messages: Vec<Message>,
    /// Status message
    pub status: String,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Whether we're waiting for a response
    pub is_loading: bool,
    /// Whether we're streaming a response
    pub is_streaming: bool,
    /// Stream receiver for real-time updates
    pub stream_rx: Option<StreamReceiver>,
    /// Current streaming text being accumulated
    pub streaming_text: String,
    /// Scroll position for message area
    pub scroll: u16,
    /// Command registry for slash commands
    pub command_registry: CommandRegistry,
    /// Command context
    pub command_ctx: Option<CommandContext>,
    /// Ctrl+C press count for exit confirmation
    pub ctrl_c_count: u32,
    /// Last Ctrl+C timestamp for timeout
    pub last_ctrl_c_time: Option<std::time::Instant>,
    /// Progress manager for tool execution indicators
    pub progress_manager: ProgressManager,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            editor: TextEditor::new(),
            messages: Vec::new(),
            status: "Type your message. Enter to send, Ctrl+Enter for newline. Ctrl+C twice to quit.".to_string(),
            should_quit: false,
            is_loading: false,
            is_streaming: false,
            stream_rx: None,
            streaming_text: String::new(),
            scroll: 0,
            command_registry: CommandRegistry::new(),
            command_ctx: None,
            ctrl_c_count: 0,
            last_ctrl_c_time: None,
            progress_manager: ProgressManager::new(),
        }
    }

    /// Create App with Provider
    pub fn with_provider(provider: Arc<RwLock<dyn Provider>>, config: Config) -> Self {
        let ctx = CommandContext::new(provider, config);
        Self {
            editor: TextEditor::new(),
            messages: Vec::new(),
            status: "Type your message. Enter to send, Ctrl+Enter for newline. Ctrl+C twice to quit.".to_string(),
            should_quit: false,
            is_loading: false,
            is_streaming: false,
            stream_rx: None,
            streaming_text: String::new(),
            scroll: 0,
            command_registry: CommandRegistry::new(),
            command_ctx: Some(ctx),
            ctrl_c_count: 0,
            last_ctrl_c_time: None,
            progress_manager: ProgressManager::new(),
        }
    }

    /// Handle a key event (always in input mode)
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        // Ctrl+C handling for exit
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            self.handle_ctrl_c();
            return;
        }

        // Reset Ctrl+C count on any other key
        self.ctrl_c_count = 0;
        self.last_ctrl_c_time = None;
        self.status = "Type your message. Enter to send, Ctrl+Enter for newline. Ctrl+C twice to quit.".to_string();

        if self.is_loading {
            // Allow scrolling while loading
            match key.code {
                KeyCode::Up => {
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }
                }
                KeyCode::Down => {
                    self.scroll += 1;
                }
                KeyCode::PageUp => {
                    if self.scroll > 5 {
                        self.scroll -= 5;
                    } else {
                        self.scroll = 0;
                    }
                }
                KeyCode::PageDown => {
                    self.scroll += 5;
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Enter => {
                // Ctrl+Enter or Alt+Enter = newline, plain Enter = send
                if key.modifiers == KeyModifiers::CONTROL || key.modifiers == KeyModifiers::ALT {
                    self.editor.insert_newline();
                } else if !self.editor.is_empty() {
                    self.send_message();
                }
            }
            KeyCode::Char(c) => {
                self.editor.insert_char(c);
            }
            KeyCode::Backspace => {
                self.editor.backspace();
            }
            KeyCode::Delete => {
                self.editor.delete();
            }
            KeyCode::Left => {
                self.editor.move_left();
            }
            KeyCode::Right => {
                self.editor.move_right();
            }
            KeyCode::Up => {
                if self.editor.cursor_row > 0 {
                    self.editor.move_up();
                } else {
                    // Scroll messages when at first line
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }
                }
            }
            KeyCode::Down => {
                if self.editor.cursor_row + 1 < self.editor.lines().len() {
                    self.editor.move_down();
                } else {
                    // Scroll messages when at last line
                    self.scroll += 1;
                }
            }
            KeyCode::Home => {
                self.editor.move_home();
            }
            KeyCode::End => {
                self.editor.move_end();
            }
            KeyCode::PageUp => {
                if self.scroll > 5 {
                    self.scroll -= 5;
                } else {
                    self.scroll = 0;
                }
            }
            KeyCode::PageDown => {
                self.scroll += 5;
            }
            KeyCode::Esc => {
                self.editor.clear();
            }
            _ => {}
        }
    }

    /// Send the current message
    fn send_message(&mut self) {
        let text = self.editor.text();
        if text.is_empty() {
            return;
        }

        // Check if it's a slash command
        if CommandRegistry::is_command(&text) {
            if let Some(ctx) = &mut self.command_ctx {
                let result = self.command_registry.execute(&text, ctx);
                match result {
                    Some(cmd_result) => {
                        self.messages.push(Message::assistant(cmd_result.output));
                        if cmd_result.clear_history {
                            self.messages.clear();
                        }
                        if cmd_result.provider_changed {
                            self.status = format!("Provider: {}", ctx.current_provider_name);
                        }
                    }
                    None => {
                        self.messages.push(Message::assistant(format!("Unknown command: {}", text)));
                    }
                }
                ctx.messages = self.messages.clone();
            } else {
                self.messages.push(Message::assistant(
                    "Commands not available. Please restart with a provider.".to_string()
                ));
            }
        } else {
            // Normal message
            let user_message = Message::user(text);
            self.messages.push(user_message);
            self.is_loading = true;
        }
        self.editor.clear();
    }

    /// Handle Ctrl+C for graceful exit
    fn handle_ctrl_c(&mut self) {
        let now = std::time::Instant::now();

        // Check if previous Ctrl+C was within 2 seconds
        if let Some(last_time) = self.last_ctrl_c_time {
            if now.duration_since(last_time) < std::time::Duration::from_secs(2) {
                self.ctrl_c_count += 1;
            } else {
                self.ctrl_c_count = 1;
            }
        } else {
            self.ctrl_c_count = 1;
        }

        self.last_ctrl_c_time = Some(now);

        if self.ctrl_c_count >= 2 {
            self.should_quit = true;
        } else {
            self.status = "Press Ctrl+C again within 2 seconds to quit.".to_string();
        }
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: String) {
        let message = Message::assistant(content);
        self.messages.push(message);
        self.is_loading = false;
        self.is_streaming = false;
    }

    /// Set the status message
    #[allow(dead_code)]
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }

    /// Start streaming with the given receiver
    pub fn start_streaming(&mut self, rx: StreamReceiver) {
        self.stream_rx = Some(rx);
        self.is_streaming = true;
        self.is_loading = true;
        self.streaming_text = String::new();

        // Add placeholder assistant message for streaming updates
        self.messages.push(Message::assistant(String::new()));
    }

    /// Handle streaming events (non-blocking)
    /// Returns true if streaming is still ongoing
    pub fn handle_stream_event(&mut self) -> bool {
        if let Some(rx) = &mut self.stream_rx {
            // Try to receive events without blocking
            while let Ok(event) = rx.try_recv() {
                match event {
                    StreamEvent::TextDelta(text) => {
                        self.streaming_text.push_str(&text);
                        // Update the last assistant message with streaming content
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = vec![ContentBlock::text(self.streaming_text.clone())];
                            }
                        }
                    }
                    StreamEvent::Done => {
                        // Finalize the streaming message
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = vec![ContentBlock::text(self.streaming_text.clone())];
                            }
                        }
                        self.is_streaming = false;
                        self.is_loading = false;
                        self.stream_rx = None;
                        self.streaming_text.clear();
                        return false;
                    }
                    StreamEvent::Error(e) => {
                        // Replace streaming message with error
                        if let Some(last) = self.messages.last_mut() {
                            if last.role == "assistant" {
                                last.content = vec![ContentBlock::text(format!("Error: {}", e))];
                            }
                        }
                        self.is_streaming = false;
                        self.is_loading = false;
                        self.stream_rx = None;
                        self.streaming_text.clear();
                        return false;
                    }
                    // Tool call events - for now, just append info text
                    StreamEvent::ToolCallStart { id, name } => {
                        self.streaming_text.push_str(&format!("\n[Tool: {} ({})]\n", name, id));
                    }
                    StreamEvent::ToolCallDelta { id: _, arguments } => {
                        self.streaming_text.push_str(&arguments);
                    }
                }
            }
            return true; // Still streaming
        }
        false
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
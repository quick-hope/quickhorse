//! Application state management

use crate::provider::Message;
use crossterm::event::{KeyCode, KeyModifiers};

/// Input mode for the TUI
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum InputMode {
    /// Normal mode - navigation
    Normal,
    /// Insert mode - typing input
    Insert,
}

/// Application state
pub struct App {
    /// Current input mode
    pub input_mode: InputMode,
    /// Current input text
    pub input: String,
    /// Chat messages
    pub messages: Vec<Message>,
    /// Status message
    pub status: String,
    /// Whether the app should quit
    pub should_quit: bool,
    /// Whether we're waiting for a response
    pub is_loading: bool,
    /// Scroll position for message area
    pub scroll: u16,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            input_mode: InputMode::Normal,
            input: String::new(),
            messages: Vec::new(),
            status: "Press 'i' to enter input mode, 'Esc' to exit, 'q' to quit".to_string(),
            should_quit: false,
            is_loading: false,
            scroll: 0,
        }
    }

    /// Handle a key event
    pub fn handle_key(&mut self, key: crossterm::event::KeyEvent) {
        match self.input_mode {
            InputMode::Normal => self.handle_normal_mode(key),
            InputMode::Insert => self.handle_insert_mode(key),
        }
    }

    fn handle_normal_mode(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('i') => {
                self.input_mode = InputMode::Insert;
                self.status = "Insert mode - Press 'Esc' to exit".to_string();
            }
            KeyCode::Up => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.scroll += 1;
            }
            _ => {}
        }
    }

    fn handle_insert_mode(&mut self, key: crossterm::event::KeyEvent) {
        match key.code {
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.status = "Press 'i' to enter input mode, 'Esc' to exit, 'q' to quit".to_string();
            }
            KeyCode::Enter => {
                if !self.input.is_empty() {
                    // Add user message
                    let user_message = Message::user(self.input.clone());
                    self.messages.push(user_message);
                    self.input.clear();
                    self.is_loading = true;
                }
            }
            KeyCode::Char(c) => {
                if key.modifiers == KeyModifiers::CONTROL && c == 'c' {
                    // Ctrl+C in insert mode
                    self.input_mode = InputMode::Normal;
                    self.status = "Press 'i' to enter input mode, 'Esc' to exit, 'q' to quit".to_string();
                } else {
                    self.input.push(c);
                }
            }
            KeyCode::Backspace => {
                self.input.pop();
            }
            KeyCode::Delete => {
                // Delete character at cursor position (for now, just pop from end)
                self.input.pop();
            }
            KeyCode::Left => {
                // Cursor movement (simplified - for future enhancement)
            }
            KeyCode::Right => {
                // Cursor movement (simplified - for future enhancement)
            }
            _ => {}
        }
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: String) {
        let message = Message::assistant(content);
        self.messages.push(message);
        self.is_loading = false;
    }

    /// Set the status message
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
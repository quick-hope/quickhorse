//! Application state management

use crate::commands::{CommandRegistry, CommandContext};
use crate::config::Config;
use crate::provider::Message;
use crate::provider::Provider;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::{Arc, RwLock};

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
    /// Command registry for slash commands
    pub command_registry: CommandRegistry,
    /// Command context
    pub command_ctx: Option<CommandContext>,
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
            command_registry: CommandRegistry::new(),
            command_ctx: None,
        }
    }

    /// Create App with Provider
    pub fn with_provider(provider: Arc<RwLock<dyn Provider>>, config: Config) -> Self {
        let ctx = CommandContext::new(provider, config);
        Self {
            input_mode: InputMode::Normal,
            input: String::new(),
            messages: Vec::new(),
            status: "Press 'i' to enter input mode, 'Esc' to exit, 'q' to quit".to_string(),
            should_quit: false,
            is_loading: false,
            scroll: 0,
            command_registry: CommandRegistry::new(),
            command_ctx: Some(ctx),
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
                    // Check if it's a slash command
                    if CommandRegistry::is_command(&self.input) {
                        if let Some(ctx) = &mut self.command_ctx {
                            let result = self.command_registry.execute(&self.input, ctx);
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
                                    self.messages.push(Message::assistant(format!("Unknown command: {}", self.input)));
                                }
                            }
                            // Sync messages
                            ctx.messages = self.messages.clone();
                        } else {
                            self.messages.push(Message::assistant(
                                "Commands not available. Please restart with a provider.".to_string()
                            ));
                        }
                    } else {
                        // Normal message processing
                        let user_message = Message::user(self.input.clone());
                        self.messages.push(user_message);
                        self.is_loading = true;
                    }
                    self.input.clear();
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
    #[allow(dead_code)]
    pub fn set_status(&mut self, status: String) {
        self.status = status;
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
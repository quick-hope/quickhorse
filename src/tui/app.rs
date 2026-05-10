//! Application state management

use crate::commands::{CommandRegistry, CommandContext};
use crate::config::Config;
use crate::provider::Message;
use crate::provider::Provider;
use crossterm::event::{KeyCode, KeyModifiers};
use std::sync::{Arc, RwLock};

/// Application state
pub struct App {
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
    /// Ctrl+C press count for exit confirmation
    pub ctrl_c_count: u32,
    /// Last Ctrl+C timestamp for timeout
    pub last_ctrl_c_time: Option<std::time::Instant>,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        Self {
            input: String::new(),
            messages: Vec::new(),
            status: "Type your message and press Enter. Ctrl+C twice to quit.".to_string(),
            should_quit: false,
            is_loading: false,
            scroll: 0,
            command_registry: CommandRegistry::new(),
            command_ctx: None,
            ctrl_c_count: 0,
            last_ctrl_c_time: None,
        }
    }

    /// Create App with Provider
    pub fn with_provider(provider: Arc<RwLock<dyn Provider>>, config: Config) -> Self {
        let ctx = CommandContext::new(provider, config);
        Self {
            input: String::new(),
            messages: Vec::new(),
            status: "Type your message and press Enter. Ctrl+C twice to quit.".to_string(),
            should_quit: false,
            is_loading: false,
            scroll: 0,
            command_registry: CommandRegistry::new(),
            command_ctx: Some(ctx),
            ctrl_c_count: 0,
            last_ctrl_c_time: None,
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
        self.status = "Type your message and press Enter. Ctrl+C twice to quit.".to_string();

        match key.code {
            KeyCode::Enter => {
                if !self.input.is_empty() && !self.is_loading {
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
                if !self.is_loading {
                    self.input.push(c);
                }
            }
            KeyCode::Backspace => {
                if !self.is_loading {
                    self.input.pop();
                }
            }
            KeyCode::Delete => {
                if !self.is_loading {
                    self.input.pop();
                }
            }
            KeyCode::Up => {
                if self.scroll > 0 {
                    self.scroll -= 1;
                }
            }
            KeyCode::Down => {
                self.scroll += 1;
            }
            KeyCode::Esc => {
                // Clear input on Esc
                self.input.clear();
            }
            _ => {}
        }
    }

    /// Handle Ctrl+C for graceful exit
    fn handle_ctrl_c(&mut self) {
        let now = std::time::Instant::now();

        // Check if previous Ctrl+C was within 2 seconds
        if let Some(last_time) = self.last_ctrl_c_time {
            if now.duration_since(last_time) < std::time::Duration::from_secs(2) {
                self.ctrl_c_count += 1;
            } else {
                // Timeout expired, reset count
                self.ctrl_c_count = 1;
            }
        } else {
            self.ctrl_c_count = 1;
        }

        self.last_ctrl_c_time = Some(now);

        if self.ctrl_c_count >= 2 {
            // Second Ctrl+C - quit
            self.should_quit = true;
        } else {
            // First Ctrl+C - show warning
            self.status = "Press Ctrl+C again within 2 seconds to quit.".to_string();
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
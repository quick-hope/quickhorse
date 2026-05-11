//! Application state management with multiline text editor

use crate::agent::PendingPermission;
use crate::commands::{CommandRegistry, CommandContext};
use crate::config::Config;
use crate::permissions::{PermissionMode, PermissionUpdate, RuleBehavior, RuleSource, RuleValue};
use crate::provider::{ContentBlock, Message, Provider, StreamEvent, StreamReceiver};
use crate::tui::completion::{CommandCompleter, CompletionProvider, CompletionState, PathCompleter};
use crate::tui::permission_dialog::{PermissionChoice, PermissionDialog};
use crate::tui::progress::{ProgressManager, ToolStatus};
use crossterm::event::{KeyCode, KeyModifiers};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use unicode_width::{UnicodeWidthStr, UnicodeWidthChar};

/// Format error message in a boxed style for TUI display
fn format_error_box(error_msg: &str) -> Vec<String> {
    let max_width = 50;
    let lines: Vec<&str> = error_msg.split('\n').collect();

    // Calculate box width
    let content_width = lines.iter().map(|l| l.width()).max().unwrap_or(0).min(max_width);
    let box_width = content_width + 2;

    let border_top = format!("┌─ 错误 ─{}┐", "─".repeat(box_width - 8));
    let border_bottom = format!("└{}┘", "─".repeat(box_width));

    let mut result = vec![border_top];

    for line in lines {
        // Truncate if too long
        let truncated = if line.width() > max_width {
            // Try to find a truncation point that preserves unicode
            let mut truncated_line = String::new();
            let mut width = 0;
            for c in line.chars() {
                if width + c.width().unwrap_or(0) > max_width - 3 {
                    break;
                }
                truncated_line.push(c);
                width += c.width().unwrap_or(0);
            }
            truncated_line.push_str("...");
            truncated_line
        } else {
            line.to_string()
        };

        let padding = box_width - truncated.width() - 2;
        result.push(format!("│ {}{}│", truncated, " ".repeat(padding)));
    }

    result.push(border_bottom);
    result
}

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
    /// Mapping from tool_id to progress manager index
    tool_progress_map: HashMap<String, usize>,
    /// Pending permission request awaiting user confirmation
    pub pending_permission: Option<PendingPermission>,
    /// Permission dialog for user interaction
    pub permission_dialog: Option<PermissionDialog>,
    /// Permission mode
    pub permission_mode: PermissionMode,
    /// Pending permission updates to save (from AllowAndSave)
    pub pending_permission_updates: Option<PermissionUpdate>,
    /// Completion state for tab completion
    pub completion_state: CompletionState,
    /// Command completer
    pub completer: CommandCompleter,
    /// Path completer
    pub path_completer: PathCompleter,
}

/// Result of permission choice selection
pub struct PermissionChoiceResult {
    /// The user's choice
    pub choice: PermissionChoice,
    /// Tool name that requested permission
    pub tool_name: String,
    /// Tool ID for tracking
    pub tool_id: String,
    /// Input that needs permission
    pub input: serde_json::Value,
    /// Permission request message
    pub message: String,
    /// Permission updates to save (for AllowAndSave)
    pub updates: Option<PermissionUpdate>,
}

impl App {
    /// Create a new App instance
    pub fn new() -> Self {
        let registry = Arc::new(CommandRegistry::new());
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
            tool_progress_map: HashMap::new(),
            pending_permission: None,
            permission_dialog: None,
            permission_mode: PermissionMode::Default,
            pending_permission_updates: None,
            completion_state: CompletionState::new(),
            completer: CommandCompleter::new(registry),
            path_completer: PathCompleter::new(),
        }
    }

    /// Create App with Provider
    pub fn with_provider(provider: Arc<RwLock<dyn Provider>>, config: Config) -> Self {
        let ctx = CommandContext::new(provider, config.clone());
        let registry = Arc::new(CommandRegistry::new());
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
            tool_progress_map: HashMap::new(),
            pending_permission: None,
            permission_dialog: None,
            permission_mode: config.permissions.mode,
            pending_permission_updates: None,
            completion_state: CompletionState::new(),
            completer: CommandCompleter::new(registry),
            path_completer: PathCompleter::new(),
        }
    }

    /// Check if there's a pending permission request
    pub fn has_pending_permission(&self) -> bool {
        self.pending_permission.is_some()
    }

    /// Set pending permission request and create dialog
    pub fn set_pending_permission(&mut self, permission: PendingPermission) {
        use crate::permissions::PermissionResult;

        // Create permission dialog with the request
        let result = PermissionResult::ask(&permission.message);
        let dialog = PermissionDialog::new(permission.message.clone(), result);

        self.status = "[?] Permission request - use ↑↓ to select, Enter to confirm".to_string();
        self.pending_permission = Some(permission);
        self.permission_dialog = Some(dialog);
        self.pending_permission_updates = None;
    }

    /// Clear pending permission and dialog
    pub fn clear_pending_permission(&mut self) {
        self.pending_permission = None;
        self.permission_dialog = None;
        self.pending_permission_updates = None;
        self.status = "Type your message. Enter to send, Ctrl+Enter for newline. Ctrl+C twice to quit.".to_string();
    }

    /// Handle permission confirmation with full PermissionChoice support
    pub fn handle_permission_choice(&mut self) -> Option<PermissionChoiceResult> {
        if let Some(dialog) = &mut self.permission_dialog {
            let choice = dialog.confirm();
            let permission = self.pending_permission.take();

            if let Some(permission) = permission {
                let updates = if choice == PermissionChoice::AllowAndSave {
                    // Generate permission update for saving
                    Some(PermissionUpdate::AddRules {
                        destination: RuleSource::UserSettings,
                        rules: vec![RuleValue {
                            tool_name: permission.tool_name.clone(),
                            rule_content: None, // Will be filled by specific tool
                        }],
                        behavior: RuleBehavior::Allow,
                    })
                } else {
                    None
                };

                let result = PermissionChoiceResult {
                    choice,
                    tool_name: permission.tool_name,
                    tool_id: permission.tool_id,
                    input: permission.input,
                    message: permission.message,
                    updates,
                };

                self.clear_pending_permission();
                return Some(result);
            }
        }
        None
    }

    /// Handle permission confirmation (legacy y/n method)
    pub fn handle_permission_response(&mut self, approved: bool) -> Option<String> {
        if let Some(permission) = self.pending_permission.take() {
            if approved {
                self.status = format!("Approved: {}", permission.tool_name);
                Some(format!("Permission approved for: {}", permission.message))
            } else {
                self.status = format!("Denied: {}", permission.tool_name);
                Some(format!("Permission denied for: {}", permission.message))
            }
        } else {
            None
        }
    }
}

impl App {
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

        // Handle pending permission request with PermissionDialog
        if self.has_pending_permission() {
            if let Some(dialog) = &mut self.permission_dialog {
                match key.code {
                    KeyCode::Up => {
                        dialog.select_up();
                        self.status = format!("[?] Selected: {}",
                            match dialog.selected_choice() {
                                PermissionChoice::AllowOnce => "Allow (once)",
                                PermissionChoice::AllowAndSave => "Allow & save rule",
                                PermissionChoice::Deny => "Deny",
                                PermissionChoice::Cancel => "Cancel",
                            });
                    }
                    KeyCode::Down => {
                        dialog.select_down();
                        self.status = format!("[?] Selected: {}",
                            match dialog.selected_choice() {
                                PermissionChoice::AllowOnce => "Allow (once)",
                                PermissionChoice::AllowAndSave => "Allow & save rule",
                                PermissionChoice::Deny => "Deny",
                                PermissionChoice::Cancel => "Cancel",
                            });
                    }
                    KeyCode::Enter => {
                        // Confirm selection
                        if let Some(result) = self.handle_permission_choice() {
                            let response = match result.choice {
                                PermissionChoice::AllowOnce => {
                                    format!("✓ Permission approved for: {}", result.message)
                                }
                                PermissionChoice::AllowAndSave => {
                                    self.pending_permission_updates = result.updates;
                                    format!("✓ Permission approved & rule will be saved for: {}", result.message)
                                }
                                PermissionChoice::Deny => {
                                    format!("✗ Permission denied for: {}", result.message)
                                }
                                PermissionChoice::Cancel => {
                                    "Permission request cancelled".to_string()
                                }
                            };
                            self.messages.push(Message::assistant(response));
                        }
                    }
                    KeyCode::Esc => {
                        // Cancel permission request
                        self.clear_pending_permission();
                        self.messages.push(Message::assistant("Permission request cancelled".to_string()));
                    }
                    _ => {
                        // Ignore other keys while permission is pending
                    }
                }
            } else {
                // Legacy y/n handling if dialog not initialized
                match key.code {
                    KeyCode::Char('y') | KeyCode::Char('Y') => {
                        if let Some(response) = self.handle_permission_response(true) {
                            self.messages.push(Message::assistant(response));
                        }
                    }
                    KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                        if let Some(response) = self.handle_permission_response(false) {
                            self.messages.push(Message::assistant(response));
                        }
                    }
                    _ => {}
                }
            }
            return;
        }

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
            KeyCode::Tab => {
                // Handle completion navigation
                if self.completion_state.is_visible() {
                    self.completion_state.select_next();
                } else {
                    // Trigger completion if input starts with /
                    let text = self.editor.text();
                    let cursor_pos = self.get_cursor_byte_pos();
                    if self.completer.can_complete(&text, cursor_pos) {
                        let suggestions = self.completer.get_suggestions(&text, cursor_pos);
                        self.completion_state.show(suggestions, &text);
                    }
                }
            }
            KeyCode::BackTab => {
                // Shift+Tab - navigate backwards
                if self.completion_state.is_visible() {
                    self.completion_state.select_prev();
                }
            }
            KeyCode::Enter => {
                // If completion is visible, accept selected suggestion
                if self.completion_state.is_visible() {
                    self.accept_completion();
                } else if key.modifiers == KeyModifiers::CONTROL || key.modifiers == KeyModifiers::ALT {
                    self.editor.insert_newline();
                } else if !self.editor.is_empty() {
                    self.send_message();
                }
            }
            KeyCode::Char(c) => {
                self.editor.insert_char(c);
                // Update completion suggestions
                self.update_completion();
            }
            KeyCode::Backspace => {
                self.editor.backspace();
                // Update completion suggestions
                self.update_completion();
            }
            KeyCode::Delete => {
                self.editor.delete();
                // Update completion suggestions
                self.update_completion();
            }
            KeyCode::Left => {
                self.editor.move_left();
                // Hide completion when moving cursor
                self.completion_state.hide();
            }
            KeyCode::Right => {
                self.editor.move_right();
                // Hide completion when moving cursor
                self.completion_state.hide();
            }
            KeyCode::Up => {
                // If completion visible, navigate in it
                if self.completion_state.is_visible() {
                    self.completion_state.select_prev();
                } else if self.editor.cursor_row > 0 {
                    self.editor.move_up();
                } else {
                    // Scroll messages when at first line
                    if self.scroll > 0 {
                        self.scroll -= 1;
                    }
                }
            }
            KeyCode::Down => {
                // If completion visible, navigate in it
                if self.completion_state.is_visible() {
                    self.completion_state.select_next();
                } else if self.editor.cursor_row + 1 < self.editor.lines().len() {
                    self.editor.move_down();
                } else {
                    // Scroll messages when at last line
                    self.scroll += 1;
                }
            }
            KeyCode::Home => {
                self.editor.move_home();
                self.completion_state.hide();
            }
            KeyCode::End => {
                self.editor.move_end();
                self.completion_state.hide();
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
                // If completion visible, hide it; otherwise clear input
                if self.completion_state.is_visible() {
                    self.completion_state.hide();
                } else {
                    self.editor.clear();
                }
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
            // Start progress spinner
            self.progress_manager.start_main(Some("Sending message".to_string()));
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

    /// Get cursor position as byte offset in full text
    fn get_cursor_byte_pos(&self) -> usize {
        let (row, col) = self.editor.cursor_position();
        let lines = self.editor.lines();

        // Calculate byte offset: sum of all lines before current + current line offset
        let mut offset = 0;
        for (i, line) in lines.iter().enumerate() {
            if i < row {
                offset += line.len() + 1; // +1 for newline
            } else if i == row {
                offset += col;
            }
        }
        offset
    }

    /// Update completion suggestions based on current input
    fn update_completion(&mut self) {
        let text = self.editor.text();
        let cursor_pos = self.get_cursor_byte_pos();

        // Try command completer first (for slash commands)
        if self.completer.can_complete(&text, cursor_pos) {
            let suggestions = self.completer.get_suggestions(&text, cursor_pos);
            if suggestions.is_empty() {
                self.completion_state.hide();
            } else {
                // Keep selection if input hasn't changed much
                let current_selected = if self.completion_state.input_changed(&text) {
                    0
                } else {
                    self.completion_state.selected_index()
                };
                self.completion_state.show(suggestions, &text);
                // Restore selection if possible
                for _ in 0..current_selected.min(self.completion_state.count() - 1) {
                    self.completion_state.select_next();
                }
            }
        } else if self.path_completer.can_complete(&text, cursor_pos) {
            // Try path completer (for file paths)
            let suggestions = self.path_completer.get_suggestions(&text, cursor_pos);
            if suggestions.is_empty() {
                self.completion_state.hide();
            } else {
                let current_selected = if self.completion_state.input_changed(&text) {
                    0
                } else {
                    self.completion_state.selected_index()
                };
                self.completion_state.show(suggestions, &text);
                for _ in 0..current_selected.min(self.completion_state.count() - 1) {
                    self.completion_state.select_next();
                }
            }
        } else {
            self.completion_state.hide();
        }
    }

    /// Accept selected completion suggestion
    fn accept_completion(&mut self) {
        if let Some(suggestion) = self.completion_state.selected_suggestion() {
            let text = self.editor.text();
            let cursor_pos = self.get_cursor_byte_pos();

            let completed_text = match suggestion.completion_type {
                crate::tui::CompletionType::Command => {
                    // For slash commands: replace the partial command name
                    format!("/{}", suggestion.id)
                }
                crate::tui::CompletionType::Path => {
                    // For paths: find where the path starts and replace with suggestion
                    // The replace_suffix contains the full path to replace
                    let path_start = self.path_completer.find_path_start(&text, cursor_pos);

                    if let Some(start) = path_start {
                        // Keep text before path, replace path portion
                        let before_path = &text[..start];
                        format!("{}{}", before_path, suggestion.replace_suffix)
                    } else {
                        // Fallback: use replace_suffix directly
                        suggestion.replace_suffix.clone()
                    }
                }
                _ => {
                    // For other types (Provider, Model): use replace_suffix
                    suggestion.replace_suffix.clone()
                }
            };

            self.editor.clear();
            for c in completed_text.chars() {
                self.editor.insert_char(c);
            }
        }
        self.completion_state.hide();
    }

    /// Add an assistant message
    pub fn add_assistant_message(&mut self, content: String) {
        let message = Message::assistant(content);
        self.messages.push(message);
        self.is_loading = false;
        self.is_streaming = false;
        // Stop progress spinner
        self.progress_manager.stop_main();
        self.progress_manager.clear_completed();
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
        // Start progress spinner for streaming
        self.progress_manager.start_main(Some("Streaming response".to_string()));

        // Add placeholder assistant message for streaming updates
        self.messages.push(Message::assistant(String::new()));
    }

    /// Handle streaming events (non-blocking)
    /// Returns true if streaming is still ongoing
    pub fn handle_stream_event(&mut self) -> bool {
        // Collect events first to avoid borrow conflicts
        let mut events_to_process: Vec<StreamEvent> = Vec::new();
        let mut still_streaming = false;

        if let Some(rx) = &mut self.stream_rx {
            // Try to receive events without blocking
            while let Ok(event) = rx.try_recv() {
                events_to_process.push(event);
            }
        }

        // Process collected events
        for event in events_to_process {
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
                    // Stop progress
                    self.progress_manager.stop_main();
                    self.progress_manager.clear_completed();
                    return false;
                }
                StreamEvent::Error(e) => {
                    // Replace streaming message with formatted error
                    // e already contains user-friendly message from QuickHorseError
                    let error_lines = format_error_box(&e);
                    let error_content = error_lines.join("\n");
                    if let Some(last) = self.messages.last_mut() {
                        if last.role == "assistant" {
                            last.content = vec![ContentBlock::text(error_content)];
                        }
                    }
                    self.is_streaming = false;
                    self.is_loading = false;
                    self.stream_rx = None;
                    self.streaming_text.clear();
                    // Stop progress
                    self.progress_manager.stop_main();
                    self.progress_manager.clear_completed();
                    return false;
                }
                // Tool call events - update progress manager
                StreamEvent::ToolCallStart { id, name } => {
                    // Start tracking this tool execution
                    self.start_tool(id.clone(), name.clone(), format!("Executing {}...", name));
                    self.streaming_text.push_str(&format!("\n[Tool: {} ({})]\n", name, id));
                }
                StreamEvent::ToolCallDelta { id: _, arguments } => {
                    self.streaming_text.push_str(&arguments);
                }
                StreamEvent::ToolCallComplete { id: _, name, arguments } => {
                    // Mark tool as ready for execution
                    // The tool will actually execute in the agent loop
                    // For now, we just note that arguments are complete
                    self.streaming_text.push_str(&format!("\n[Tool {} ready: {} bytes]\n", name, arguments.len()));
                }
            }
            still_streaming = true;
        }

        still_streaming
    }

    /// Start tracking a tool execution
    pub fn start_tool(&mut self, tool_id: String, tool_name: String, description: String) {
        let idx = self.progress_manager.add_tool(tool_name, description);
        self.tool_progress_map.insert(tool_id, idx);
        self.progress_manager.update_tool_status(idx, ToolStatus::Executing);
    }

    /// Update tool execution status
    pub fn update_tool_status(&mut self, tool_id: &str, status: ToolStatus) {
        if let Some(idx) = self.tool_progress_map.get(tool_id) {
            self.progress_manager.update_tool_status(*idx, status);
        }
    }

    /// Complete tool execution with result
    pub fn complete_tool(&mut self, tool_id: &str, success: bool, _result_summary: String) {
        if let Some(idx) = self.tool_progress_map.get(tool_id) {
            let status = if success {
                ToolStatus::Success
            } else {
                ToolStatus::Error
            };
            self.progress_manager.update_tool_status(*idx, status);

            // Update description with result summary
            // Note: ProgressManager doesn't have a method to update description
            // We could add one, or just leave the original description

            // Remove from active tracking after a delay (via clear_completed)
        }
    }

    /// Clear all completed tool progresses
    pub fn clear_completed_tools(&mut self) {
        self.progress_manager.clear_completed();
        // Clear mapping for completed tools
        // Note: we'd need to track which tools are completed, but for simplicity
        // we just clear all mapping when progress_manager clears
        self.tool_progress_map.clear();
    }

    /// Get tool progress count
    pub fn tool_progress_count(&self) -> usize {
        self.progress_manager.active_tool_count()
    }
}

impl Default for App {
    fn default() -> Self {
        Self::new()
    }
}
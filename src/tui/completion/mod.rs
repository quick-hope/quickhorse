//! Completion module - Tab completion for TUI
//!
//! Provides autocomplete functionality for:
//! - Slash commands (/help, /provider, /model, etc.)
//! - File paths (~/, /, ./, ../)
//! - Provider/model arguments (future)

mod command;
mod path;

pub use command::CommandCompleter;
pub use path::{PathCompleter, PathEntry, PathEntryType};

/// Completion type identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionType {
    /// Slash command completion
    Command,
    /// File path completion
    Path,
    /// Provider name completion
    Provider,
    /// Model name completion
    Model,
}

/// A single completion suggestion
#[derive(Debug, Clone)]
pub struct Suggestion {
    /// Unique identifier for the suggestion
    pub id: String,
    /// Text to display in the completion popup
    pub display_text: String,
    /// Optional description/help text
    pub description: Option<String>,
    /// Type of completion
    pub completion_type: CompletionType,
    /// The text that should replace the input after cursor position
    pub replace_suffix: String,
}

impl Suggestion {
    /// Create a new suggestion
    pub fn new(
        id: String,
        display_text: String,
        description: Option<String>,
        completion_type: CompletionType,
        replace_suffix: String,
    ) -> Self {
        Self {
            id,
            display_text,
            description,
            completion_type,
            replace_suffix,
        }
    }

    /// Create a command suggestion
    pub fn command(name: &str, description: Option<String>) -> Self {
        Self::new(
            name.to_string(),
            format!("/{}", name),
            description,
            CompletionType::Command,
            name.to_string(),
        )
    }
}

/// Trait for completion providers
pub trait CompletionProvider {
    /// Get completion suggestions for the given input
    ///
    /// # Arguments
    /// * `input` - The current input text
    /// * `cursor_pos` - The cursor position (byte offset)
    ///
    /// # Returns
    /// A list of suggestions that match the input
    fn get_suggestions(&self, input: &str, cursor_pos: usize) -> Vec<Suggestion>;

    /// Check if this provider can provide completions for the given input
    fn can_complete(&self, input: &str, cursor_pos: usize) -> bool;
}

/// Completion state manager
#[derive(Debug, Clone)]
pub struct CompletionState {
    /// Current suggestions
    suggestions: Vec<Suggestion>,
    /// Selected suggestion index
    selected: usize,
    /// Whether completion popup is visible
    visible: bool,
    /// The input that triggered completion (for detecting changes)
    trigger_input: String,
}

impl CompletionState {
    /// Create a new completion state
    pub fn new() -> Self {
        Self {
            suggestions: Vec::new(),
            selected: 0,
            visible: false,
            trigger_input: String::new(),
        }
    }

    /// Show completion popup with suggestions
    pub fn show(&mut self, suggestions: Vec<Suggestion>, input: &str) {
        if suggestions.is_empty() {
            self.hide();
            return;
        }
        self.suggestions = suggestions;
        self.selected = 0;
        self.visible = true;
        self.trigger_input = input.to_string();
    }

    /// Hide completion popup
    pub fn hide(&mut self) {
        self.suggestions.clear();
        self.selected = 0;
        self.visible = false;
        self.trigger_input.clear();
    }

    /// Check if visible
    pub fn is_visible(&self) -> bool {
        self.visible && !self.suggestions.is_empty()
    }

    /// Get current suggestions
    pub fn suggestions(&self) -> &[Suggestion] {
        &self.suggestions
    }

    /// Get selected suggestion index
    pub fn selected_index(&self) -> usize {
        self.selected
    }

    /// Get selected suggestion
    pub fn selected_suggestion(&self) -> Option<&Suggestion> {
        self.suggestions.get(self.selected)
    }

    /// Move selection to next suggestion (cycle)
    pub fn select_next(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = (self.selected + 1) % self.suggestions.len();
    }

    /// Move selection to previous suggestion (cycle)
    pub fn select_prev(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        self.selected = if self.selected == 0 {
            self.suggestions.len() - 1
        } else {
            self.selected - 1
        };
    }

    /// Check if input has changed since completion was triggered
    pub fn input_changed(&self, current_input: &str) -> bool {
        self.trigger_input != current_input
    }

    /// Get number of suggestions
    pub fn count(&self) -> usize {
        self.suggestions.len()
    }
}

impl Default for CompletionState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_suggestion_creation() {
        let suggestion = Suggestion::command("help", Some("Show help".to_string()));
        assert_eq!(suggestion.id, "help");
        assert_eq!(suggestion.display_text, "/help");
        assert_eq!(suggestion.completion_type, CompletionType::Command);
    }

    #[test]
    fn test_completion_state_show_hide() {
        let mut state = CompletionState::new();

        let suggestions = vec![
            Suggestion::command("help", None),
            Suggestion::command("provider", None),
        ];

        state.show(suggestions, "/h");
        assert!(state.is_visible());
        assert_eq!(state.count(), 2);
        assert_eq!(state.selected_index(), 0);

        state.hide();
        assert!(!state.is_visible());
        assert_eq!(state.count(), 0);
    }

    #[test]
    fn test_completion_state_navigation() {
        let mut state = CompletionState::new();

        let suggestions = vec![
            Suggestion::command("help", None),
            Suggestion::command("provider", None),
            Suggestion::command("model", None),
        ];

        state.show(suggestions, "/");

        // Navigate forward
        state.select_next();
        assert_eq!(state.selected_index(), 1);

        state.select_next();
        assert_eq!(state.selected_index(), 2);

        // Cycle back to first
        state.select_next();
        assert_eq!(state.selected_index(), 0);

        // Navigate backward
        state.select_prev();
        assert_eq!(state.selected_index(), 2);

        state.select_prev();
        assert_eq!(state.selected_index(), 1);
    }

    #[test]
    fn test_input_changed_detection() {
        let mut state = CompletionState::new();

        let suggestions = vec![Suggestion::command("help", None)];
        state.show(suggestions, "/h");

        assert!(!state.input_changed("/h"));
        assert!(state.input_changed("/he"));
        assert!(state.input_changed("/help"));
    }
}
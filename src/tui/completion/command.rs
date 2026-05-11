//! Command completion provider
//!
//! Provides autocomplete for slash commands like /help, /provider, /model

use super::{CompletionProvider, Suggestion};
use crate::commands::CommandRegistry;
use std::sync::Arc;

/// Command completer - provides suggestions for slash commands
pub struct CommandCompleter {
    registry: Arc<CommandRegistry>,
}

impl CommandCompleter {
    /// Create a new command completer with the given registry
    pub fn new(registry: Arc<CommandRegistry>) -> Self {
        Self { registry }
    }

    /// Create with owned registry
    pub fn with_registry(registry: CommandRegistry) -> Self {
        Self {
            registry: Arc::new(registry),
        }
    }

    /// Parse the partial command from input
    fn parse_partial(&self, input: &str, cursor_pos: usize) -> Option<String> {
        // Must start with /
        if !input.starts_with('/') {
            return None;
        }

        // Get text up to cursor
        let text_up_to_cursor = if cursor_pos <= input.len() {
            &input[..cursor_pos]
        } else {
            input
        };

        // Extract command name (after /, before space or end)
        let after_slash = &text_up_to_cursor[1..];
        let partial = after_slash.split_whitespace().next()?;

        Some(partial.to_string())
    }

    /// Get matching commands from registry
    fn get_matching_commands(&self, partial: &str) -> Vec<Arc<dyn crate::commands::Command>> {
        // Get all commands and filter by prefix match
        let all_commands = self.registry.list_commands();

        all_commands
            .into_iter()
            .filter_map(|name| self.registry.get(name))
            .filter(|cmd| {
                // Prefix match on command name
                cmd.name().starts_with(partial)
            })
            .collect()
    }
}

impl CompletionProvider for CommandCompleter {
    fn get_suggestions(&self, input: &str, cursor_pos: usize) -> Vec<Suggestion> {
        // Only complete if starts with /
        if !self.can_complete(input, cursor_pos) {
            return vec![];
        }

        // Parse partial command
        let partial = match self.parse_partial(input, cursor_pos) {
            Some(p) => p,
            None => return vec![],
        };

        let matching_commands = self.get_matching_commands(&partial);

        matching_commands
            .into_iter()
            .map(|cmd| {
                Suggestion::command(
                    cmd.name(),
                    Some(cmd.description()),
                )
            })
            .collect()
    }

    fn can_complete(&self, input: &str, cursor_pos: usize) -> bool {
        // Can complete if:
        // 1. Input starts with /
        // 2. Cursor is after the /
        // 3. No space after / (we're completing command name, not args)

        if !input.starts_with('/') {
            return false;
        }

        // Cursor must be after /
        if cursor_pos < 1 {
            return false;
        }

        // Get text between / and cursor
        let after_slash = if cursor_pos <= input.len() {
            &input[1..cursor_pos]
        } else {
            &input[1..]
        };

        // If there's a space, we're past command name (args completion - not implemented yet)
        !after_slash.contains(' ')
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commands::CommandRegistry;
    use crate::tui::CompletionType;

    #[test]
    fn test_command_completer_creation() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);
        assert!(completer.registry.list_commands().len() > 0);
    }

    #[test]
    fn test_can_complete_slash_input() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);

        // Should complete slash commands
        assert!(completer.can_complete("/", 1));
        assert!(completer.can_complete("/h", 2));
        assert!(completer.can_complete("/help", 5));

        // Should NOT complete non-slash input
        assert!(!completer.can_complete("hello", 5));
        assert!(!completer.can_complete("", 0));

        // Should NOT complete after space (args)
        assert!(!completer.can_complete("/help ", 6));
    }

    #[test]
    fn test_get_suggestions_for_partial() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);

        // Test /h - should match help
        let suggestions = completer.get_suggestions("/h", 2);
        assert!(suggestions.len() > 0);

        // All should be Command type
        for s in &suggestions {
            assert_eq!(s.completion_type, CompletionType::Command);
            assert!(s.display_text.starts_with('/'));
        }

        // Should include /help
        let help_found = suggestions.iter().any(|s| s.id == "help");
        assert!(help_found);
    }

    #[test]
    fn test_get_suggestions_empty_for_non_slash() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);

        let suggestions = completer.get_suggestions("hello", 5);
        assert_eq!(suggestions.len(), 0);
    }

    #[test]
    fn test_get_suggestions_exact_match() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);

        let suggestions = completer.get_suggestions("/help", 5);
        assert_eq!(suggestions.len(), 1);
        assert_eq!(suggestions[0].id, "help");
    }

    #[test]
    fn test_parse_partial_command() {
        let registry = CommandRegistry::new();
        let completer = CommandCompleter::with_registry(registry);

        assert_eq!(completer.parse_partial("/h", 2), Some("h".to_string()));
        assert_eq!(completer.parse_partial("/help", 5), Some("help".to_string()));
        assert_eq!(completer.parse_partial("/prov", 5), Some("prov".to_string()));
        assert_eq!(completer.parse_partial("hello", 5), None);
    }
}
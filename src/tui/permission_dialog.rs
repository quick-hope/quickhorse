//! Permission dialog component for TUI
//!
//! Provides user confirmation UI for permission requests.

#![allow(dead_code)] // Future use: permission request fields

use crate::permissions::{PermissionResult, PermissionUpdate, RuleBehavior, RuleSource, RuleValue};
use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

/// User's choice for permission request
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionChoice {
    /// Allow this operation once
    AllowOnce,
    /// Allow and save rule for future
    AllowAndSave,
    /// Deny this operation
    Deny,
    /// Cancel/close dialog
    Cancel,
}

/// Permission dialog state
pub struct PermissionDialog {
    /// Permission request message
    message: String,
    /// The permission result with details
    result: PermissionResult,
    /// Current selection
    selected: usize,
    /// Available choices
    choices: Vec<PermissionChoice>,
    /// Whether dialog is active
    active: bool,
}

impl PermissionDialog {
    /// Create a new permission dialog
    pub fn new(message: String, result: PermissionResult) -> Self {
        Self {
            message,
            result,
            selected: 0,
            choices: vec![
                PermissionChoice::AllowOnce,
                PermissionChoice::AllowAndSave,
                PermissionChoice::Deny,
                PermissionChoice::Cancel,
            ],
            active: true,
        }
    }

    /// Check if dialog is active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Close the dialog
    pub fn close(&mut self) {
        self.active = false;
    }

    /// Get current selected choice
    pub fn selected_choice(&self) -> PermissionChoice {
        self.choices[self.selected]
    }

    /// Move selection up
    pub fn select_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }

    /// Move selection down
    pub fn select_down(&mut self) {
        if self.selected < self.choices.len() - 1 {
            self.selected += 1;
        }
    }

    /// Confirm selection and return choice
    pub fn confirm(&mut self) -> PermissionChoice {
        let choice = self.selected_choice();
        self.close();
        choice
    }

    /// Get permission updates for AllowAndSave
    pub fn get_save_updates(&self, tool_name: &str, operation: &str) -> Option<PermissionUpdate> {
        if self.selected_choice() == PermissionChoice::AllowAndSave {
            Some(PermissionUpdate::AddRules {
                destination: RuleSource::UserSettings,
                rules: vec![RuleValue {
                    tool_name: tool_name.to_string(),
                    rule_content: Some(operation.to_string()),
                }],
                behavior: RuleBehavior::Allow,
            })
        } else {
            None
        }
    }

    /// Render the dialog
    pub fn render(&self, frame: &mut Frame) {
        if !self.active {
            return;
        }

        // Calculate dialog size
        let area = frame.size();
        let dialog_width = (area.width as usize).min(60).max(40) as u16;
        let dialog_height = 10u16;

        // Center the dialog
        let dialog_area = Rect {
            x: (area.width.saturating_sub(dialog_width)) / 2,
            y: (area.height.saturating_sub(dialog_height)) / 2,
            width: dialog_width,
            height: dialog_height,
        };

        // Clear the area
        frame.render_widget(Clear, dialog_area);

        // Create dialog block
        let block = Block::default()
            .title(" Permission Request ")
            .title_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));

        // Layout for dialog content
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),  // Message
                Constraint::Length(4),  // Choices
                Constraint::Length(1),  // Hint
            ])
            .margin(1)
            .split(block.inner(dialog_area));

        // Render block
        frame.render_widget(block, dialog_area);

        // Render message
        let message_widget = Paragraph::new(self.message.clone())
            .style(Style::default().fg(Color::White))
            .alignment(Alignment::Center)
            .wrap(ratatui::widgets::Wrap { trim: true });
        frame.render_widget(message_widget, chunks[0]);

        // Render choices
        let choices_text: Vec<Line> = self.choices
            .iter()
            .enumerate()
            .map(|(i, choice)| {
                let label = match choice {
                    PermissionChoice::AllowOnce => "✓ Allow (once)",
                    PermissionChoice::AllowAndSave => "✓ Allow & save rule",
                    PermissionChoice::Deny => "✗ Deny",
                    PermissionChoice::Cancel => "↩ Cancel",
                };
                let style = if i == self.selected {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::Gray)
                };
                Line::from(Span::styled(label, style))
            })
            .collect();

        let choices_widget = Paragraph::new(choices_text)
            .alignment(Alignment::Left);
        frame.render_widget(choices_widget, chunks[1]);

        // Render hint
        let hint_text = "↑↓ Select  Enter Confirm  Esc Cancel";
        let hint_widget = Paragraph::new(hint_text)
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center);
        frame.render_widget(hint_widget, chunks[2]);
    }
}

/// Permission request widget for inline display
pub struct PermissionRequestWidget {
    /// Request message
    message: String,
    /// Tool name
    tool_name: String,
    /// Operation description
    operation: String,
}

impl PermissionRequestWidget {
    /// Create a new permission request widget
    pub fn new(message: String, tool_name: String, operation: String) -> Self {
        Self {
            message,
            tool_name,
            operation,
        }
    }

    /// Render as inline message
    pub fn render_inline(&self, frame: &mut Frame, area: Rect) {
        let text = format!("[?] {} - awaiting confirmation", self.message);

        let widget = Paragraph::new(text)
            .style(Style::default().fg(Color::Yellow))
            .block(Block::default().borders(Borders::LEFT).border_style(Style::default().fg(Color::Yellow)));

        frame.render_widget(widget, area);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dialog_creation() {
        let result = PermissionResult::ask("Test permission");
        let dialog = PermissionDialog::new("Test message".to_string(), result);

        assert!(dialog.is_active());
        assert_eq!(dialog.selected_choice(), PermissionChoice::AllowOnce);
    }

    #[test]
    fn test_dialog_selection() {
        let result = PermissionResult::ask("Test permission");
        let mut dialog = PermissionDialog::new("Test message".to_string(), result);

        dialog.select_down();
        assert_eq!(dialog.selected_choice(), PermissionChoice::AllowAndSave);

        dialog.select_down();
        assert_eq!(dialog.selected_choice(), PermissionChoice::Deny);

        dialog.select_up();
        assert_eq!(dialog.selected_choice(), PermissionChoice::AllowAndSave);
    }

    #[test]
    fn test_dialog_confirm() {
        let result = PermissionResult::ask("Test permission");
        let mut dialog = PermissionDialog::new("Test message".to_string(), result);

        dialog.select_down();
        let choice = dialog.confirm();

        assert_eq!(choice, PermissionChoice::AllowAndSave);
        assert!(!dialog.is_active());
    }

    #[test]
    fn test_save_updates() {
        let result = PermissionResult::ask("Test permission");
        let mut dialog = PermissionDialog::new("Test message".to_string(), result);

        // AllowOnce - no save
        assert!(dialog.get_save_updates("Bash", "git status").is_none());

        // AllowAndSave - has save
        dialog.select_down();
        let update = dialog.get_save_updates("Bash", "git status");
        assert!(update.is_some());

        if let Some(PermissionUpdate::AddRules { destination, rules, behavior }) = update {
            assert_eq!(destination, RuleSource::UserSettings);
            assert_eq!(behavior, RuleBehavior::Allow);
            assert_eq!(rules.len(), 1);
        }
    }
}
//! Command history management
//!
//! Provides:
//! - Command history persistence
//! - History search functionality
//! - History entry management

#![allow(dead_code)] // Future use: Ctrl+R history search

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Maximum number of history entries to keep
pub const MAX_HISTORY_ENTRIES: usize = 1000;

/// History entry representing a past command/message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    /// Unique ID
    pub id: u64,
    /// Timestamp when entry was created
    pub timestamp: DateTime<Utc>,
    /// The command or message content
    pub content: String,
    /// Whether this was a command (starts with /)
    pub is_command: bool,
    /// Optional result summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_summary: Option<String>,
}

/// Command history manager
pub struct CommandHistory {
    /// History entries
    entries: Vec<HistoryEntry>,
    /// Maximum entries to keep
    max_entries: usize,
    /// File path for persistence
    file_path: PathBuf,
    /// Next ID for new entries
    next_id: u64,
}

impl CommandHistory {
    /// Create a new history manager
    pub fn new() -> Self {
        let file_path = Self::default_path();
        Self::with_path(file_path)
    }

    /// Create history manager with custom path
    pub fn with_path(file_path: PathBuf) -> Self {
        Self {
            entries: Vec::new(),
            max_entries: MAX_HISTORY_ENTRIES,
            file_path,
            next_id: 1,
        }
    }

    /// Get default history file path
    pub fn default_path() -> PathBuf {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/".to_string());
        PathBuf::from(home)
            .join(".quickhorse")
            .join("history.json")
    }

    /// Load history from file
    pub fn load() -> Self {
        let file_path = Self::default_path();
        let mut history = Self::with_path(file_path.clone());

        if file_path.exists() {
            if let Ok(content) = fs::read_to_string(&file_path) {
                if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&content) {
                    // Calculate next_id before moving entries
                    let max_id = entries.iter().map(|e| e.id).max().unwrap_or(0);
                    history.entries = entries;
                    history.next_id = max_id + 1;
                }
            }
        }

        history
    }

    /// Save history to file
    pub fn save(&self) -> std::io::Result<()> {
        // Ensure parent directory exists
        if let Some(parent) = self.file_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let content = serde_json::to_string_pretty(&self.entries)?;
        fs::write(&self.file_path, content)
    }

    /// Add a new entry to history
    pub fn add(&mut self, content: String) -> &HistoryEntry {
        // Check if same content already exists (avoid duplicates)
        if self.entries.iter().any(|e| e.content == content) {
            // Return the existing entry
            return self.entries.iter().find(|e| e.content == content).unwrap();
        }

        let is_command = content.starts_with('/');
        let entry = HistoryEntry {
            id: self.next_id,
            timestamp: Utc::now(),
            content,
            is_command,
            result_summary: None,
        };

        self.next_id += 1;
        self.entries.push(entry.clone());

        // Trim if exceeds max
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        // Save to file
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save history: {}", e);
        }

        self.entries.last().unwrap()
    }

    /// Add entry with result summary
    pub fn add_with_result(&mut self, content: String, result_summary: String) -> &HistoryEntry {
        let is_command = content.starts_with('/');
        let entry = HistoryEntry {
            id: self.next_id,
            timestamp: Utc::now(),
            content,
            is_command,
            result_summary: Some(result_summary),
        };

        self.next_id += 1;
        self.entries.push(entry.clone());

        // Trim if exceeds max
        if self.entries.len() > self.max_entries {
            self.entries.remove(0);
        }

        // Save to file
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save history: {}", e);
        }

        self.entries.last().unwrap()
    }

    /// Get all entries
    pub fn entries(&self) -> &[HistoryEntry] {
        &self.entries
    }

    /// Get recent entries
    pub fn recent(&self, count: usize) -> Vec<&HistoryEntry> {
        self.entries.iter().rev().take(count).collect()
    }

    /// Search history by content
    pub fn search(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();

        self.entries
            .iter()
            .rev()
            .filter(|e| e.content.to_lowercase().contains(&query_lower))
            .collect()
    }

    /// Search with fuzzy matching (prefix/suffix)
    pub fn search_fuzzy(&self, query: &str) -> Vec<&HistoryEntry> {
        let query_lower = query.to_lowercase();

        self.entries
            .iter()
            .rev()
            .filter(|e| {
                let content_lower = e.content.to_lowercase();
                // Match prefix, contains, or exact
                content_lower.starts_with(&query_lower)
                    || content_lower.contains(&query_lower)
                    || content_lower == query_lower
            })
            .collect()
    }

    /// Get commands only
    pub fn commands(&self) -> Vec<&HistoryEntry> {
        self.entries
            .iter()
            .rev()
            .filter(|e| e.is_command)
            .collect()
    }

    /// Clear all history
    pub fn clear(&mut self) {
        self.entries.clear();
        self.next_id = 1;
        if let Err(e) = self.save() {
            eprintln!("Warning: Failed to save cleared history: {}", e);
        }
    }

    /// Delete a specific entry by ID
    pub fn delete(&mut self, id: u64) -> bool {
        let idx = self.entries.iter().position(|e| e.id == id);
        if let Some(idx) = idx {
            self.entries.remove(idx);
            if let Err(e) = self.save() {
                eprintln!("Warning: Failed to save history after delete: {}", e);
            }
            true
        } else {
            false
        }
    }

    /// Get entry by ID
    pub fn get(&self, id: u64) -> Option<&HistoryEntry> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// Get history statistics
    pub fn stats(&self) -> HistoryStats {
        let commands = self.entries.iter().filter(|e| e.is_command).count();
        let messages = self.entries.len() - commands;

        HistoryStats {
            total_entries: self.entries.len(),
            commands,
            messages,
            oldest_timestamp: self.entries.first().map(|e| e.timestamp),
            newest_timestamp: self.entries.last().map(|e| e.timestamp),
        }
    }
}

impl Default for CommandHistory {
    fn default() -> Self {
        Self::new()
    }
}

/// History statistics
#[derive(Debug, Clone)]
pub struct HistoryStats {
    pub total_entries: usize,
    pub commands: usize,
    pub messages: usize,
    pub oldest_timestamp: Option<DateTime<Utc>>,
    pub newest_timestamp: Option<DateTime<Utc>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_history_creation() {
        let history = CommandHistory::new();
        assert!(history.entries.is_empty());
    }

    #[test]
    fn test_add_entry() {
        let mut history = CommandHistory::new();
        history.add("test command".to_string());

        assert_eq!(history.entries.len(), 1);
        assert_eq!(history.entries[0].content, "test command");
    }

    #[test]
    fn test_add_command_entry() {
        let mut history = CommandHistory::new();
        history.add("/help".to_string());

        assert!(history.entries[0].is_command);
    }

    #[test]
    fn test_avoid_duplicate() {
        let mut history = CommandHistory::new();
        history.add("test".to_string());
        history.add("test".to_string());

        assert_eq!(history.entries.len(), 1);
    }

    #[test]
    fn test_search() {
        let mut history = CommandHistory::new();
        history.add("hello world".to_string());
        history.add("test command".to_string());
        history.add("another hello".to_string());

        let results = history.search("hello");
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_search_fuzzy() {
        let mut history = CommandHistory::new();
        history.add("/help".to_string());
        history.add("/provider".to_string());
        history.add("/model".to_string());

        let results = history.search_fuzzy("/h");
        assert_eq!(results.len(), 1);
        assert!(results[0].content == "/help");
    }

    #[test]
    fn test_commands_only() {
        let mut history = CommandHistory::new();
        history.add("regular message".to_string());
        history.add("/help".to_string());
        history.add("/provider".to_string());

        let commands = history.commands();
        assert_eq!(commands.len(), 2);
    }

    #[test]
    fn test_recent() {
        let mut history = CommandHistory::new();
        history.add("first".to_string());
        history.add("second".to_string());
        history.add("third".to_string());

        let recent = history.recent(2);
        assert_eq!(recent.len(), 2);
        assert_eq!(recent[0].content, "third");
        assert_eq!(recent[1].content, "second");
    }

    #[test]
    fn test_clear() {
        let mut history = CommandHistory::new();
        history.add("test".to_string());
        history.clear();

        assert!(history.entries.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut history = CommandHistory::new();
        history.add("message".to_string());
        history.add("/help".to_string());

        let stats = history.stats();
        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.commands, 1);
        assert_eq!(stats.messages, 1);
    }

    #[test]
    fn test_persistence() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("history.json");

        // Create and save history
        let mut history = CommandHistory::with_path(path.clone());
        history.add("saved entry".to_string());
        assert_eq!(history.entries.len(), 1);

        // Force save
        history.save().expect("Failed to save");

        // Verify file exists
        assert!(path.exists());

        // Load history from file - use with_path not load()
        let loaded = CommandHistory::with_path(path.clone());
        // Need to manually load since with_path doesn't auto-load
        if path.exists() {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(entries) = serde_json::from_str::<Vec<HistoryEntry>>(&content) {
                    // Verify we got the entry
                    assert_eq!(entries.len(), 1);
                    assert_eq!(entries[0].content, "saved entry");
                    return;
                }
            }
        }

        panic!("Failed to load saved history");
    }
}
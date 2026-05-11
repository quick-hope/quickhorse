//! Path completion provider
//!
//! Provides autocomplete for file system paths like ~/, ./, ../, /path

use super::{CompletionProvider, CompletionType, Suggestion};
use std::cmp::Ordering;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// Path entry type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathEntryType {
    Directory,
    File,
}

/// Path entry for completion
#[derive(Debug, Clone)]
pub struct PathEntry {
    pub name: String,
    pub path: String,
    pub entry_type: PathEntryType,
}

/// Path completer - provides suggestions for file system paths
pub struct PathCompleter {
    /// Current working directory
    cwd: PathBuf,
    /// Include hidden files (starting with .)
    include_hidden: bool,
    /// Include files (not just directories)
    include_files: bool,
    /// Maximum results to return
    max_results: usize,
}

impl PathCompleter {
    /// Create a new path completer with current directory
    pub fn new() -> Self {
        Self {
            cwd: env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
            include_hidden: false,
            include_files: true,
            max_results: 20,
        }
    }

    /// Create with custom working directory
    pub fn with_cwd(cwd: PathBuf) -> Self {
        Self {
            cwd,
            include_hidden: false,
            include_files: true,
            max_results: 20,
        }
    }

    /// Set whether to include hidden files
    pub fn set_include_hidden(&mut self, include: bool) {
        self.include_hidden = include;
    }

    /// Set whether to include files
    pub fn set_include_files(&mut self, include: bool) {
        self.include_files = include;
    }

    /// Check if a token looks like a path
    pub fn is_path_like(token: &str) -> bool {
        token.starts_with("~/")
            || token.starts_with("/")
            || token.starts_with("./")
            || token.starts_with("../")
            || token == "~"
            || token == "."
            || token == ".."
    }

    /// Expand path with home directory and environment variables
    fn expand_path(&self, path: &str) -> PathBuf {
        if path.starts_with("~") {
            // Expand home directory
            let home = env::var("HOME")
                .or_else(|_| env::var("USERPROFILE"))
                .unwrap_or_else(|_| "/".to_string());

            let after_home = if path == "~" {
                ""
            } else if path.starts_with("~/") {
                &path[2..]
            } else if path.starts_with("~") {
                // Handle ~user format (not commonly used)
                &path[1..]
            } else {
                ""
            };

            PathBuf::from(home).join(after_home)
        } else if path.starts_with("./") {
            self.cwd.join(&path[2..])
        } else if path == "." {
            self.cwd.clone()
        } else if path.starts_with("../") {
            // Handle parent directory
            let mut result = self.cwd.clone();
            let mut remaining = &path[..];

            while remaining.starts_with("../") {
                if result.pop() {
                    remaining = &remaining[3..];
                } else {
                    break;
                }
            }

            result.join(remaining)
        } else if path == ".." {
            self.cwd.parent().unwrap_or(&self.cwd).to_path_buf()
        } else if path.starts_with("/") {
            // Absolute path
            PathBuf::from(path)
        } else {
            // Relative path without prefix
            self.cwd.join(path)
        }
    }

    /// Parse partial path into directory and prefix
    fn parse_partial_path(&self, input: &str) -> (PathBuf, String) {
        let expanded = self.expand_path(input);

        // If input ends with /, treat as directory with no prefix
        if input.ends_with('/') {
            return (expanded, String::new());
        }

        // Split into directory and prefix (file name portion)
        let directory = expanded.parent().unwrap_or(&expanded).to_path_buf();
        let prefix = expanded
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

        (directory, prefix)
    }

    /// Scan directory for matching entries
    fn scan_directory(&self, dir: &Path, prefix: &str) -> Vec<PathEntry> {
        if !dir.exists() || !dir.is_dir() {
            return vec![];
        }

        let mut entries = Vec::new();

        if let Ok(dir_entries) = fs::read_dir(dir) {
            for entry in dir_entries.filter_map(|e| e.ok()).take(self.max_results + 50) {
                let name = entry.file_name().to_string_lossy().to_string();

                // Filter hidden files
                if !self.include_hidden && name.starts_with('.') {
                    continue;
                }

                // Filter by prefix (case-insensitive)
                if !name.to_lowercase().starts_with(&prefix.to_lowercase()) {
                    continue;
                }

                let path = entry.path().to_string_lossy().to_string();
                let entry_type = if entry.path().is_dir() {
                    PathEntryType::Directory
                } else if self.include_files {
                    PathEntryType::File
                } else {
                    continue; // Skip files if not including
                };

                entries.push(PathEntry {
                    name,
                    path,
                    entry_type,
                });
            }
        }

        // Sort: directories first, then alphabetically
        entries.sort_by(|a, b| {
            if a.entry_type == PathEntryType::Directory && b.entry_type != PathEntryType::Directory {
                Ordering::Less
            } else if a.entry_type != PathEntryType::Directory && b.entry_type == PathEntryType::Directory {
                Ordering::Greater
            } else {
                a.name.cmp(&b.name)
            }
        });

        entries.truncate(self.max_results);
        entries
    }

    /// Get the directory portion of the input path (for display)
    fn get_dir_portion(input: &str) -> String {
        // Find last separator
        let last_sep = if input.contains('/') {
            input.rfind('/').unwrap_or(0)
        } else {
            0
        };

        if last_sep > 0 {
            input[..last_sep + 1].to_string()
        } else {
            String::new()
        }
    }

    /// Convert path entry to suggestion
    fn entry_to_suggestion(&self, entry: &PathEntry, dir_portion: &str) -> Suggestion {
        // Display text: dir_portion + name + '/' for directories
        let display_text = if entry.entry_type == PathEntryType::Directory {
            format!("{}{}/", dir_portion, entry.name)
        } else {
            format!("{}{}", dir_portion, entry.name)
        };

        // Description: show type
        let description = Some(if entry.entry_type == PathEntryType::Directory {
            "directory".to_string()
        } else {
            "file".to_string()
        });

        // Replace suffix: the portion after the prefix
        // For simplicity, we replace the entire partial name with the full name
        let replace_suffix = if entry.entry_type == PathEntryType::Directory {
            format!("{}{}/", dir_portion, entry.name)
        } else {
            format!("{}{}", dir_portion, entry.name)
        };

        Suggestion::new(
            entry.name.clone(),
            display_text,
            description,
            CompletionType::Path,
            replace_suffix,
        )
    }
}

impl Default for PathCompleter {
    fn default() -> Self {
        Self::new()
    }
}

impl CompletionProvider for PathCompleter {
    fn get_suggestions(&self, input: &str, cursor_pos: usize) -> Vec<Suggestion> {
        // Extract the path portion from input
        // Find where the path starts in the input
        let path_start = self.find_path_start(input, cursor_pos);

        if path_start.is_none() {
            return vec![];
        }

        let start = path_start.unwrap();
        let path_text = &input[start..cursor_pos.min(input.len())];

        // Check if it looks like a path
        if !Self::is_path_like(path_text) {
            return vec![];
        }

        // Parse the partial path
        let (directory, prefix) = self.parse_partial_path(path_text);

        // Scan for matching entries
        let entries = self.scan_directory(&directory, &prefix);

        // Get directory portion for display
        let dir_portion = Self::get_dir_portion(path_text);

        // Convert to suggestions
        entries
            .iter()
            .map(|e| self.entry_to_suggestion(e, &dir_portion))
            .collect()
    }

    fn can_complete(&self, input: &str, cursor_pos: usize) -> bool {
        // Find where the path might start
        let path_start = self.find_path_start(input, cursor_pos);

        if path_start.is_none() {
            return false;
        }

        let start = path_start.unwrap();
        let path_text = &input[start..cursor_pos.min(input.len())];

        Self::is_path_like(path_text)
    }
}

impl PathCompleter {
    /// Find the start position of a path-like token in input
    pub fn find_path_start(&self, input: &str, cursor_pos: usize) -> Option<usize> {
        // Look backwards from cursor to find path prefix
        // Ensure cursor_pos is at a valid UTF-8 boundary
        let safe_cursor_pos = if cursor_pos <= input.len() && input.is_char_boundary(cursor_pos) {
            cursor_pos
        } else {
            // Find the nearest valid boundary before cursor_pos
            let mut pos = cursor_pos.min(input.len());
            while pos > 0 && !input.is_char_boundary(pos) {
                pos -= 1;
            }
            pos
        };

        let text_up_to_cursor = &input[..safe_cursor_pos];

        // Find the last occurrence of path-like prefixes
        // Look for ~/ at start, or /, ./, ../ anywhere

        // Check for ~ prefix (only at start or after space)
        if text_up_to_cursor.starts_with("~") {
            return Some(0);
        }

        // Look for /, ./, ../ in the last part of input
        // Use char_indices to properly handle UTF-8 boundaries
        let char_indices: Vec<(usize, char)> = text_up_to_cursor.char_indices().collect();

        // Find the start of the current "word" (after last space or at beginning)
        // Iterate backwards through character indices
        let mut word_start_idx = 0;
        for i in (0..char_indices.len()).rev() {
            let (byte_pos, c) = char_indices[i];
            if c == ' ' || c == '\n' || c == '\t' {
                // Word starts after this whitespace
                if i + 1 < char_indices.len() {
                    word_start_idx = char_indices[i + 1].0;
                } else {
                    word_start_idx = byte_pos + c.len_utf8();
                }
                break;
            }
            if i == 0 {
                word_start_idx = 0;
            }
        }

        // Check if this word starts with a path prefix
        let word_end = safe_cursor_pos.min(input.len());
        let word = &input[word_start_idx..word_end];

        if word.starts_with("/")
            || word.starts_with("./")
            || word.starts_with("../")
            || word.starts_with("~")
        {
            Some(word_start_idx)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_path_completer_creation() {
        let completer = PathCompleter::new();
        assert!(completer.cwd.exists() || completer.cwd == PathBuf::from("."));
    }

    #[test]
    fn test_is_path_like() {
        assert!(PathCompleter::is_path_like("~"));
        assert!(PathCompleter::is_path_like("~/"));
        assert!(PathCompleter::is_path_like("~/path"));
        assert!(PathCompleter::is_path_like("/"));
        assert!(PathCompleter::is_path_like("/path"));
        assert!(PathCompleter::is_path_like("./"));
        assert!(PathCompleter::is_path_like("./file"));
        assert!(PathCompleter::is_path_like("../"));
        assert!(PathCompleter::is_path_like("../dir"));
        assert!(PathCompleter::is_path_like("."));
        assert!(PathCompleter::is_path_like(".."));

        // Not path-like
        assert!(!PathCompleter::is_path_like("hello"));
        assert!(!PathCompleter::is_path_like("path"));
        assert!(!PathCompleter::is_path_like(""));
    }

    #[test]
    fn test_expand_path_home() {
        let completer = PathCompleter::new();
        let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());

        let expanded = completer.expand_path("~");
        assert!(expanded.to_string_lossy().starts_with(&home));

        let expanded = completer.expand_path("~/src");
        assert!(expanded.to_string_lossy().contains("src"));
    }

    #[test]
    fn test_expand_path_relative() {
        let completer = PathCompleter::new();

        let expanded = completer.expand_path(".");
        assert_eq!(expanded, completer.cwd);

        let expanded = completer.expand_path("./src");
        assert!(expanded.ends_with("src"));

        let expanded = completer.expand_path("../parent");
        // Should be parent of cwd joined with "parent" or just cwd if no parent
        assert!(expanded.ends_with("parent") || expanded == completer.cwd.join("parent"));
    }

    #[test]
    fn test_parse_partial_path() {
        let completer = PathCompleter::new();

        // Path ending with /
        let (dir, prefix) = completer.parse_partial_path("~/src/");
        assert!(prefix.is_empty());
        assert!(dir.to_string_lossy().contains("src"));

        // Path with partial name
        let (dir, prefix) = completer.parse_partial_path("~/src/te");
        assert_eq!(prefix, "te");
        assert!(dir.to_string_lossy().contains("src"));
    }

    #[test]
    fn test_scan_directory() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path();

        // Create test entries
        fs::create_dir(temp_path.join("dir1")).unwrap();
        fs::create_dir(temp_path.join("dir2")).unwrap();
        fs::write(temp_path.join("file1.txt"), "content").unwrap();
        fs::write(temp_path.join("file2.txt"), "content").unwrap();
        fs::write(temp_path.join(".hidden"), "hidden").unwrap();

        let completer = PathCompleter::with_cwd(temp_path.to_path_buf());

        // Scan for all entries (excluding hidden)
        let entries = completer.scan_directory(temp_path, "");
        assert!(entries.len() >= 4); // dir1, dir2, file1.txt, file2.txt

        // Check sorting: directories first
        assert!(entries[0].entry_type == PathEntryType::Directory);

        // Check filtering by prefix
        let entries = completer.scan_directory(temp_path, "dir");
        assert_eq!(entries.len(), 2);
        assert!(entries.iter().all(|e| e.name.starts_with("dir")));

        // Check file filtering
        let mut completer_no_files = PathCompleter::with_cwd(temp_path.to_path_buf());
        completer_no_files.set_include_files(false);
        let entries = completer_no_files.scan_directory(temp_path, "");
        assert!(entries.iter().all(|e| e.entry_type == PathEntryType::Directory));

        // Check hidden files
        let mut completer_hidden = PathCompleter::with_cwd(temp_path.to_path_buf());
        completer_hidden.set_include_hidden(true);
        let entries = completer_hidden.scan_directory(temp_path, "");
        assert!(entries.iter().any(|e| e.name == ".hidden"));
    }

    #[test]
    fn test_get_dir_portion() {
        assert_eq!(PathCompleter::get_dir_portion("~/src/test"), "~/src/");
        assert_eq!(PathCompleter::get_dir_portion("/path/to/file"), "/path/to/");
        assert_eq!(PathCompleter::get_dir_portion("./dir/file"), "./dir/");
        assert_eq!(PathCompleter::get_dir_portion("../parent/file"), "../parent/");
        assert_eq!(PathCompleter::get_dir_portion("~/"), "~/");
        assert_eq!(PathCompleter::get_dir_portion("~/test"), "~/");
    }

    #[test]
    fn test_entry_to_suggestion() {
        let completer = PathCompleter::new();

        let dir_entry = PathEntry {
            name: "src".to_string(),
            path: "/home/user/src".to_string(),
            entry_type: PathEntryType::Directory,
        };

        let suggestion = completer.entry_to_suggestion(&dir_entry, "~/");
        assert_eq!(suggestion.display_text, "~/src/");
        assert_eq!(suggestion.description, Some("directory".to_string()));
        assert_eq!(suggestion.completion_type, CompletionType::Path);

        let file_entry = PathEntry {
            name: "file.txt".to_string(),
            path: "/home/user/file.txt".to_string(),
            entry_type: PathEntryType::File,
        };

        let suggestion = completer.entry_to_suggestion(&file_entry, "~/");
        assert_eq!(suggestion.display_text, "~/file.txt");
        assert_eq!(suggestion.description, Some("file".to_string()));
    }

    #[test]
    fn test_find_path_start() {
        let completer = PathCompleter::new();

        // Path at beginning
        assert_eq!(completer.find_path_start("~/test", 6), Some(0));
        assert_eq!(completer.find_path_start("/path", 5), Some(0));

        // Path after space
        assert_eq!(completer.find_path_start("read ~/src", 10), Some(5));
        assert_eq!(completer.find_path_start("cd ./dir", 8), Some(3));

        // Not a path
        assert_eq!(completer.find_path_start("hello world", 5), None);
        assert_eq!(completer.find_path_start("just text", 9), None);
    }

    #[test]
    fn test_can_complete() {
        let completer = PathCompleter::new();

        assert!(completer.can_complete("~/src", 5));
        assert!(completer.can_complete("/path/to", 8));
        assert!(completer.can_complete("./file", 6));
        assert!(completer.can_complete("../dir", 6));

        // Not path-like
        assert!(!completer.can_complete("hello", 5));
        assert!(!completer.can_complete("", 0));
    }

    #[test]
    fn test_find_path_start_with_chinese() {
        let completer = PathCompleter::new();

        // Chinese characters before path (UTF-8 multi-byte)
        // "您好 ~/test" - 您好 is 6 bytes (2 chars * 3 bytes each), space is 1 byte
        // Byte positions: 您(0-2), 好(3-5), space(6), ~(7), /test(8-12)
        let input = "您好 ~/test";
        // Cursor at position 9 (after "您好 ")
        assert_eq!(completer.find_path_start(input, 9), Some(7));

        // "你好 /path" - 你好 is 6 bytes, space is 1 byte
        // Byte positions: 你(0-2), 好(3-5), space(6), /(7), path(8-11)
        let input2 = "你好 /path";
        assert_eq!(completer.find_path_start(input2, 8), Some(7));

        // Chinese only, no path - should return None
        let input3 = "您好您好";
        // '您' is 3 bytes, cursor at byte 3 (end of first char)
        // This should NOT panic
        let result = completer.find_path_start(input3, 3);
        assert_eq!(result, None);

        // Test all valid UTF-8 boundary positions (should not panic)
        let input4 = "您好世界";
        // Valid boundaries: 0, 3, 6, 9, 12 (each Chinese char is 3 bytes)
        for pos in [0, 3, 6, 9, 12] {
            // Should not panic at valid boundary positions
            let _ = completer.find_path_start(input4, pos);
        }

        // Test invalid boundary positions (inside multi-byte chars)
        // Should handle gracefully without panic
        for pos in 1..=input4.len() {
            // Should not panic even at invalid byte positions
            let _ = completer.find_path_start(input4, pos);
        }
    }
}
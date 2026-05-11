//! WriteTool - Create or overwrite files
//!
//! Creates new files or completely overwrites existing files.
//! Requires write permission and tracks modifications.

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

use crate::permissions::{PermissionResult, PermissionBehavior, PermissionMode};
use crate::tools::{Tool, ToolContext, ToolResult, build_schema};

/// WriteTool input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct WriteInput {
    /// The absolute path to the file to write (must be absolute, not relative)
    pub file_path: String,
    /// The content to write to the file
    pub content: String,
}

/// WriteTool output result
#[derive(Debug, Clone, Serialize)]
pub struct WriteOutput {
    /// Operation type: "create" or "update"
    #[serde(rename = "type")]
    pub operation_type: String,
    /// File path that was written
    pub file_path: String,
    /// Content that was written
    pub content: String,
    /// Number of lines added
    pub lines_added: u32,
    /// Number of lines removed (for updates)
    pub lines_removed: u32,
    /// Original file content (for updates)
    pub original_content: Option<String>,
}

/// WriteTool implementation
pub struct WriteTool;

impl WriteTool {
    /// Create a new WriteTool instance
    pub fn new() -> Self {
        Self
    }

    /// Expand path with home directory support
    fn expand_path(path: &str) -> PathBuf {
        if path.starts_with("~") {
            let home = std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| "/".to_string());
            let after_home = if path == "~" {
                ""
            } else if path.starts_with("~/") {
                &path[2..]
            } else {
                &path[1..]
            };
            PathBuf::from(home).join(after_home)
        } else if path.starts_with("./") {
            std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join(&path[2..])
        } else {
            PathBuf::from(path)
        }
    }

    /// Check write permission for the file path
    fn check_write_permission(path: &PathBuf, mode: PermissionMode) -> PermissionResult {
        // In BypassPermissions mode, allow all writes
        if mode == PermissionMode::BypassPermissions {
            return PermissionResult::allow("Bypass permissions mode");
        }

        // In AcceptEdits mode, allow all writes
        if mode == PermissionMode::AcceptEdits {
            return PermissionResult::allow("Accept edits mode");
        }

        // Check if path exists - if not, allow creation
        if !path.exists() {
            return PermissionResult::allow("Creating new file");
        }

        // For existing files, check if in current working directory
        let cwd = std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("/"));

        if path.starts_with(&cwd) {
            PermissionResult::allow("Writing to file in current directory")
        } else {
            PermissionResult::ask("Writing to file outside current directory")
        }
    }

    /// Count lines in content
    fn count_lines(content: &str) -> u32 {
        if content.is_empty() {
            0
        } else {
            content.lines().count() as u32
        }
    }

    /// Read existing file content
    fn read_existing_file(path: &PathBuf) -> Option<String> {
        if path.exists() && path.is_file() {
            fs::read_to_string(path).ok()
        } else {
            None
        }
    }
}

impl Default for WriteTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WriteTool {
    fn name(&self) -> &str {
        "Write"
    }

    fn description(&self) -> String {
        "Write a file to the local filesystem. Creates a new file if it doesn't exist, or completely overwrites an existing file with new content. Use this tool when you want to create new files or replace entire file contents.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<WriteInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        // Parse input
        let write_input: WriteInput = serde_json::from_value(input.clone())
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid input: {}", e)
            )) as Box<dyn std::error::Error + Send + Sync>)?;

        // Expand path
        let full_path = Self::expand_path(&write_input.file_path);

        // Check permission
        let permission = Self::check_write_permission(&full_path, context.permission_mode);
        if permission.behavior == PermissionBehavior::Deny {
            return Ok(ToolResult::error(format!(
                "Permission denied: {}",
                permission.message
            )));
        }

        // Read existing content if file exists
        let original_content = Self::read_existing_file(&full_path);

        // Create parent directories if needed
        if let Some(parent) = full_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .map_err(|e| Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        format!("Failed to create parent directory: {}", e)
                    )) as Box<dyn std::error::Error + Send + Sync>)?;
            }
        }

        // Write file
        fs::write(&full_path, &write_input.content)
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to write file: {}", e)
            )) as Box<dyn std::error::Error + Send + Sync>)?;

        // Calculate line changes
        let lines_added = Self::count_lines(&write_input.content);
        let lines_removed = if let Some(original) = &original_content {
            Self::count_lines(original)
        } else {
            0
        };

        // Determine operation type
        let is_update = original_content.is_some();
        let operation_type = if is_update {
            "update"
        } else {
            "create"
        };

        // Format result message
        let message = match operation_type {
            "create" => format!(
                "File created successfully at: {} ({} lines)",
                write_input.file_path, lines_added
            ),
            "update" => format!(
                "File updated: {} (+{} lines, -{} lines)",
                write_input.file_path,
                lines_added,
                lines_removed
            ),
            _ => format!("File written: {}", write_input.file_path),
        };

        // Create output (for future use in structured responses)
        let _output = WriteOutput {
            operation_type: operation_type.to_string(),
            file_path: write_input.file_path.clone(),
            content: write_input.content.clone(),
            lines_added,
            lines_removed,
            original_content,
        };

        Ok(ToolResult::success(message))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false // Write tool is never read-only
    }

    fn check_permissions(&self, input: &serde_json::Value) -> PermissionResult {
        // Parse input to get file path
        if let Ok(write_input) = serde_json::from_value::<WriteInput>(input.clone()) {
            let full_path = Self::expand_path(&write_input.file_path);
            Self::check_write_permission(&full_path, PermissionMode::Default)
        } else {
            PermissionResult::deny("Invalid input for permission check")
        }
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(write_input) = serde_json::from_value::<WriteInput>(input.clone()) {
            let path = &write_input.file_path;
            let lines = write_input.content.lines().count();
            format!("Writing {} lines to {}", lines, path)
        } else {
            "Writing file".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_context() -> ToolContext {
        ToolContext::default()
    }

    #[test]
    fn test_expand_path_home() {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/".to_string());
        let path = WriteTool::expand_path("~/src/test.txt");
        assert!(path.to_string_lossy().starts_with(&home));
        assert!(path.to_string_lossy().contains("src/test.txt"));
    }

    #[test]
    fn test_expand_path_relative() {
        let cwd = std::env::current_dir().unwrap();
        let path = WriteTool::expand_path("./test.txt");
        assert!(path.starts_with(&cwd));
        assert!(path.ends_with("test.txt"));
    }

    #[test]
    fn test_expand_path_absolute() {
        let path = WriteTool::expand_path("/usr/local/test.txt");
        assert_eq!(path, PathBuf::from("/usr/local/test.txt"));
    }

    #[test]
    fn test_count_lines_empty() {
        assert_eq!(WriteTool::count_lines(""), 0);
    }

    #[test]
    fn test_count_lines_single() {
        assert_eq!(WriteTool::count_lines("Hello"), 1);
    }

    #[test]
    fn test_count_lines_multiple() {
        assert_eq!(WriteTool::count_lines("Line1\nLine2\nLine3"), 3);
    }

    #[tokio::test]
    async fn test_write_create_new_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("new_file.txt").to_string_lossy().to_string();

        let input = serde_json::json!({
            "file_path": path,
            "content": "Hello, World!"
        });

        let tool = WriteTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("File created successfully"));

        // Verify file was created
        let file_path = temp_dir.path().join("new_file.txt");
        assert!(file_path.exists());
        let content = fs::read_to_string(&file_path).unwrap();
        assert_eq!(content, "Hello, World!");
    }

    #[tokio::test]
    async fn test_write_update_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("existing.txt");

        // Create existing file
        fs::write(&path, "Old content").unwrap();

        let input = serde_json::json!({
            "file_path": path.to_string_lossy().to_string(),
            "content": "New content"
        });

        let tool = WriteTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("File updated"));

        // Verify file was updated
        let content = fs::read_to_string(&path).unwrap();
        assert_eq!(content, "New content");
    }

    #[tokio::test]
    async fn test_write_creates_parent_directory() {
        let temp_dir = TempDir::new().unwrap();
        let path = temp_dir.path().join("subdir/nested/deep/file.txt");

        let input = serde_json::json!({
            "file_path": path.to_string_lossy().to_string(),
            "content": "Nested file content"
        });

        let tool = WriteTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);

        // Verify nested directory was created
        assert!(path.exists());
        assert!(temp_dir.path().join("subdir/nested/deep").exists());
    }

    #[test]
    fn test_summarize() {
        let input = serde_json::json!({
            "file_path": "/test/path.txt",
            "content": "Line1\nLine2\nLine3"
        });

        let tool = WriteTool::new();
        let summary = tool.summarize(&input);
        assert!(summary.contains("Writing 3 lines"));
        assert!(summary.contains("/test/path.txt"));
    }
}
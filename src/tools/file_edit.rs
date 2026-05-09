//! FileEditTool - Precise text replacement in files

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult, PermissionResult};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs;

/// FileEditTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileEditInput {
    /// The absolute path to the file to edit
    pub file_path: String,
    /// The text to search for - must match exactly
    pub old_string: String,
    /// The text to replace with
    pub new_string: String,
    /// Replace all occurrences (default: false, replaces first occurrence only)
    #[serde(default)]
    pub replace_all: bool,
}

/// FileEditTool - Perform precise string replacements in files
pub struct FileEditTool;

impl FileEditTool {
    /// Create a new FileEditTool instance
    pub fn new() -> Self {
        Self
    }

    /// Resolve path relative to cwd
    fn resolve_path(file_path: &str, cwd: &str) -> String {
        let path = Path::new(file_path);
        if path.is_absolute() {
            file_path.to_string()
        } else {
            Path::new(cwd).join(file_path).to_string_lossy().to_string()
        }
    }

    /// Find occurrences of old_string in content
    fn find_occurrences(content: &str, old_string: &str) -> Vec<usize> {
        let mut positions = Vec::new();
        let mut start = 0;

        while let Some(pos) = content[start..].find(old_string) {
            positions.push(start + pos);
            start += pos + old_string.len();
        }

        positions
    }
}

impl Default for FileEditTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileEditTool {
    fn name(&self) -> &str {
        "Edit"
    }

    fn description(&self) -> String {
        "Perform precise string replacements in files. Use for editing code, configs, etc. Must match old_string exactly.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<FileEditInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let edit_input: FileEditInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        // Validate old_string is not empty
        if edit_input.old_string.is_empty() {
            return Ok(ToolResult::error("old_string cannot be empty".to_string()));
        }

        // Validate old_string != new_string
        if edit_input.old_string == edit_input.new_string {
            return Ok(ToolResult::error("old_string and new_string are identical, no changes needed".to_string()));
        }

        let full_path = Self::resolve_path(&edit_input.file_path, &context.cwd);
        let path = Path::new(&full_path);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File does not exist: {}",
                edit_input.file_path
            )));
        }

        // Check if it's a file
        if !path.is_file() {
            return Ok(ToolResult::error(format!(
                "Path is not a file: {}",
                edit_input.file_path
            )));
        }

        // Read file content
        let content = fs::read_to_string(path).await
            .map_err(|e| format!("Failed to read file: {}", e))?;

        // Find occurrences
        let occurrences = Self::find_occurrences(&content, &edit_input.old_string);

        if occurrences.is_empty() {
            return Ok(ToolResult::error(format!(
                "Could not find old_string in file. Make sure it matches exactly, including whitespace and line breaks.\nFile: {}",
                edit_input.file_path
            )));
        }

        // Check for uniqueness (unless replace_all)
        if !edit_input.replace_all && occurrences.len() > 1 {
            return Ok(ToolResult::error(format!(
                "Found {} occurrences of old_string. Use replace_all=true to replace all, or provide a more specific old_string to replace exactly one occurrence.\nOccurrences at positions: {}",
                occurrences.len(),
                occurrences.iter().map(|p| p.to_string()).collect::<Vec<_>>().join(", ")
            )));
        }

        // Perform replacement
        let new_content = if edit_input.replace_all {
            content.replace(&edit_input.old_string, &edit_input.new_string)
        } else {
            // Replace first occurrence only
            if let Some(pos) = occurrences.first() {
                let mut result = String::new();
                result.push_str(&content[..*pos]);
                result.push_str(&edit_input.new_string);
                result.push_str(&content[*pos + edit_input.old_string.len()..]);
                result
            } else {
                content
            }
        };

        // Write back to file
        fs::write(path, &new_content).await
            .map_err(|e| format!("Failed to write file: {}", e))?;

        let replacement_count = if edit_input.replace_all {
            occurrences.len()
        } else {
            1
        };

        Ok(ToolResult::success(format!(
            "Successfully edited {} ({} replacement(s))\nold_string → new_string:\n  {}\n  {}",
            edit_input.file_path,
            replacement_count,
            edit_input.old_string.lines().collect::<Vec<_>>().first().unwrap_or(&""),
            edit_input.new_string.lines().collect::<Vec<_>>().first().unwrap_or(&"")
        )))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false
    }

    fn check_permissions(&self, input: &serde_json::Value) -> PermissionResult {
        if let Ok(edit_input) = serde_json::from_value::<FileEditInput>(input.clone()) {
            PermissionResult::Ask(format!(
                "Edit file '{}': replace '{}' with '{}'?",
                edit_input.file_path,
                edit_input.old_string.lines().next().unwrap_or(&""),
                edit_input.new_string.lines().next().unwrap_or(&"")
            ))
        } else {
            PermissionResult::Allow
        }
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(edit_input) = serde_json::from_value::<FileEditInput>(input.clone()) {
            format!("Editing: {}", edit_input.file_path)
        } else {
            "Editing file".to_string()
        }
    }
}
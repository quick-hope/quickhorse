//! FileReadTool - Read file contents

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

/// FileReadTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct FileReadInput {
    /// The absolute path to the file to read
    pub file_path: String,
    /// The line number to start reading from (1-indexed, default 1)
    #[serde(default = "default_offset")]
    pub offset: u32,
    /// The number of lines to read (optional, reads entire file if not specified)
    #[serde(default)]
    pub limit: Option<u32>,
}

fn default_offset() -> u32 {
    1
}

/// FileReadTool - Read file contents with optional offset and limit
pub struct FileReadTool;

impl FileReadTool {
    /// Create a new FileReadTool instance
    pub fn new() -> Self {
        Self
    }

    /// Format file content with line numbers
    fn format_with_line_numbers(content: &str, start_line: u32) -> String {
        content
            .lines()
            .enumerate()
            .map(|(i, line)| format!("{:>6}\t{}", start_line + i as u32, line))
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// Truncate content if too large
    fn truncate(content: &str, max_chars: usize) -> String {
        if content.len() > max_chars {
            format!("{}... (truncated, {} chars omitted)",
                &content[..max_chars],
                content.len() - max_chars)
        } else {
            content.to_string()
        }
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
}

impl Default for FileReadTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for FileReadTool {
    fn name(&self) -> &str {
        "Read"
    }

    fn description(&self) -> String {
        "Read file contents. Use for viewing source code, config files, logs, etc. Supports offset and limit for large files.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<FileReadInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let read_input: FileReadInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        let full_path = Self::resolve_path(&read_input.file_path, &context.cwd);
        let path = Path::new(&full_path);

        // Check if file exists
        if !path.exists() {
            return Ok(ToolResult::error(format!(
                "File does not exist: {}",
                read_input.file_path
            )));
        }

        // Check if it's a file (not a directory)
        if !path.is_file() {
            return Ok(ToolResult::error(format!(
                "Path is not a file: {}",
                read_input.file_path
            )));
        }

        // Read file
        let file = fs::File::open(path).await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        let mut lines = reader.lines();

        // Skip to offset
        let start_line = read_input.offset;
        let mut current_line = 1;
        let mut content_lines: Vec<String> = Vec::new();

        while let Ok(Some(line)) = lines.next_line().await {
            if current_line >= start_line {
                content_lines.push(line);
            }
            current_line += 1;

            // Check limit
            if let Some(limit) = read_input.limit {
                if content_lines.len() >= limit as usize {
                    break;
                }
            }
        }

        let total_lines = current_line - 1;

        if content_lines.is_empty() {
            return Ok(ToolResult::success(format!(
                "File {} has {} lines. Offset {} is beyond file length.",
                read_input.file_path, total_lines, start_line
            )));
        }

        // Format with line numbers
        let content = Self::format_with_line_numbers(&content_lines.join("\n"), start_line);
        let truncated = Self::truncate(&content, 10000);

        // Add metadata
        let result = format!(
            "File: {} ({} lines total, showing lines {}-{})\n\n{}",
            read_input.file_path,
            total_lines,
            start_line,
            start_line + content_lines.len() as u32 - 1,
            truncated
        );

        Ok(ToolResult::success(result))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(read_input) = serde_json::from_value::<FileReadInput>(input.clone()) {
            format!("Reading: {}", read_input.file_path)
        } else {
            "Reading file".to_string()
        }
    }
}
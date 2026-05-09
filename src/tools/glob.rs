//! GlobTool - File pattern matching using glob patterns

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use glob::glob;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;

/// GlobTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GlobInput {
    /// The glob pattern to match files (e.g., "**/*.rs", "src/**/*.ts")
    pub pattern: String,
    /// The directory to search from (default: current working directory)
    #[serde(default)]
    pub path: Option<String>,
}

/// GlobTool - Find files matching glob patterns
pub struct GlobTool;

impl GlobTool {
    /// Create a new GlobTool instance
    pub fn new() -> Self {
        Self
    }

    /// Resolve path relative to cwd
    fn resolve_base_path(path: Option<&str>, cwd: &str) -> String {
        match path {
            Some(p) => {
                let p_path = Path::new(p);
                if p_path.is_absolute() {
                    p.to_string()
                } else {
                    Path::new(cwd).join(p).to_string_lossy().to_string()
                }
            }
            None => cwd.to_string(),
        }
    }

    /// Build full glob pattern with base path
    fn build_pattern(pattern: &str, base_path: &str) -> String {
        let base = Path::new(base_path);
        if pattern.starts_with('/') || pattern.starts_with('~') {
            pattern.to_string()
        } else {
            base.join(pattern).to_string_lossy().to_string()
        }
    }

    /// Format results
    fn format_results(files: &[String], pattern: &str, truncated: bool) -> String {
        if files.is_empty() {
            format!("No files found matching pattern: {}", pattern)
        } else {
            let file_list = files
                .iter()
                .map(|f| format!("  {}", f))
                .collect::<Vec<_>>()
                .join("\n");

            if truncated {
                format!("Found {}+ files matching: {}\n{}\n... (results truncated)", files.len(), pattern, file_list)
            } else {
                format!("Found {} files matching: {}\n{}", files.len(), pattern, file_list)
            }
        }
    }
}

impl Default for GlobTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GlobTool {
    fn name(&self) -> &str {
        "Glob"
    }

    fn description(&self) -> String {
        "Find files matching glob patterns. Use for discovering source files, configs, etc. Supports patterns like **/*.rs, src/**/*.ts.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<GlobInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let glob_input: GlobInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        // Validate pattern is not empty
        if glob_input.pattern.is_empty() {
            return Ok(ToolResult::error("Pattern cannot be empty".to_string()));
        }

        let base_path = Self::resolve_base_path(glob_input.path.as_deref(), &context.cwd);
        let full_pattern = Self::build_pattern(&glob_input.pattern, &base_path);

        // Execute glob
        let paths: Vec<String> = glob(&full_pattern)
            .map_err(|e| format!("Invalid glob pattern: {}", e))?
            .filter_map(|entry| entry.ok())
            .map(|path| path.to_string_lossy().to_string())
            .collect();

        // Limit results to prevent overwhelming output
        const MAX_RESULTS: usize = 100;
        let truncated = paths.len() > MAX_RESULTS;
        let display_paths: Vec<String> = paths.into_iter().take(MAX_RESULTS).collect();

        let result = Self::format_results(&display_paths, &glob_input.pattern, truncated);

        Ok(ToolResult::success(result))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(glob_input) = serde_json::from_value::<GlobInput>(input.clone()) {
            format!("Searching: {}", glob_input.pattern)
        } else {
            "Searching files".to_string()
        }
    }
}
//! GrepTool - File content search using regex

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use regex::Regex;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::path::Path;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};
use glob::glob;

/// GrepTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GrepInput {
    /// The regex pattern to search for in file contents
    pub pattern: String,
    /// The directory or file to search (default: current working directory)
    #[serde(default)]
    pub path: Option<String>,
    /// Glob pattern to filter files (e.g., "*.rs", "**/*.ts")
    #[serde(default)]
    pub glob: Option<String>,
    /// Case insensitive search
    #[serde(default)]
    pub ignore_case: bool,
    /// Show line numbers in output
    #[serde(default = "default_show_lines")]
    pub show_lines: bool,
    /// Number of context lines to show before/after matches
    #[serde(default)]
    pub context: Option<u32>,
    /// Maximum number of matches to return
    #[serde(default)]
    pub head_limit: Option<u32>,
}

fn default_show_lines() -> bool {
    true
}

/// GrepTool - Search file contents using regex patterns
pub struct GrepTool;

impl GrepTool {
    /// Create a new GrepTool instance
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

    /// Build regex with options
    fn build_regex(pattern: &str, ignore_case: bool) -> Result<Regex, regex::Error> {
        let mut builder = regex::RegexBuilder::new(pattern);
        if ignore_case {
            builder.case_insensitive(true);
        }
        builder.build()
    }

    /// Get files to search based on path and glob filter
    fn get_files_to_search(base_path: &str, glob_pattern: Option<&str>) -> Vec<String> {
        let mut files: Vec<String> = Vec::new();

        if let Some(glob_filter) = glob_pattern {
            // Use glob pattern to find files
            let full_glob = if Path::new(glob_filter).is_absolute() {
                glob_filter.to_string()
            } else {
                Path::new(base_path).join(glob_filter).to_string_lossy().to_string()
            };

            if let Ok(paths) = glob(&full_glob) {
                for entry in paths.flatten() {
                    if entry.is_file() {
                        files.push(entry.to_string_lossy().to_string());
                    }
                }
            }
        } else {
            // Search all files in directory recursively
            let search_pattern = Path::new(base_path).join("**/*").to_string_lossy().to_string();
            if let Ok(paths) = glob(&search_pattern) {
                for entry in paths.flatten() {
                    if entry.is_file() {
                        // Skip hidden files and common non-text directories
                        let path_str = entry.to_string_lossy().to_string();
                        if !path_str.contains("/.git/") &&
                           !path_str.contains("/node_modules/") &&
                           !path_str.contains("/target/") &&
                           !path_str.contains(".DS_Store") {
                            files.push(path_str);
                        }
                    }
                }
            }
        }

        files
    }

    /// Search a single file for matches
    async fn search_file(
        file_path: &str,
        regex: &Regex,
        _show_lines: bool,
        context: Option<u32>,
    ) -> Result<Vec<MatchResult>, Box<dyn Error + Send + Sync>> {
        let mut matches: Vec<MatchResult> = Vec::new();

        let file = fs::File::open(file_path).await
            .map_err(|e| format!("Failed to open file: {}", e))?;

        let reader = BufReader::new(file);
        let mut lines: Vec<(u32, String)> = Vec::new();
        let mut line_num = 1u32;

        // Read all lines
        let mut lines_reader = reader.lines();
        while let Ok(Some(line)) = lines_reader.next_line().await {
            lines.push((line_num, line));
            line_num += 1;
        }

        // Search for matches
        for (idx, (num, line)) in lines.iter().enumerate() {
            if regex.is_match(line) {
                let mut context_before: Vec<(u32, String)> = Vec::new();
                let mut context_after: Vec<(u32, String)> = Vec::new();

                if let Some(ctx) = context {
                    // Get context before
                    for i in (idx.saturating_sub(ctx as usize)).min(idx)..idx {
                        context_before.push(lines[i].clone());
                    }
                    // Get context after
                    for i in (idx + 1)..(idx + 1 + ctx as usize).min(lines.len()) {
                        context_after.push(lines[i].clone());
                    }
                }

                matches.push(MatchResult {
                    file_path: file_path.to_string(),
                    line_number: *num,
                    line_content: line.clone(),
                    context_before,
                    context_after,
                });
            }
        }

        Ok(matches)
    }

    /// Format match results for output
    fn format_results(matches: &[MatchResult], show_lines: bool, truncated: bool) -> String {
        if matches.is_empty() {
            return "No matches found".to_string();
        }

        let mut output = Vec::new();

        for m in matches {
            output.push(format!("{}:{}", m.file_path, m.line_number));
            if show_lines {
                // Context before
                for (num, line) in &m.context_before {
                    output.push(format!("  {:>6}-\t{}", num, line));
                }
                // Match line
                output.push(format!("  {:>6}\t{}", m.line_number, m.line_content));
                // Context after
                for (num, line) in &m.context_after {
                    output.push(format!("  {:>6}-\t{}", num, line));
                }
            }
        }

        let result_str = output.join("\n");
        if truncated {
            format!("{}... (results truncated)", result_str)
        } else {
            format!("Found {} matches:\n{}", matches.len(), result_str)
        }
    }
}

impl Default for GrepTool {
    fn default() -> Self {
        Self::new()
    }
}

/// Single match result
struct MatchResult {
    file_path: String,
    line_number: u32,
    line_content: String,
    context_before: Vec<(u32, String)>,
    context_after: Vec<(u32, String)>,
}

#[async_trait]
impl Tool for GrepTool {
    fn name(&self) -> &str {
        "Grep"
    }

    fn description(&self) -> String {
        "Search file contents using regex patterns. Use for finding code symbols, keywords, patterns across files. Supports glob filtering.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<GrepInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let grep_input: GrepInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        // Validate pattern
        if grep_input.pattern.is_empty() {
            return Ok(ToolResult::error("Pattern cannot be empty".to_string()));
        }

        // Build regex
        let regex = Self::build_regex(&grep_input.pattern, grep_input.ignore_case)
            .map_err(|e| format!("Invalid regex pattern: {}", e))?;

        let base_path = Self::resolve_base_path(grep_input.path.as_deref(), &context.cwd);

        // Get files to search
        let files = Self::get_files_to_search(&base_path, grep_input.glob.as_deref());

        if files.is_empty() {
            return Ok(ToolResult::success("No files found to search".to_string()));
        }

        // Search all files
        let mut all_matches: Vec<MatchResult> = Vec::new();
        const MAX_MATCHES: usize = 100;
        let max_matches = grep_input.head_limit
            .map(|l| l as usize)
            .unwrap_or(MAX_MATCHES);

        for file_path in files {
            if all_matches.len() >= max_matches {
                break;
            }

            let file_matches = Self::search_file(
                &file_path,
                &regex,
                grep_input.show_lines,
                grep_input.context,
            ).await?;

            for m in file_matches {
                if all_matches.len() >= max_matches {
                    break;
                }
                all_matches.push(m);
            }
        }

        let truncated = all_matches.len() >= max_matches;
        let result = Self::format_results(&all_matches, grep_input.show_lines, truncated);

        Ok(ToolResult::success(result))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(grep_input) = serde_json::from_value::<GrepInput>(input.clone()) {
            format!("Searching for: {}", grep_input.pattern)
        } else {
            "Searching content".to_string()
        }
    }
}
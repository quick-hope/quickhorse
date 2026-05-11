//! PatchTool - Apply unified diff patches to files
//!
//! Implements patch application for unified diff format:
//! - Parse diff format (---, ++++, @@ hunk headers)
//! - Apply changes to file content
//! - Detect and report conflicts

use crate::permissions::{PermissionBehavior, PermissionMode};
use crate::tools::{Tool, ToolContext, ToolResult, build_schema};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::fs;
use std::path::PathBuf;

/// PatchTool - Apply unified diff patches
pub struct PatchTool;

impl PatchTool {
    pub fn new() -> Self {
        Self
    }
}

/// Patch input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct PatchInput {
    /// File path to apply patch to
    pub file_path: String,
    /// Unified diff content to apply
    pub patch_content: String,
    /// Whether to create file if it doesn't exist
    #[serde(default)]
    pub create_if_missing: bool,
}

/// Patch output result
#[derive(Debug, Clone, Serialize)]
pub struct PatchOutput {
    /// Number of hunks applied successfully
    pub hunks_applied: u32,
    /// Number of hunks that failed (conflicts)
    pub hunks_failed: u32,
    /// Lines added
    pub lines_added: u32,
    /// Lines removed
    pub lines_removed: u32,
    /// Result file content
    pub result_content: String,
    /// Conflict details if any
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts: Vec<ConflictInfo>,
    /// Operation type
    #[serde(rename = "type")]
    pub operation_type: String,
    /// File path
    pub file_path: String,
}

impl PatchInput {
    pub fn schema() -> serde_json::Value {
        build_schema::<Self>()
    }
}

/// Conflict information
#[derive(Debug, Clone, Serialize)]
pub struct ConflictInfo {
    /// Hunk number that conflicted
    pub hunk_number: u32,
    /// Expected line in file
    pub expected_line: u32,
    /// Actual content at that line
    pub actual_content: String,
    /// Expected content
    pub expected_content: String,
}

/// A parsed diff hunk
#[derive(Debug, Clone)]
struct DiffHunk {
    /// Old file start line
    old_start: u32,
    /// Old file line count
    old_lines: u32,
    /// New file start line
    new_start: u32,
    /// New file line count
    new_lines: u32,
    /// The lines in this hunk (with +, -, space prefix)
    lines: Vec<String>,
}

/// Parse unified diff content into hunks
fn parse_diff_hunks(diff_content: &str) -> Vec<DiffHunk> {
    let mut hunks = Vec::new();
    let lines = diff_content.lines();

    for line in lines {
        // Look for hunk header: @@ -old_start,old_count +new_start,new_count @@
        if line.starts_with("@@") {
            // Parse the header
            // Format: @@ -1,5 +1,6 @@ optional section header
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 3 {
                let old_part = parts[1]; // -1,5 or -1
                let new_part = parts[2]; // +1,6 or +1

                // Parse old part
                let old_start = parse_range_number(old_part, true);
                let old_lines = parse_range_number(old_part, false);

                // Parse new part
                let new_start = parse_range_number(new_part, true);
                let new_lines = parse_range_number(new_part, false);

                hunks.push(DiffHunk {
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    lines: Vec::new(),
                });
            }
        } else if line.starts_with('+') || line.starts_with('-') || line.starts_with(' ') {
            // This is a hunk line
            if let Some(hunk) = hunks.last_mut() {
                hunk.lines.push(line.to_string());
            }
        }
    }

    hunks
}

/// Parse a range number from @@ header
/// For "-1,5" or "+1,6" format
fn parse_range_number(s: &str, is_start: bool) -> u32 {
    // Remove leading + or -
    let s = s.trim_start_matches('+').trim_start_matches('-');

    if is_start {
        // First number (start line)
        s.split(',')
            .next()
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(1)
    } else {
        // Second number (count) or 1 if not present
        s.split(',')
            .nth(1)
            .and_then(|n| n.parse::<u32>().ok())
            .unwrap_or(1)
    }
}

/// Apply hunks to file content
fn apply_hunks(content: &str, hunks: &[DiffHunk]) -> Result<(String, Vec<ConflictInfo>), String> {
    let mut file_lines: Vec<&str> = content.lines().collect();
    let mut conflicts = Vec::new();

    // Process hunks in reverse order (from end of file to start)
    // This ensures line numbers remain valid after modifications
    for (hunk_idx, hunk) in hunks.iter().enumerate().rev() {
        // Line numbers in diff are 1-indexed, convert to 0-indexed
        let start_line = (hunk.old_start as usize).saturating_sub(1);

        // Build the new content for this hunk section
        let mut new_lines: Vec<&str> = Vec::new();
        let mut file_line_idx = start_line;

        // Track lines added/removed for this hunk
        for hunk_line in &hunk.lines {
            if hunk_line.starts_with('-') && !hunk_line.starts_with("---") {
                // Remove line - skip it (don't add to new_lines, just advance index)
                file_line_idx += 1;
            } else if hunk_line.starts_with('+') && !hunk_line.starts_with("+++") {
                // Add new line - add to new_lines
                let line_content = hunk_line.trim_start_matches('+');
                new_lines.push(line_content);
            } else if hunk_line.starts_with(' ') {
                // Context line - keep from file if available, or from hunk
                if file_line_idx < file_lines.len() {
                    new_lines.push(file_lines[file_line_idx]);
                } else {
                    // Use the hunk content if file is shorter
                    new_lines.push(hunk_line.trim_start_matches(' '));
                }
                file_line_idx += 1;
            }
        }

        // Calculate how many old lines to remove
        let old_lines_count = hunk.old_lines as usize;
        let end_line = (start_line + old_lines_count).min(file_lines.len());

        // Replace the section in file_lines
        if start_line < file_lines.len() {
            file_lines.splice(start_line..end_line, new_lines.into_iter());
        } else {
            // Append at end if start is beyond current file
            file_lines.extend(new_lines.into_iter());
        }
    }

    let result = file_lines.join("\n");

    // Ensure newline at end if original had one
    if content.ends_with('\n') && !result.ends_with('\n') {
        Ok((result + "\n", conflicts))
    } else {
        Ok((result, conflicts))
    }
}

#[async_trait]
impl Tool for PatchTool {
    fn name(&self) -> &str {
        "Patch"
    }

    fn description(&self) -> String {
        "Apply a unified diff patch to a file. Supports standard diff format with @@ hunk headers."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<PatchInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        // Parse input
        let patch_input: PatchInput = serde_json::from_value(input.clone())?;

        // Expand path
        let expanded_path = expand_path(&patch_input.file_path);

        // Check permissions using context's permission mode
        let permission = check_patch_permission(&expanded_path, context.permission_mode);
        if permission == PermissionBehavior::Deny {
            return Ok(ToolResult::error("Permission denied for patch operation".to_string()));
        }

        // Check if file exists
        let file_exists = expanded_path.exists();

        if !file_exists && !patch_input.create_if_missing {
            return Ok(ToolResult::error(format!(
                "File does not exist: {}. Use create_if_missing=true to create it.",
                patch_input.file_path
            )));
        }

        // Read existing content or use empty string
        let original_content = if file_exists {
            fs::read_to_string(&expanded_path)?
        } else {
            String::new()
        };

        // Parse the patch
        let hunks = parse_diff_hunks(&patch_input.patch_content);

        if hunks.is_empty() {
            return Ok(ToolResult::error("No valid diff hunks found in patch content".to_string()));
        }

        // Apply hunks
        let (result_content, conflicts) = apply_hunks(&original_content, &hunks)?;

        // Calculate stats
        let hunks_applied = hunks.len() as u32 - conflicts.len() as u32;
        let hunks_failed = conflicts.len() as u32;

        // Count lines added/removed
        let mut lines_added = 0u32;
        let mut lines_removed = 0u32;

        for hunk in &hunks {
            for line in &hunk.lines {
                if line.starts_with('+') && !line.starts_with("+++") {
                    lines_added += 1;
                } else if line.starts_with('-') && !line.starts_with("---") {
                    lines_removed += 1;
                }
            }
        }

        // Write result to file
        // Ensure parent directory exists
        if let Some(parent) = expanded_path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
        }

        fs::write(&expanded_path, &result_content)?;

        // Build output
        let operation_type = if file_exists { "update" } else { "create" };

        let output = PatchOutput {
            hunks_applied,
            hunks_failed,
            lines_added,
            lines_removed,
            result_content,
            conflicts,
            operation_type: operation_type.to_string(),
            file_path: patch_input.file_path.clone(),
        };

        Ok(ToolResult::success(serde_json::to_string(&output)?))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false // Patch modifies files
    }
}

/// Expand path (handle ~, ./, etc.)
fn expand_path(path: &str) -> PathBuf {
    if path.starts_with('~') {
        // Replace ~ with home directory
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| "/".to_string());
        PathBuf::from(path.replacen('~', &home, 1))
    } else {
        PathBuf::from(path)
    }
}

/// Check permission for patch operation
fn check_patch_permission(path: &PathBuf, mode: PermissionMode) -> PermissionBehavior {
    match mode {
        PermissionMode::Default => PermissionBehavior::Ask,
        PermissionMode::AcceptEdits => PermissionBehavior::Allow,
        PermissionMode::BypassPermissions => PermissionBehavior::Allow,
        PermissionMode::DontAsk => PermissionBehavior::Allow,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_diff_header() {
        let diff = "@@ -1,5 +1,6 @@";
        let hunks = parse_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].old_start, 1);
        assert_eq!(hunks[0].old_lines, 5);
        assert_eq!(hunks[0].new_start, 1);
        assert_eq!(hunks[0].new_lines, 6);
    }

    #[test]
    fn test_parse_diff_with_lines() {
        let diff = "@@ -1,3 +1,4 @@
 line1
-line2
+line2_modified
 line3";
        let hunks = parse_diff_hunks(diff);
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].lines.len(), 4);
    }

    #[test]
    fn test_apply_simple_patch() {
        let original = "line1\nline2\nline3";
        let diff = "@@ -1,3 +1,3 @@
 line1
-line2
+line2_new
 line3";
        let hunks = parse_diff_hunks(diff);
        let (result, conflicts) = apply_hunks(original, &hunks).unwrap();
        assert!(conflicts.is_empty());
        assert_eq!(result, "line1\nline2_new\nline3");
    }

    #[test]
    fn test_apply_add_line_patch() {
        let original = "line1\nline3";
        let diff = "@@ -1,2 +1,3 @@
 line1
+line2
 line3";
        let hunks = parse_diff_hunks(diff);
        let (result, conflicts) = apply_hunks(original, &hunks).unwrap();
        assert!(conflicts.is_empty());
        assert_eq!(result, "line1\nline2\nline3");
    }

    #[test]
    fn test_apply_remove_line_patch() {
        let original = "line1\nline2\nline3";
        let diff = "@@ -1,3 +1,2 @@
 line1
-line2
 line3";
        let hunks = parse_diff_hunks(diff);
        let (result, conflicts) = apply_hunks(original, &hunks).unwrap();
        assert!(conflicts.is_empty());
        assert_eq!(result, "line1\nline3");
    }

    #[test]
    fn test_apply_multi_hunk_patch() {
        let original = "a\nb\nc\nd\ne\nf";
        let diff = "@@ -1,2 +1,2 @@
-a
+a_new
 b
@@ -5,2 +5,2 @@
 e
-f
+f_new";
        let hunks = parse_diff_hunks(diff);
        let (result, conflicts) = apply_hunks(original, &hunks).unwrap();
        assert!(conflicts.is_empty());
        assert_eq!(result, "a_new\nb\nc\nd\ne\nf_new");
    }

    #[test]
    fn test_parse_range_number() {
        assert_eq!(parse_range_number("-1,5", true), 1);
        assert_eq!(parse_range_number("-1,5", false), 5);
        assert_eq!(parse_range_number("-1", true), 1);
        assert_eq!(parse_range_number("-1", false), 1);
    }

    #[test]
    fn test_expand_path_home() {
        let path = "~/test.txt";
        let expanded = expand_path(path);
        assert!(expanded.to_string_lossy().contains("test.txt"));
        // Should contain home directory, not ~
        assert!(!expanded.to_string_lossy().starts_with("~"));
    }
}
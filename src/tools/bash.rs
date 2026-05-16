//! BashTool - Execute shell commands

#![allow(dead_code)] // Future use: bash permission fields

use crate::permissions::{BashPermissionChecker, PermissionMode};
use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult, PermissionResult};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::io::{AsyncBufReadExt, BufReader};

/// BashTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct BashInput {
    /// The command to execute
    pub command: String,
    /// Timeout in seconds (default 30)
    #[serde(default = "default_timeout")]
    pub timeout: u32,
    /// Whether to run in background
    #[serde(default)]
    pub background: bool,
}

fn default_timeout() -> u32 {
    30
}

/// BashTool - Execute shell commands with permission checks
pub struct BashTool {
    /// Permission checker
    permission_checker: BashPermissionChecker,
}

impl BashTool {
    /// Create a new BashTool instance
    pub fn new() -> Self {
        Self {
            permission_checker: BashPermissionChecker::new(),
        }
    }

    /// Create with permission mode
    pub fn with_permission_mode(mode: PermissionMode) -> Self {
        Self {
            permission_checker: BashPermissionChecker::with_mode(mode),
        }
    }

    /// Get permission checker reference
    pub fn permission_checker(&self) -> &BashPermissionChecker {
        &self.permission_checker
    }

    /// Get mutable permission checker
    pub fn permission_checker_mut(&mut self) -> &mut BashPermissionChecker {
        &mut self.permission_checker
    }

    /// Sanitize command output
    fn sanitize_output(output: &str, max_lines: usize, max_chars: usize) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let truncated_lines: Vec<String> = if lines.len() > max_lines {
            let mut result: Vec<String> = lines[..max_lines].iter().map(|s| s.to_string()).collect();
            result.push(format!("... (truncated, {} lines omitted)", lines.len() - max_lines));
            result
        } else {
            lines.iter().map(|s| s.to_string()).collect()
        };

        let combined = truncated_lines.join("\n");
        if combined.len() > max_chars {
            format!("{}... (truncated, {} chars omitted)",
                &combined[..max_chars.min(combined.len())],
                combined.len() - max_chars)
        } else {
            combined
        }
    }
}

impl Default for BashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for BashTool {
    fn name(&self) -> &str {
        "Bash"
    }

    fn description(&self) -> String {
        "Execute shell commands. Use for file operations, git, package managers, etc. Commands run in the current working directory.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<BashInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        // Parse input
        let bash_input: BashInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        // Permission check using BashPermissionChecker
        let perm_result = self.permission_checker.check(&bash_input.command);

        // Handle permission result
        if perm_result.is_denied() {
            return Ok(ToolResult::error(format!(
                "Permission denied: {}",
                perm_result.message
            )));
        }

        // For ask permission, we return a special result that TUI can handle
        // The Agent layer will handle the user confirmation flow
        if perm_result.needs_confirmation() {
            return Ok(ToolResult {
                content: format!("PERMISSION_REQUEST: {}", perm_result.message),
                is_error: false, // Not an error, just needs confirmation
            });
        }

        // Timeout handling
        let timeout_duration = Duration::from_secs(bash_input.timeout.min(300) as u64);

        // Execute command
        let mut child = Command::new("sh")
            .arg("-c")
            .arg(&bash_input.command)
            .current_dir(&context.cwd)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("Failed to spawn process: {}", e))?;

        // Read output with timeout
        let stdout = child.stdout.take();
        let stderr = child.stderr.take();

        let mut stdout_content = String::new();
        let mut stderr_content = String::new();

        if let Some(stdout) = stdout {
            let mut reader = BufReader::new(stdout).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                stdout_content.push_str(&line);
                stdout_content.push('\n');
            }
        }

        if let Some(stderr) = stderr {
            let mut reader = BufReader::new(stderr).lines();
            while let Ok(Some(line)) = reader.next_line().await {
                stderr_content.push_str(&line);
                stderr_content.push('\n');
            }
        }

        // Wait for process with timeout
        let status = tokio::time::timeout(timeout_duration, child.wait())
            .await
            .map_err(|_| format!("Command timed out after {} seconds", bash_input.timeout))?
            .map_err(|e| format!("Failed to wait for process: {}", e))?;

        // Combine output
        let mut full_output = String::new();
        if !stdout_content.is_empty() {
            full_output.push_str(&stdout_content);
        }
        if !stderr_content.is_empty() {
            full_output.push_str(&stderr_content);
        }

        // Truncate if too long
        full_output = Self::sanitize_output(&full_output, 100, 10000);

        if status.success() {
            Ok(ToolResult::success(full_output))
        } else {
            Ok(ToolResult::error(format!(
                "Command failed with exit code {}: {}",
                status.code().unwrap_or(-1),
                full_output
            )))
        }
    }

    fn is_read_only(&self, input: &serde_json::Value) -> bool {
        // Parse command and check if it's likely read-only
        if let Ok(bash_input) = serde_json::from_value::<BashInput>(input.clone()) {
            let cmd = bash_input.command.trim();
            // Simple heuristic: commands starting with these are usually read-only
            let read_only_prefixes = ["ls", "cat", "head", "tail", "grep", "find", "git status", "git log", "git diff", "which", "echo", "pwd", "type"];
            for prefix in read_only_prefixes {
                if cmd.starts_with(prefix) {
                    return true;
                }
            }
        }
        false
    }

    fn check_permissions(&self, input: &serde_json::Value) -> PermissionResult {
        if let Ok(bash_input) = serde_json::from_value::<BashInput>(input.clone()) {
            self.permission_checker.check(&bash_input.command)
        } else {
            PermissionResult::allow("Could not parse input")
        }
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(bash_input) = serde_json::from_value::<BashInput>(input.clone()) {
            format!("Running: {}", bash_input.command)
        } else {
            "Running Bash command".to_string()
        }
    }
}
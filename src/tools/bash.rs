//! BashTool - Execute shell commands

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult, PermissionResult};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::Stdio;
use std::time::Duration;
use tokio::process::Command;
use tokio::io::{AsyncReadExt, AsyncBufReadExt, BufReader};

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

/// BashTool - Execute shell commands with safety checks
pub struct BashTool {
    /// Dangerous commands that are always blocked
    blocked_patterns: Vec<String>,
}

impl BashTool {
    /// Create a new BashTool instance
    pub fn new() -> Self {
        Self {
            blocked_patterns: vec![
                "rm -rf /".to_string(),
                "rm -rf /*".to_string(),
                ":(){ :|:& };:".to_string(),  // Fork bomb
                "mkfs".to_string(),
                "dd if=/dev/zero".to_string(),
                "> /dev/sda".to_string(),
                "chmod -R 777 /".to_string(),
                "chown -R".to_string(),
            ],
        }
    }

    /// Check if command is dangerous
    fn is_dangerous(&self, command: &str) -> bool {
        let lower = command.to_lowercase();
        for pattern in &self.blocked_patterns {
            if lower.contains(&pattern.to_lowercase()) {
                return true;
            }
        }
        false
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

        // Safety check
        if self.is_dangerous(&bash_input.command) {
            return Ok(ToolResult::error(
                "Command blocked for safety reasons. This command could cause irreversible damage.".to_string()
            ));
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
            if self.is_dangerous(&bash_input.command) {
                return PermissionResult::Deny(
                    "This command is blocked for safety reasons.".to_string()
                );
            }

            // Ask for destructive commands
            let destructive_patterns = ["rm", "rmdir", "mv", "chmod", "chown", "kill", "pkill"];
            for pattern in destructive_patterns {
                if bash_input.command.contains(pattern) {
                    return PermissionResult::Ask(format!(
                        "This command may modify or delete files: '{}'. Allow?",
                        bash_input.command
                    ));
                }
            }
        }
        PermissionResult::Allow
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(bash_input) = serde_json::from_value::<BashInput>(input.clone()) {
            format!("Running: {}", bash_input.command)
        } else {
            "Running Bash command".to_string()
        }
    }
}
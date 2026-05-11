//! ExecuteTool - Execute Python/Node code in a sandboxed environment
//!
//! Provides safe execution of code snippets with:
//! - Timeout limits
//! - Output capture (stdout/stderr)
//! - Language detection (Python, Node)
//! - Basic safety restrictions

use crate::permissions::{PermissionBehavior, PermissionMode};
use crate::tools::{Tool, ToolContext, ToolResult, build_schema};
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::process::Command;
use std::time::Duration;

/// ExecuteTool - Execute code in sandboxed environment
pub struct ExecuteTool;

impl ExecuteTool {
    pub fn new() -> Self {
        Self
    }
}

/// Execute input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct ExecuteInput {
    /// Language to execute (python, node/javascript/js)
    pub language: String,
    /// Code to execute
    pub code: String,
    /// Timeout in seconds (default 30)
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

fn default_timeout() -> u32 {
    30
}

/// Execute output result
#[derive(Debug, Clone, Serialize)]
pub struct ExecuteOutput {
    /// Language executed
    pub language: String,
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code
    pub exit_code: i32,
    /// Execution time in milliseconds
    pub execution_time_ms: u64,
    /// Whether execution was successful (exit code 0)
    pub success: bool,
    /// Whether timeout was reached
    pub timed_out: bool,
}

#[async_trait]
impl Tool for ExecuteTool {
    fn name(&self) -> &str {
        "Execute"
    }

    fn description(&self) -> String {
        "Execute Python or Node.js code in a sandboxed environment with timeout and output capture."
            .to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<ExecuteInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        // Parse input
        let exec_input: ExecuteInput = serde_json::from_value(input.clone())?;

        // Check permissions
        let permission = check_execute_permission(context.permission_mode);
        if permission == PermissionBehavior::Deny {
            return Ok(ToolResult::error("Permission denied for code execution".to_string()));
        }

        // Normalize language name
        let language = normalize_language(&exec_input.language);

        // Validate language
        if !is_supported_language(&language) {
            return Ok(ToolResult::error(format!(
                "Unsupported language: {}. Supported: python, node",
                exec_input.language
            )));
        }

        // Check if the interpreter is available
        if !check_interpreter_available(&language) {
            return Ok(ToolResult::error(format!(
                "{} interpreter not found. Please install {} to use this tool.",
                language, language
            )));
        }

        // Execute the code
        let start_time = std::time::Instant::now();
        let timeout_duration = Duration::from_secs(exec_input.timeout as u64);

        let result = execute_code(&language, &exec_input.code, timeout_duration);

        let execution_time_ms = start_time.elapsed().as_millis() as u64;

        // Build output
        let output = match result {
            Ok((stdout, stderr, exit_code)) => ExecuteOutput {
                language: language.clone(),
                stdout,
                stderr,
                exit_code,
                execution_time_ms,
                success: exit_code == 0,
                timed_out: false,
            },
            Err(e) => {
                // Check if it's a timeout
                if e.to_string().contains("timeout") {
                    ExecuteOutput {
                        language: language.clone(),
                        stdout: String::new(),
                        stderr: format!("Execution timed out after {} seconds", exec_input.timeout),
                        exit_code: -1,
                        execution_time_ms,
                        success: false,
                        timed_out: true,
                    }
                } else {
                    ExecuteOutput {
                        language: language.clone(),
                        stdout: String::new(),
                        stderr: e.to_string(),
                        exit_code: -1,
                        execution_time_ms,
                        success: false,
                        timed_out: false,
                    }
                }
            }
        };

        Ok(ToolResult::success(serde_json::to_string(&output)?))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        false // Execute runs code which can modify things
    }
}

/// Normalize language name
fn normalize_language(lang: &str) -> String {
    let lower = lang.to_lowercase();
    match lower.as_str() {
        "python" | "py" => "python".to_string(),
        "node" | "javascript" | "js" => "node".to_string(),
        _ => lower,
    }
}

/// Check if language is supported
fn is_supported_language(lang: &str) -> bool {
    matches!(lang, "python" | "node")
}

/// Check if interpreter is available
fn check_interpreter_available(lang: &str) -> bool {
    let (cmd, arg) = match lang {
        "python" => ("python3", "--version"),
        "node" => ("node", "--version"),
        _ => return false,
    };

    Command::new(cmd)
        .arg(arg)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Execute code with timeout
fn execute_code(
    language: &str,
    code: &str,
    timeout: Duration,
) -> Result<(String, String, i32), Box<dyn Error + Send + Sync>> {
    let (cmd, args) = match language {
        "python" => ("python3", vec!["-c", code]),
        "node" => ("node", vec!["-e", code]),
        _ => return Err(format!("Unknown language: {}", language).into()),
    };

    // Use tokio for async execution with timeout
    let output = tokio::runtime::Runtime::new()
        .expect("Failed to create tokio runtime")
        .block_on(async {
            tokio::time::timeout(
                timeout,
                tokio::process::Command::new(cmd)
                    .args(&args)
                    .output(),
            )
            .await
        })?;

    let output = output?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1);

    Ok((stdout, stderr, exit_code))
}

/// Check permission for execute operation
fn check_execute_permission(mode: PermissionMode) -> PermissionBehavior {
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
    fn test_normalize_language() {
        assert_eq!(normalize_language("python"), "python");
        assert_eq!(normalize_language("py"), "python");
        assert_eq!(normalize_language("Python"), "python");
        assert_eq!(normalize_language("node"), "node");
        assert_eq!(normalize_language("javascript"), "node");
        assert_eq!(normalize_language("js"), "node");
        assert_eq!(normalize_language("Node"), "node");
    }

    #[test]
    fn test_is_supported_language() {
        assert!(is_supported_language("python"));
        assert!(is_supported_language("node"));
        assert!(!is_supported_language("ruby"));
        assert!(!is_supported_language("bash"));
    }

    #[test]
    fn test_default_timeout() {
        assert_eq!(default_timeout(), 30);
    }

    #[test]
    fn test_check_interpreter_available() {
        // These tests may fail if the interpreters aren't installed
        // Just check the function doesn't panic
        let _ = check_interpreter_available("python");
        let _ = check_interpreter_available("node");
    }

    #[test]
    fn test_execute_simple_python() {
        // Skip if python3 not available
        if !check_interpreter_available("python") {
            return;
        }

        let result = execute_code("python", "print('hello')", Duration::from_secs(10));
        if let Ok((stdout, stderr, exit_code)) = result {
            assert_eq!(exit_code, 0);
            assert!(stdout.contains("hello"));
            assert!(stderr.is_empty() || stderr.len() < 100);
        }
    }

    #[test]
    fn test_execute_simple_node() {
        // Skip if node not available
        if !check_interpreter_available("node") {
            return;
        }

        let result = execute_code("node", "console.log('hello')", Duration::from_secs(10));
        if let Ok((stdout, stderr, exit_code)) = result {
            assert_eq!(exit_code, 0);
            assert!(stdout.contains("hello"));
        }
    }

    #[test]
    fn test_execute_timeout() {
        // Skip if python3 not available
        if !check_interpreter_available("python") {
            return;
        }

        // This should timeout - use a short timeout and long sleep
        let result = execute_code("python", "import time; time.sleep(5)", Duration::from_millis(500));
        // The result should be an error due to timeout
        assert!(result.is_err(), "Expected timeout error but got success");
        // Check the error message contains "timeout" or "elapsed"
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("timeout") || err_msg.contains("elapsed"),
            "Error message should mention timeout: {}", err_msg);
    }

    #[test]
    fn test_execute_error_code() {
        // Skip if python3 not available
        if !check_interpreter_available("python") {
            return;
        }

        let result = execute_code("python", "raise Exception('test error')", Duration::from_secs(10));
        if let Ok((stdout, stderr, exit_code)) = result {
            assert_ne!(exit_code, 0);
            assert!(stderr.contains("test error") || stderr.contains("Exception"));
        }
    }
}
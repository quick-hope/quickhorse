//! GitTool - Git operations wrapper
//!
//! Provides safe Git operations: status, diff, log, add, commit

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::process::Command;

use crate::permissions::{PermissionResult, PermissionBehavior};
use crate::tools::{Tool, ToolContext, ToolResult, build_schema};

/// Git operation type
#[derive(Debug, Clone, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GitOperation {
    /// Show working tree status
    Status,
    /// Show changes between commits, commit and working tree, etc
    Diff,
    /// Show commit logs
    Log,
    /// Add file contents to the index
    Add,
    /// Record changes to the repository
    Commit,
    /// List, create, or delete branches
    Branch,
    /// Switch branches or restore working tree files
    Checkout,
    /// Show current branch name
    CurrentBranch,
}

/// GitTool input parameters
#[derive(Debug, Clone, Deserialize, JsonSchema)]
pub struct GitInput {
    /// Git operation to perform
    pub operation: GitOperation,
    /// Optional arguments for the operation
    pub args: Option<Vec<String>>,
    /// Commit message (for commit operation)
    pub message: Option<String>,
    /// Branch name (for checkout operation)
    pub branch: Option<String>,
    /// Files to add (for add operation)
    pub files: Option<Vec<String>>,
    /// Number of commits to show (for log operation)
    pub limit: Option<u32>,
}

/// GitTool output result
#[derive(Debug, Clone, Serialize)]
pub struct GitOutput {
    /// Operation that was performed
    pub operation: String,
    /// Git command output
    pub output: String,
    /// Whether the operation succeeded
    pub success: bool,
    /// Error message if failed
    pub error: Option<String>,
}

/// GitTool implementation
pub struct GitTool;

impl GitTool {
    /// Create a new GitTool instance
    pub fn new() -> Self {
        Self
    }

    /// Check if operation is dangerous
    fn is_dangerous_operation(operation: &GitOperation) -> bool {
        match operation {
            // Safe read-only operations
            GitOperation::Status => false,
            GitOperation::Diff => false,
            GitOperation::Log => false,
            GitOperation::Branch => false,
            GitOperation::CurrentBranch => false,

            // Operations that need caution
            GitOperation::Add => false,
            GitOperation::Commit => false,
            GitOperation::Checkout => true, // Can lose uncommitted changes
        }
    }

    /// Build git command from operation
    fn build_command(operation: &GitOperation, input: &GitInput) -> Command {
        let mut cmd = Command::new("git");

        match operation {
            GitOperation::Status => {
                cmd.args(["status", "--porcelain"]);
            }
            GitOperation::Diff => {
                cmd.arg("diff");
                if let Some(args) = &input.args {
                    cmd.args(args);
                }
            }
            GitOperation::Log => {
                cmd.args(["log", "--oneline"]);
                if let Some(limit) = input.limit {
                    cmd.args(["-n", &limit.to_string()]);
                }
                if let Some(args) = &input.args {
                    cmd.args(args);
                }
            }
            GitOperation::Add => {
                cmd.arg("add");
                if let Some(files) = &input.files {
                    cmd.args(files);
                } else {
                    cmd.arg(".");
                }
            }
            GitOperation::Commit => {
                cmd.args(["commit", "-m"]);
                if let Some(message) = &input.message {
                    cmd.arg(message);
                } else {
                    cmd.arg("Update");
                }
            }
            GitOperation::Branch => {
                cmd.arg("branch");
                if let Some(args) = &input.args {
                    cmd.args(args);
                }
            }
            GitOperation::Checkout => {
                cmd.arg("checkout");
                if let Some(branch) = &input.branch {
                    cmd.arg(branch);
                }
            }
            GitOperation::CurrentBranch => {
                cmd.args(["branch", "--show-current"]);
            }
        }

        cmd
    }

    /// Check permission for git operation
    fn check_permission(operation: &GitOperation) -> PermissionResult {
        if Self::is_dangerous_operation(operation) {
            PermissionResult::ask("Git checkout may lose uncommitted changes")
        } else {
            PermissionResult::allow("Git operation allowed")
        }
    }
}

impl Default for GitTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for GitTool {
    fn name(&self) -> &str {
        "Git"
    }

    fn description(&self) -> String {
        "Execute Git operations. Supported operations: status, diff, log, add, commit, branch, checkout, current_branch. Use this tool to check repository status, view changes, commit files, and manage branches.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<GitInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn std::error::Error + Send + Sync>> {
        // Parse input
        let git_input: GitInput = serde_json::from_value(input.clone())
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                format!("Invalid input: {}", e)
            )) as Box<dyn std::error::Error + Send + Sync>)?;

        // Check permission
        let permission = Self::check_permission(&git_input.operation);
        if permission.behavior == PermissionBehavior::Deny {
            return Ok(ToolResult::error(format!(
                "Permission denied: {}",
                permission.message
            )));
        }

        // Build command
        let mut cmd = Self::build_command(&git_input.operation, &git_input);

        // Execute command
        let output = cmd.output()
            .map_err(|e| Box::new(std::io::Error::new(
                std::io::ErrorKind::Other,
                format!("Failed to execute git command: {}", e)
            )) as Box<dyn std::error::Error + Send + Sync>)?;

        // Parse result
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        let success = output.status.success();

        let operation_name = match git_input.operation {
            GitOperation::Status => "status",
            GitOperation::Diff => "diff",
            GitOperation::Log => "log",
            GitOperation::Add => "add",
            GitOperation::Commit => "commit",
            GitOperation::Branch => "branch",
            GitOperation::Checkout => "checkout",
            GitOperation::CurrentBranch => "current_branch",
        };

        // Format message
        let message = if success {
            if stdout.is_empty() {
                format!("Git {}: No changes", operation_name)
            } else {
                format!("Git {}:\n{}", operation_name, stdout.trim())
            }
        } else {
            format!("Git {} failed: {}", operation_name, stderr.trim())
        };

        Ok(ToolResult::success(message))
    }

    fn is_read_only(&self, input: &serde_json::Value) -> bool {
        if let Ok(git_input) = serde_json::from_value::<GitInput>(input.clone()) {
            matches!(
                git_input.operation,
                GitOperation::Status
                | GitOperation::Diff
                | GitOperation::Log
                | GitOperation::Branch
                | GitOperation::CurrentBranch
            )
        } else {
            true // Default to read-only for invalid input
        }
    }

    fn check_permissions(&self, input: &serde_json::Value) -> PermissionResult {
        if let Ok(git_input) = serde_json::from_value::<GitInput>(input.clone()) {
            Self::check_permission(&git_input.operation)
        } else {
            PermissionResult::deny("Invalid input for permission check")
        }
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(git_input) = serde_json::from_value::<GitInput>(input.clone()) {
            let op = match git_input.operation {
                GitOperation::Status => "Checking git status",
                GitOperation::Diff => "Viewing git diff",
                GitOperation::Log => "Viewing git log",
                GitOperation::Add => "Adding files to git",
                GitOperation::Commit => "Committing changes",
                GitOperation::Branch => "Managing git branches",
                GitOperation::Checkout => "Switching git branch",
                GitOperation::CurrentBranch => "Getting current branch",
            };
            op.to_string()
        } else {
            "Running git operation".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_context() -> ToolContext {
        ToolContext::default()
    }

    #[test]
    fn test_is_dangerous_operation() {
        assert!(!GitTool::is_dangerous_operation(&GitOperation::Status));
        assert!(!GitTool::is_dangerous_operation(&GitOperation::Diff));
        assert!(!GitTool::is_dangerous_operation(&GitOperation::Log));
        assert!(!GitTool::is_dangerous_operation(&GitOperation::Add));
        assert!(!GitTool::is_dangerous_operation(&GitOperation::Commit));
        assert!(GitTool::is_dangerous_operation(&GitOperation::Checkout));
    }

    #[test]
    fn test_check_permission() {
        let result = GitTool::check_permission(&GitOperation::Status);
        assert_eq!(result.behavior, PermissionBehavior::Allow);

        let result = GitTool::check_permission(&GitOperation::Checkout);
        assert_eq!(result.behavior, PermissionBehavior::Ask);
    }

    #[test]
    fn test_summarize() {
        let input = serde_json::json!({
            "operation": "status"
        });
        let tool = GitTool::new();
        let summary = tool.summarize(&input);
        assert_eq!(summary, "Checking git status");

        let input = serde_json::json!({
            "operation": "diff"
        });
        let summary = tool.summarize(&input);
        assert_eq!(summary, "Viewing git diff");

        let input = serde_json::json!({
            "operation": "commit",
            "message": "Test commit"
        });
        let summary = tool.summarize(&input);
        assert_eq!(summary, "Committing changes");
    }

    #[tokio::test]
    async fn test_git_status_in_repo() {
        // This test runs in the quickhorse repo which is a git repo
        let input = serde_json::json!({
            "operation": "status"
        });

        let tool = GitTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("Git status"));
    }

    #[tokio::test]
    async fn test_git_current_branch() {
        let input = serde_json::json!({
            "operation": "current_branch"
        });

        let tool = GitTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);
        // Should show current branch (main)
        assert!(result.content.contains("Git current_branch"));
    }

    #[tokio::test]
    async fn test_git_log() {
        let input = serde_json::json!({
            "operation": "log",
            "limit": 5
        });

        let tool = GitTool::new();
        let context = create_context();

        let result = tool.call(input, &context).await.unwrap();
        assert!(!result.is_error);
        assert!(result.content.contains("Git log"));
    }

    #[test]
    fn test_is_read_only() {
        let tool = GitTool::new();

        let input = serde_json::json!({"operation": "status"});
        assert!(tool.is_read_only(&input));

        let input = serde_json::json!({"operation": "diff"});
        assert!(tool.is_read_only(&input));

        let input = serde_json::json!({"operation": "add"});
        assert!(!tool.is_read_only(&input));

        let input = serde_json::json!({"operation": "commit"});
        assert!(!tool.is_read_only(&input));
    }
}
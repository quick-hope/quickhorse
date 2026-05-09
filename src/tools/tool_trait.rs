//! Tool trait definition - core interface for all tools

use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Context passed to tool execution
pub struct ToolContext {
    /// Current working directory
    pub cwd: String,
    /// Abort signal
    pub abort_signal: Option<std::sync::Arc<tokio::sync::Notify>>,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            abort_signal: None,
        }
    }
}

/// Result from tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// Tool output content
    pub content: String,
    /// Whether the result is an error
    pub is_error: bool,
}

impl ToolResult {
    /// Create a successful result
    pub fn success(content: String) -> Self {
        Self { content, is_error: false }
    }

    /// Create an error result
    pub fn error(message: String) -> Self {
        Self { content: message, is_error: true }
    }
}

/// Permission check result
#[derive(Debug, Clone)]
pub enum PermissionResult {
    /// Allow the tool to run
    Allow,
    /// Deny with a reason
    Deny(String),
    /// Ask user for permission
    Ask(String),
}

/// Core trait for all tools
#[async_trait]
pub trait Tool: Send + Sync {
    /// Tool name (used in API calls)
    fn name(&self) -> &str;

    /// Tool description (shown to LLM)
    fn description(&self) -> String;

    /// Input schema in JSON Schema format
    fn input_schema(&self) -> serde_json::Value;

    /// Execute the tool
    async fn call(
        &self,
        input: serde_json::Value,
        context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>>;

    /// Check if this is a read-only operation
    fn is_read_only(&self, input: &serde_json::Value) -> bool;

    /// Check permissions for this tool call
    fn check_permissions(&self, _input: &serde_json::Value) -> PermissionResult {
        // Default: allow (implementations can override for security)
        PermissionResult::Allow
    }

    /// Get a summary of the tool call for display
    fn summarize(&self, _input: &serde_json::Value) -> String {
        format!("Running {}", self.name())
    }
}

/// Helper function to build JSON schema from a type
pub fn build_schema<T: JsonSchema>() -> serde_json::Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(&schema.schema).unwrap_or(serde_json::json!({}))
}
//! Tool trait definition - core interface for all tools

#![allow(dead_code)] // Future use: tool capabilities

use crate::permissions::PermissionResult as PermResult;
use crate::permissions::BashPermissionChecker;
use async_trait::async_trait;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Provider capabilities for multimodal support
#[derive(Debug, Clone, Default)]
pub struct ProviderCapabilities {
    /// Whether provider supports vision/image input
    pub multimodal: bool,
}

/// Context passed to tool execution
#[allow(dead_code)]
pub struct ToolContext {
    /// Current working directory
    pub cwd: String,
    /// Abort signal
    pub abort_signal: Option<std::sync::Arc<tokio::sync::Notify>>,
    /// Permission mode
    pub permission_mode: crate::permissions::PermissionMode,
    /// Permission checker
    pub permissions: BashPermissionChecker,
    /// Provider capabilities
    pub provider_capabilities: ProviderCapabilities,
}

impl Default for ToolContext {
    fn default() -> Self {
        Self {
            cwd: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            abort_signal: None,
            permission_mode: crate::permissions::PermissionMode::Default,
            permissions: BashPermissionChecker::new(),
            provider_capabilities: ProviderCapabilities::default(),
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

/// Permission check result (alias to permissions module)
pub type PermissionResult = PermResult;

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
    #[allow(dead_code)]
    fn is_read_only(&self, input: &serde_json::Value) -> bool;

    /// Check permissions for this tool call
    #[allow(dead_code)]
    fn check_permissions(&self, _input: &serde_json::Value) -> PermissionResult {
        // Default: allow (implementations can override for security)
        PermissionResult::allow("Default permission check")
    }

    /// Get a summary of the tool call for display
    #[allow(dead_code)]
    fn summarize(&self, _input: &serde_json::Value) -> String {
        format!("Running {}", self.name())
    }
}

/// Helper function to build JSON schema from a type
pub fn build_schema<T: JsonSchema>() -> serde_json::Value {
    let schema = schemars::schema_for!(T);
    serde_json::to_value(&schema.schema).unwrap_or(serde_json::json!({}))
}
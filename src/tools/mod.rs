//! Tools module - tool implementations for the coding agent

mod bash;
mod file_edit;
mod file_read;
mod git;
mod glob;
mod grep;
mod tool_trait;
mod web_fetch;
mod write;

pub use bash::BashTool;
pub use file_edit::FileEditTool;
pub use file_read::FileReadTool;
pub use git::GitTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use web_fetch::WebFetchTool;
pub use write::WriteTool;
pub use tool_trait::{Tool, ToolContext, ToolResult, build_schema};

use std::collections::HashMap;
use std::sync::Arc;

/// Registry of all available tools
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create a new empty registry
    pub fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// Create a registry with default tools
    pub fn with_default_tools() -> Self {
        let mut registry = Self::new();
        registry.register(Arc::new(BashTool::new()));
        registry.register(Arc::new(FileReadTool::new()));
        registry.register(Arc::new(FileEditTool::new()));
        registry.register(Arc::new(WriteTool::new()));
        registry.register(Arc::new(GitTool::new()));
        registry.register(Arc::new(GlobTool::new()));
        registry.register(Arc::new(GrepTool::new()));
        registry.register(Arc::new(WebFetchTool::new()));
        registry
    }

    /// Register a tool
    pub fn register(&mut self, tool: Arc<dyn Tool>) {
        self.tools.insert(tool.name().to_string(), tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Get all registered tools
    pub fn all(&self) -> Vec<Arc<dyn Tool>> {
        self.tools.values().cloned().collect()
    }

    /// Get tool schemas for API request
    pub fn schemas_for_api(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|tool| {
                serde_json::json!({
                    "type": "function",
                    "function": {
                        "name": tool.name(),
                        "description": tool.description(),
                        "parameters": tool.input_schema(),
                    }
                })
            })
            .collect()
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::with_default_tools()
    }
}
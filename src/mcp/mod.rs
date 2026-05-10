//! MCP module - Model Context Protocol implementation
//!
//! MCP is a JSON-RPC 2.0 based protocol for connecting LLMs to tools, resources, and prompts.
//! See: https://spec.modelcontextprotocol.io/
//!
//! These types are implemented for MCP support but not yet integrated into the main flow.
#![allow(dead_code)]

mod protocol;
mod server;
mod client;

pub use protocol::{McpMessage, McpRequest, McpResponse, McpNotification, RequestId};

/// MCP protocol version
pub const MCP_VERSION: &str = "2024-11-05";

/// MCP capabilities
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpCapabilities {
    /// Tools capability
    #[serde(default)]
    pub tools: Option<ToolsCapability>,
    /// Resources capability
    #[serde(default)]
    pub resources: Option<ResourcesCapability>,
    /// Prompts capability
    #[serde(default)]
    pub prompts: Option<PromptsCapability>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolsCapability {
    /// Whether tool list can change
    #[serde(default)]
    pub list_changed: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ResourcesCapability {
    /// Whether resource list can change
    #[serde(default)]
    pub list_changed: Option<bool>,
    /// Whether resources can be subscribed to
    #[serde(default)]
    pub subscribe: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptsCapability {
    /// Whether prompt list can change
    #[serde(default)]
    pub list_changed: Option<bool>,
}

/// MCP tool definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpTool {
    /// Tool name
    pub name: String,
    /// Tool description
    #[serde(default)]
    pub description: Option<String>,
    /// Input schema (JSON Schema)
    pub input_schema: serde_json::Value,
}

/// MCP resource definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpResource {
    /// Resource URI
    pub uri: String,
    /// Resource name
    pub name: String,
    /// Resource description
    #[serde(default)]
    pub description: Option<String>,
    /// MIME type
    #[serde(default)]
    pub mime_type: Option<String>,
}

/// MCP prompt definition
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpPrompt {
    /// Prompt name
    pub name: String,
    /// Prompt description
    #[serde(default)]
    pub description: Option<String>,
    /// Prompt arguments
    #[serde(default)]
    pub arguments: Option<Vec<McpPromptArgument>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpPromptArgument {
    /// Argument name
    pub name: String,
    /// Argument description
    #[serde(default)]
    pub description: Option<String>,
    /// Whether argument is required
    #[serde(default)]
    pub required: Option<bool>,
}

/// MCP implementation info
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct McpImplementationInfo {
    /// Implementation name
    pub name: String,
    /// Implementation version
    pub version: String,
}
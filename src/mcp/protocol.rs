//! MCP Protocol - JSON-RPC 2.0 message types
//!
//! These types are implemented for MCP support but not yet integrated into the main flow.
#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request ID
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
}

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// JSON-RPC version (always "2.0")
    jsonrpc: String,
    /// Method name
    pub method: String,
    /// Request ID
    pub id: RequestId,
    /// Method parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 response (success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// JSON-RPC version (always "2.0")
    jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Result
    pub result: serde_json::Value,
}

/// JSON-RPC 2.0 error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// JSON-RPC version (always "2.0")
    jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Error object
    pub error: JsonRpcErrorObject,
}

/// JSON-RPC error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcErrorObject {
    /// Error code
    pub code: i64,
    /// Error message
    pub message: String,
    /// Error data
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 notification (no ID)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcNotification {
    /// JSON-RPC version (always "2.0")
    jsonrpc: String,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

// MCP-specific message types

/// MCP request (wraps JSON-RPC request)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpRequest {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl McpRequest {
    /// Create a new MCP request
    pub fn new(id: RequestId, method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            method,
            params,
        }
    }

    /// Create initialize request
    pub fn initialize(id: RequestId, client_info: crate::mcp::McpImplementationInfo) -> Self {
        Self::new(
            id,
            "initialize".to_string(),
            Some(serde_json::json!({
                "protocolVersion": crate::mcp::MCP_VERSION,
                "capabilities": {},
                "clientInfo": client_info,
            })),
        )
    }

    /// Create tools/list request
    pub fn tools_list(id: RequestId) -> Self {
        Self::new(id, "tools/list".to_string(), None)
    }

    /// Create tools/call request
    pub fn tools_call(id: RequestId, name: String, arguments: serde_json::Value) -> Self {
        Self::new(
            id,
            "tools/call".to_string(),
            Some(serde_json::json!({
                "name": name,
                "arguments": arguments,
            })),
        )
    }

    /// Create resources/list request
    pub fn resources_list(id: RequestId) -> Self {
        Self::new(id, "resources/list".to_string(), None)
    }

    /// Create resources/read request
    pub fn resources_read(id: RequestId, uri: String) -> Self {
        Self::new(
            id,
            "resources/read".to_string(),
            Some(serde_json::json!({
                "uri": uri,
            })),
        )
    }

    /// Create prompts/list request
    pub fn prompts_list(id: RequestId) -> Self {
        Self::new(id, "prompts/list".to_string(), None)
    }

    /// Create prompts/get request
    pub fn prompts_get(id: RequestId, name: String, arguments: Option<serde_json::Value>) -> Self {
        Self::new(
            id,
            "prompts/get".to_string(),
            Some(serde_json::json!({
                "name": name,
                "arguments": arguments,
            })),
        )
    }
}

/// MCP response (success)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpResponse {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Result
    pub result: serde_json::Value,
}

impl McpResponse {
    /// Create a new MCP response
    pub fn new(id: RequestId, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result,
        }
    }

    /// Create initialize response
    pub fn initialize(
        id: RequestId,
        capabilities: crate::mcp::McpCapabilities,
        server_info: crate::mcp::McpImplementationInfo,
    ) -> Self {
        Self::new(
            id,
            serde_json::json!({
                "protocolVersion": crate::mcp::MCP_VERSION,
                "capabilities": capabilities,
                "serverInfo": server_info,
            }),
        )
    }

    /// Create tools/list response
    pub fn tools_list(id: RequestId, tools: Vec<crate::mcp::McpTool>) -> Self {
        Self::new(id, serde_json::json!({ "tools": tools }))
    }

    /// Create tools/call response
    pub fn tools_call(id: RequestId, content: Vec<ToolContent>) -> Self {
        Self::new(id, serde_json::json!({ "content": content }))
    }

    /// Create resources/list response
    pub fn resources_list(id: RequestId, resources: Vec<crate::mcp::McpResource>) -> Self {
        Self::new(id, serde_json::json!({ "resources": resources }))
    }

    /// Create resources/read response
    pub fn resources_read(id: RequestId, contents: Vec<ResourceContent>) -> Self {
        Self::new(id, serde_json::json!({ "contents": contents }))
    }

    /// Create prompts/list response
    pub fn prompts_list(id: RequestId, prompts: Vec<crate::mcp::McpPrompt>) -> Self {
        Self::new(id, serde_json::json!({ "prompts": prompts }))
    }

    /// Create prompts/get response
    pub fn prompts_get(id: RequestId, messages: Vec<PromptMessage>) -> Self {
        Self::new(id, serde_json::json!({ "messages": messages }))
    }

    /// Create success response
    pub fn success(id: RequestId, result: serde_json::Value) -> Self {
        Self::new(id, result)
    }
}

/// MCP error response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpError {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Request ID
    pub id: RequestId,
    /// Error object
    pub error: McpErrorObject,
}

impl McpError {
    /// Create a new MCP error
    pub fn new(id: RequestId, code: i64, message: String, data: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            error: McpErrorObject { code, message, data },
        }
    }

    /// Parse error
    pub fn parse_error(id: RequestId) -> Self {
        Self::new(id, -32700, "Parse error".to_string(), None)
    }

    /// Invalid request
    pub fn invalid_request(id: RequestId) -> Self {
        Self::new(id, -32600, "Invalid Request".to_string(), None)
    }

    /// Method not found
    pub fn method_not_found(id: RequestId, method: &str) -> Self {
        Self::new(id.clone(), -32601, format!("Method not found: {}", method), None)
    }

    /// Invalid params
    pub fn invalid_params(message: String) -> Self {
        Self::new(RequestId::Number(0), -32602, message, None)
    }

    /// Internal error
    pub fn internal_error(message: String) -> Self {
        Self::new(RequestId::Number(0), -32603, message, None)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpErrorObject {
    pub code: i64,
    pub message: String,
    #[serde(default)]
    pub data: Option<serde_json::Value>,
}

/// MCP notification
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpNotification {
    /// JSON-RPC version
    pub jsonrpc: String,
    /// Method name
    pub method: String,
    /// Method parameters
    #[serde(default)]
    pub params: Option<serde_json::Value>,
}

impl McpNotification {
    /// Create a new notification
    pub fn new(method: String, params: Option<serde_json::Value>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }

    /// Create initialized notification
    pub fn initialized() -> Self {
        Self::new("notifications/initialized".to_string(), None)
    }

    /// Create tools/list_changed notification
    pub fn tools_list_changed() -> Self {
        Self::new("notifications/tools/list_changed".to_string(), None)
    }

    /// Create resources/list_changed notification
    pub fn resources_list_changed() -> Self {
        Self::new("notifications/resources/list_changed".to_string(), None)
    }

    /// Create prompts/list_changed notification
    pub fn prompts_list_changed() -> Self {
        Self::new("notifications/prompts/list_changed".to_string(), None)
    }
}

/// MCP message (either request, response, error, or notification)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum McpMessage {
    Request(McpRequest),
    Response(McpResponse),
    Error(McpError),
    Notification(McpNotification),
}

// Content types

/// Tool content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ToolContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: EmbeddedResource },
}

/// Embedded resource
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddedResource {
    pub uri: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// Resource content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceContent {
    pub uri: String,
    #[serde(default)]
    pub mime_type: Option<String>,
    pub text: Option<String>,
    pub blob: Option<String>,
}

/// Prompt message
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptMessage {
    pub role: String,
    pub content: PromptContent,
}

/// Prompt content
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PromptContent {
    Text { text: String },
    Image { data: String, mime_type: String },
    Resource { resource: EmbeddedResource },
}
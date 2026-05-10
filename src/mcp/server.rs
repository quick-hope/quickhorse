//! MCP Server - Server-side MCP implementation

use crate::mcp::{
    McpCapabilities, McpImplementationInfo, McpTool, McpResource, McpPrompt,
    McpMessage, McpRequest, McpResponse,
    MCP_VERSION,
};
use crate::tools::{ToolRegistry, ToolContext};
use std::sync::Arc;

/// MCP Server implementation
pub struct McpServer {
    /// Server name
    name: String,
    /// Server version
    version: String,
    /// Tool registry
    tools: Arc<ToolRegistry>,
    /// Resources (placeholder)
    resources: Vec<McpResource>,
    /// Prompts (placeholder)
    prompts: Vec<McpPrompt>,
}

impl McpServer {
    /// Create a new MCP server
    pub fn new(name: String, version: String, tools: Arc<ToolRegistry>) -> Self {
        Self {
            name,
            version,
            tools,
            resources: Vec::new(),
            prompts: Vec::new(),
        }
    }

    /// Get server capabilities
    pub fn capabilities() -> McpCapabilities {
        McpCapabilities {
            tools: Some(crate::mcp::ToolsCapability {
                list_changed: Some(true),
            }),
            resources: Some(crate::mcp::ResourcesCapability {
                list_changed: Some(false),
                subscribe: Some(false),
            }),
            prompts: Some(crate::mcp::PromptsCapability {
                list_changed: Some(false),
            }),
        }
    }

    /// Handle an MCP message
    pub async fn handle_message(&mut self, message: McpMessage) -> Option<McpMessage> {
        match message {
            McpMessage::Request(request) => {
                Some(McpMessage::Response(self.handle_request(request).await))
            }
            McpMessage::Notification(_notification) => {
                None
            }
            McpMessage::Response(_) | McpMessage::Error(_) => {
                None
            }
        }
    }

    /// Handle an MCP request
    async fn handle_request(&mut self, request: McpRequest) -> McpResponse {
        match request.method.as_str() {
            "initialize" => self.handle_initialize(&request),
            "tools/list" => self.handle_tools_list(&request),
            "tools/call" => self.handle_tools_call(&request).await,
            "resources/list" => self.handle_resources_list(&request),
            "prompts/list" => self.handle_prompts_list(&request),
            _ => McpResponse::success(request.id.clone(), serde_json::json!({
                "error": "Method not found"
            })),
        }
    }

    /// Handle initialize request
    fn handle_initialize(&self, request: &McpRequest) -> McpResponse {
        McpResponse::success(
            request.id.clone(),
            serde_json::json!({
                "protocolVersion": MCP_VERSION,
                "capabilities": Self::capabilities(),
                "serverInfo": McpImplementationInfo {
                    name: self.name.clone(),
                    version: self.version.clone(),
                }
            }),
        )
    }

    /// Handle tools/list request
    fn handle_tools_list(&self, request: &McpRequest) -> McpResponse {
        let tools: Vec<McpTool> = self.tools.all()
            .iter()
            .map(|t| McpTool {
                name: t.name().to_string(),
                description: Some(t.description()),
                input_schema: t.input_schema(),
            })
            .collect();

        McpResponse::success(request.id.clone(), serde_json::json!({
            "tools": tools
        }))
    }

    /// Handle tools/call request
    async fn handle_tools_call(&self, request: &McpRequest) -> McpResponse {
        let params = request.params.clone().unwrap_or(serde_json::json!({}));
        
        let tool_name = params.get("name")
            .and_then(|n| n.as_str())
            .unwrap_or("");

        let tool_args = params.get("arguments")
            .cloned()
            .unwrap_or(serde_json::json!({}));

        // Find the tool
        let tool = self.tools.get(tool_name);
        if tool.is_none() {
            return McpResponse::success(
                request.id.clone(),
                serde_json::json!({
                    "content": [{"type": "text", "text": format!("Tool '{}' not found", tool_name)}],
                    "isError": true
                }),
            );
        }

        let tool = tool.unwrap();
        let context = ToolContext::default();

        // Execute the tool
        let result = tool.call(tool_args, &context).await;

        match result {
            Ok(tool_result) => {
                McpResponse::success(
                    request.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": tool_result.content}],
                        "isError": tool_result.is_error
                    }),
                )
            }
            Err(e) => {
                McpResponse::success(
                    request.id.clone(),
                    serde_json::json!({
                        "content": [{"type": "text", "text": e.to_string()}],
                        "isError": true
                    }),
                )
            }
        }
    }

    /// Handle resources/list request
    fn handle_resources_list(&self, request: &McpRequest) -> McpResponse {
        McpResponse::success(request.id.clone(), serde_json::json!({
            "resources": self.resources
        }))
    }

    /// Handle prompts/list request
    fn handle_prompts_list(&self, request: &McpRequest) -> McpResponse {
        McpResponse::success(request.id.clone(), serde_json::json!({
            "prompts": self.prompts
        }))
    }
}
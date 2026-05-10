//! MCP Client - Client-side MCP implementation

use crate::mcp::{
    McpCapabilities, McpImplementationInfo, McpTool, McpResource,
    McpMessage, McpRequest, McpResponse,
    McpNotification, RequestId,
};

/// MCP Client implementation
pub struct McpClient {
    /// Client name
    name: String,
    /// Client version
    version: String,
    /// Server capabilities (after initialization)
    server_capabilities: Option<McpCapabilities>,
    /// Available tools from server
    tools: Vec<McpTool>,
    /// Available resources from server
    resources: Vec<McpResource>,
    /// Request ID counter
    request_id: i64,
}

impl McpClient {
    /// Create a new MCP client
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            server_capabilities: None,
            tools: Vec::new(),
            resources: Vec::new(),
            request_id: 0,
        }
    }

    /// Get next request ID
    fn next_id(&mut self) -> RequestId {
        self.request_id += 1;
        RequestId::Number(self.request_id)
    }

    /// Get client capabilities
    pub fn capabilities() -> McpCapabilities {
        McpCapabilities {
            tools: Some(crate::mcp::ToolsCapability {
                list_changed: Some(false),
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

    /// Create initialize request
    pub fn initialize(&mut self) -> McpMessage {
        let id = self.next_id();
        let client_info = McpImplementationInfo {
            name: self.name.clone(),
            version: self.version.clone(),
        };
        McpMessage::Request(McpRequest::initialize(id, client_info))
    }

    /// Handle initialize response
    pub fn handle_initialize_response(&mut self, response: &McpResponse) {
        // Parse capabilities from result
        if let Some(capabilities) = response.result.get("capabilities") {
            self.server_capabilities = serde_json::from_value(capabilities.clone()).ok();
        }
    }

    /// Create initialized notification
    pub fn initialized() -> McpMessage {
        McpMessage::Notification(McpNotification::initialized())
    }

    /// Create tools/list request
    pub fn list_tools(&mut self) -> McpMessage {
        let id = self.next_id();
        McpMessage::Request(McpRequest::tools_list(id))
    }

    /// Handle tools/list response
    pub fn handle_tools_list_response(&mut self, response: &McpResponse) {
        if let Some(tools) = response.result.get("tools") {
            self.tools = serde_json::from_value(tools.clone()).unwrap_or_default();
        }
    }

    /// Create tools/call request
    pub fn call_tool(&mut self, name: String, arguments: serde_json::Value) -> McpMessage {
        let id = self.next_id();
        McpMessage::Request(McpRequest::tools_call(id, name, arguments))
    }

    /// Create resources/list request
    pub fn list_resources(&mut self) -> McpMessage {
        let id = self.next_id();
        McpMessage::Request(McpRequest::resources_list(id))
    }

    /// Handle resources/list response
    pub fn handle_resources_list_response(&mut self, response: &McpResponse) {
        if let Some(resources) = response.result.get("resources") {
            self.resources = serde_json::from_value(resources.clone()).unwrap_or_default();
        }
    }

    /// Get available tools
    pub fn tools(&self) -> &[McpTool] {
        &self.tools
    }

    /// Get available resources
    pub fn resources(&self) -> &[McpResource] {
        &self.resources
    }

    /// Check if server supports tools
    pub fn has_tools_support(&self) -> bool {
        self.server_capabilities.as_ref()
            .and_then(|c| c.tools.as_ref())
            .is_some()
    }

    /// Get server capabilities
    pub fn capabilities_ref(&self) -> Option<&McpCapabilities> {
        self.server_capabilities.as_ref()
    }
}
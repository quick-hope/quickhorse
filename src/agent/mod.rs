//! Agent module - core agent logic with tool call loop

mod context;

pub use context::{
    estimate_tokens, estimate_message_tokens, estimate_total_tokens,
    compress_messages, compress_tool_results, needs_compression,
    compression_stats, CompressionStats, DEFAULT_MAX_TOKENS,
};

use crate::permissions::{PermissionMode, PermissionResult};
use crate::provider::{ContentBlock, Message, Provider};
use crate::tools::{Tool, ToolContext, ToolRegistry, ToolResult};
use futures::future::join_all;
use std::sync::{Arc, RwLock};
use std::error::Error;

/// Agent configuration
pub struct AgentConfig {
    /// Maximum tool call iterations
    pub max_iterations: usize,
    /// System prompt
    pub system_prompt: String,
    /// Permission mode
    pub permission_mode: PermissionMode,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            system_prompt: "You are QuickHorse, a CLI coding agent. You have access to tools for executing commands, reading files, and more. Use tools when appropriate to help the user with their requests.".to_string(),
            permission_mode: PermissionMode::Default,
        }
    }
}

/// Permission check result from tool execution
#[derive(Debug, Clone)]
pub enum ToolPermissionStatus {
    /// Tool executed successfully
    Executed(ToolResult),
    /// Permission denied
    Denied(String),
    /// Permission needs user confirmation
    NeedsConfirmation {
        /// Tool name
        tool_name: String,
        /// Tool ID
        tool_id: String,
        /// Input that needs confirmation
        input: serde_json::Value,
        /// Permission request message
        message: String,
    },
}

/// Agent that manages conversation with tools
pub struct Agent {
    /// Provider for LLM calls (wrapped in RwLock for dynamic switching)
    provider: Arc<RwLock<dyn Provider>>,
    /// Tool registry
    tools: ToolRegistry,
    /// Configuration
    config: AgentConfig,
    /// Conversation history
    messages: Vec<Message>,
    /// Pending permission request
    pending_permission: Option<PendingPermission>,
}

/// Pending permission request waiting for user confirmation
#[derive(Debug, Clone)]
pub struct PendingPermission {
    /// Tool name
    pub tool_name: String,
    /// Tool ID
    pub tool_id: String,
    /// Input that needs confirmation
    pub input: serde_json::Value,
    /// Permission request message
    pub message: String,
}

impl Agent {
    /// Create a new agent
    pub fn new(provider: Arc<RwLock<dyn Provider>>, config: AgentConfig) -> Self {
        Self {
            provider,
            tools: ToolRegistry::with_default_tools(),
            config,
            messages: Vec::new(),
            pending_permission: None,
        }
    }

    /// Set permission mode
    pub fn set_permission_mode(&mut self, mode: PermissionMode) {
        self.config.permission_mode = mode;
    }

    /// Get current permission mode
    pub fn permission_mode(&self) -> PermissionMode {
        self.config.permission_mode
    }

    /// Check if there's a pending permission request
    pub fn has_pending_permission(&self) -> bool {
        self.pending_permission.is_some()
    }

    /// Get pending permission request
    pub fn pending_permission(&self) -> Option<&PendingPermission> {
        self.pending_permission.as_ref()
    }

    /// Approve pending permission and execute tool
    pub async fn approve_permission(&mut self) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        if let Some(pending) = self.pending_permission.take() {
            let tool = self.tools.get(&pending.tool_name);
            if let Some(tool) = tool {
                let context = ToolContext {
                    cwd: std::env::current_dir()
                        .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                        .to_string_lossy()
                        .to_string(),
                    abort_signal: None,
                    permission_mode: PermissionMode::BypassPermissions,
                    permissions: crate::permissions::BashPermissionChecker::new(),
                    provider_capabilities: crate::tools::ProviderCapabilities::default(),
                };
                tool.call(pending.input, &context).await
            } else {
                Err(format!("Tool '{}' not found", pending.tool_name).into())
            }
        } else {
            Err("No pending permission request".into())
        }
    }

    /// Deny pending permission
    pub fn deny_permission(&mut self) -> String {
        if let Some(pending) = self.pending_permission.take() {
            format!("Permission denied for: {}", pending.message)
        } else {
            "No pending permission request".to_string()
        }
    }

    /// Add a tool to the registry
    #[allow(dead_code)]
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.register(tool);
    }

    /// Get current messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Get provider reference
    pub fn provider(&self) -> Arc<RwLock<dyn Provider>> {
        self.provider.clone()
    }

    /// Add a message to history
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Clear message history
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Process the last user message (assumes message already in history)
    pub async fn process_last_user_message(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Run tool call loop - don't add user message again
        self.run_loop().await
    }

    /// Process a new user message (adds message to history)
    #[allow(dead_code)]
    pub async fn process(&mut self, user_input: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Add user message
        self.messages.push(Message::user(user_input));

        // Run tool call loop
        self.run_loop().await
    }

    /// Run the tool call loop
    async fn run_loop(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut iteration = 0;

        // Get tool schemas for API
        let tool_schemas = self.tools.schemas_for_api();

        loop {
            iteration += 1;
            if iteration > self.config.max_iterations {
                return Err(format!("Max iterations ({}) reached", self.config.max_iterations).into());
            }

            // Call provider with tools (acquire read lock)
            let response = self.provider
                .read()
                .unwrap()
                .send_message_with_tools(&self.messages, &tool_schemas)
                .await?;

            // Add assistant response to history
            self.messages.push(response.clone());

            // Check if there are tool calls
            let tool_uses = response.tool_uses();

            if tool_uses.is_empty() {
                // No tools - return text content
                return Ok(response.text_content());
            }

            // Execute tools in parallel (independent tools can run concurrently)
            let tool_results = self.execute_tools_parallel(tool_uses).await;

            // Add tool results as user message
            self.messages.push(Message::user_with_tool_results(tool_results));
        }
    }

    /// Execute multiple tools in parallel
    async fn execute_tools_parallel(&self, tool_uses: Vec<&ContentBlock>) -> Vec<ContentBlock> {
        // Collect tool execution futures
        let futures: Vec<_> = tool_uses
            .into_iter()
            .filter_map(|block| {
                if let ContentBlock::ToolUse { id, name, input } = block {
                    // Clone necessary data for async execution
                    let tool_id = id.clone();
                    let tool_name = name.clone();
                    let tool_input = input.clone();
                    let tools = self.tools.clone();
                    let permission_mode = self.config.permission_mode;

                    Some(async move {
                        execute_tool_async(tool_id, tool_name, tool_input, tools, permission_mode).await
                    })
                } else {
                    None
                }
            })
            .collect();

        // Execute all tools concurrently
        join_all(futures).await
    }

    /// Execute a single tool with permission checking (for sequential execution)
    async fn execute_tool(&mut self, tool_id: String, name: String, input: serde_json::Value) -> ContentBlock {
        // Get tool from registry
        let tool = self.tools.get(&name);

        if tool.is_none() {
            return ContentBlock::tool_result(
                tool_id,
                format!("Tool '{}' not found", name),
                true,
            );
        }

        let tool = tool.unwrap();

        // Check permissions first
        let perm_result = tool.check_permissions(&input);

        // Handle permission result based on mode
        if perm_result.is_denied() {
            return ContentBlock::tool_result(
                tool_id,
                format!("Permission denied: {}", perm_result.message),
                true,
            );
        }

        // If needs confirmation and mode is Default, store pending permission
        if perm_result.needs_confirmation() && self.config.permission_mode == PermissionMode::Default {
            // Store pending permission and return special result
            self.pending_permission = Some(PendingPermission {
                tool_name: name.clone(),
                tool_id: tool_id.clone(),
                input: input.clone(),
                message: perm_result.message.clone(),
            });

            // Return a special marker that TUI can detect
            return ContentBlock::tool_result(
                tool_id,
                format!("PERMISSION_ASK: {}", perm_result.message),
                false, // Not an error, just needs confirmation
            );
        }

        // Create context with permission mode
        let context = ToolContext {
            cwd: std::env::current_dir()
                .unwrap_or_else(|_| std::path::PathBuf::from("/"))
                .to_string_lossy()
                .to_string(),
            abort_signal: None,
            permission_mode: self.config.permission_mode,
            permissions: crate::permissions::BashPermissionChecker::new(),
            provider_capabilities: crate::tools::ProviderCapabilities::default(),
        };

        // Execute tool
        let result = tool.call(input.clone(), &context).await;

        match result {
            Ok(ToolResult { content, is_error }) => {
                // Check if result contains permission request marker
                if content.starts_with("PERMISSION_REQUEST:") {
                    // Extract message and store pending permission
                    let message = content.replace("PERMISSION_REQUEST: ", "");
                    self.pending_permission = Some(PendingPermission {
                        tool_name: name.clone(),
                        tool_id: tool_id.clone(),
                        input,
                        message: message.clone(),
                    });
                    return ContentBlock::tool_result(
                        tool_id,
                        format!("PERMISSION_ASK: {}", message),
                        false,
                    );
                }
                ContentBlock::tool_result(tool_id, content, is_error)
            }
            Err(e) => {
                ContentBlock::tool_result(tool_id, format!("Error: {}", e), true)
            }
        }
    }
}

/// Execute a tool asynchronously (for parallel execution)
async fn execute_tool_async(
    tool_id: String,
    name: String,
    input: serde_json::Value,
    tools: ToolRegistry,
    permission_mode: PermissionMode,
) -> ContentBlock {
    // Get tool from registry
    let tool = tools.get(&name);

    if tool.is_none() {
        return ContentBlock::tool_result(
            tool_id,
            format!("Tool '{}' not found", name),
            true,
        );
    }

    let tool = tool.unwrap();

    // Check permissions first
    let perm_result = tool.check_permissions(&input);

    // Handle permission result
    if perm_result.is_denied() {
        return ContentBlock::tool_result(
            tool_id,
            format!("Permission denied: {}", perm_result.message),
            true,
        );
    }

    // For parallel execution, we skip confirmation requests
    // (parallel execution assumes tools are pre-approved or in bypass mode)
    if perm_result.needs_confirmation() && permission_mode == PermissionMode::Default {
        return ContentBlock::tool_result(
            tool_id,
            format!("Permission required (parallel mode): {}", perm_result.message),
            true,
        );
    }

    // Create context
    let context = ToolContext {
        cwd: std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"))
            .to_string_lossy()
            .to_string(),
        abort_signal: None,
        permission_mode: permission_mode,
        permissions: crate::permissions::BashPermissionChecker::new(),
        provider_capabilities: crate::tools::ProviderCapabilities::default(),
    };

    // Execute tool
    let result = tool.call(input, &context).await;

    match result {
        Ok(ToolResult { content, is_error }) => {
            ContentBlock::tool_result(tool_id, content, is_error)
        }
        Err(e) => {
            ContentBlock::tool_result(tool_id, format!("Error: {}", e), true)
        }
    }
}
//! Agent module - core agent logic with tool call loop

use crate::provider::{ContentBlock, Message, Provider};
use crate::tools::{Tool, ToolContext, ToolRegistry, ToolResult};
use std::sync::{Arc, RwLock};
use std::error::Error;

/// Agent configuration
pub struct AgentConfig {
    /// Maximum tool call iterations
    pub max_iterations: usize,
    /// System prompt
    pub system_prompt: String,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            max_iterations: 10,
            system_prompt: "You are QuickHorse, a CLI coding agent. You have access to tools for executing commands, reading files, and more. Use tools when appropriate to help the user with their requests.".to_string(),
        }
    }
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
}

impl Agent {
    /// Create a new agent
    pub fn new(provider: Arc<RwLock<dyn Provider>>, config: AgentConfig) -> Self {
        Self {
            provider,
            tools: ToolRegistry::with_default_tools(),
            config,
            messages: Vec::new(),
        }
    }

    /// Add a tool to the registry
    pub fn add_tool(&mut self, tool: Arc<dyn Tool>) {
        self.tools.register(tool);
    }

    /// Get current messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Add a message to history
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
    }

    /// Clear message history
    pub fn clear(&mut self) {
        self.messages.clear();
    }

    /// Process a user message and return the final response
    pub async fn process(&mut self, user_input: String) -> Result<String, Box<dyn Error + Send + Sync>> {
        // Add user message
        self.messages.push(Message::user(user_input));

        // Run tool call loop
        self.run_loop().await
    }

    /// Run the tool call loop
    async fn run_loop(&mut self) -> Result<String, Box<dyn Error + Send + Sync>> {
        let mut iteration = 0;
        let mut final_text = String::new();

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
                final_text = response.text_content();
                break;
            }

            // Execute tools and collect results
            let tool_results: Vec<ContentBlock> = tool_uses
                .iter()
                .map(|block| {
                    if let ContentBlock::ToolUse { id, name, input } = block {
                        self.execute_tool(id.clone(), name.clone(), input.clone())
                    } else {
                        ContentBlock::tool_result(
                            "unknown".to_string(),
                            "Invalid tool call".to_string(),
                            true,
                        )
                    }
                })
                .collect();

            // Add tool results as user message
            self.messages.push(Message::user_with_tool_results(tool_results));
        }

        Ok(final_text)
    }

    /// Execute a single tool
    fn execute_tool(&self, tool_id: String, name: String, input: serde_json::Value) -> ContentBlock {
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
        let context = ToolContext::default();

        // Execute tool (blocking for now)
        let result = tokio::runtime::Handle::current().block_on(async {
            tool.call(input.clone(), &context).await
        });

        match result {
            Ok(ToolResult { content, is_error }) => {
                ContentBlock::tool_result(tool_id, content, is_error)
            }
            Err(e) => {
                ContentBlock::tool_result(tool_id, format!("Error: {}", e), true)
            }
        }
    }
}
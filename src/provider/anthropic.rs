//! Anthropic (Claude) provider implementation

use crate::provider::{ContentBlock, Message, Provider};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Default Anthropic API URL
const DEFAULT_ANTHROPIC_API_URL: &str = "https://api.anthropic.com/v1/messages";

/// Anthropic provider
pub struct AnthropicProvider {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Model to use
    model: String,
    /// Base URL for API
    base_url: String,
}

impl AnthropicProvider {
    /// Create a new Anthropic provider with default API URL
    pub fn new(api_key: String, model: String) -> Self {
        Self::new_with_base_url(api_key, model, DEFAULT_ANTHROPIC_API_URL.to_string())
    }

    /// Create a new Anthropic provider with custom base URL
    pub fn new_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
        }
    }

    /// Create with default model (Claude 3.5 Sonnet)
    pub fn with_default_model(api_key: String) -> Self {
        Self::new(api_key, "claude-3-5-sonnet-20241022".to_string())
    }

    /// Convert messages to Anthropic format
    fn messages_to_anthropic(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_prompt: Option<String> = None;
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                // Anthropic uses separate system parameter
                system_prompt = Some(msg.text_content());
            } else {
                let content: Vec<AnthropicContentBlock> = msg
                    .content
                    .iter()
                    .map(|block| {
                        match block {
                            ContentBlock::Text { text } => {
                                AnthropicContentBlock::Text { text: text.clone() }
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                AnthropicContentBlock::ToolUse {
                                    id: id.clone(),
                                    name: name.clone(),
                                    input: input.clone(),
                                }
                            }
                            ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                                AnthropicContentBlock::ToolResult {
                                    tool_use_id: tool_use_id.clone(),
                                    content: content.clone(),
                                    is_error: is_error.clone(),
                                }
                            }
                        }
                    })
                    .collect();

                anthropic_messages.push(AnthropicMessage {
                    role: msg.role.clone(),
                    content,
                });
            }
        }

        (system_prompt, anthropic_messages)
    }
}

/// Anthropic message format
#[derive(Serialize)]
struct AnthropicMessage {
    role: String,
    content: Vec<AnthropicContentBlock>,
}

/// Anthropic content block
#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicContentBlock {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

/// Anthropic request
#[derive(Serialize)]
struct AnthropicRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<AnthropicTool>>,
}

/// Anthropic tool definition
#[derive(Serialize)]
struct AnthropicTool {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    input_schema: serde_json::Value,
}

/// Anthropic response
#[derive(Deserialize)]
struct AnthropicResponse {
    id: String,
    #[serde(rename = "type")]
    response_type: String,
    role: String,
    content: Vec<AnthropicResponseContent>,
    model: String,
    #[serde(default)]
    stop_reason: Option<String>,
    #[serde(default)]
    usage: Option<AnthropicUsage>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicResponseContent {
    Text { text: String },
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
}

#[derive(Deserialize)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

#[async_trait]
impl Provider for AnthropicProvider {
    fn name(&self) -> &str {
        "anthropic"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn Error + Send + Sync>> {
        self.send_message_with_tools(messages, &[]).await
    }

    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let (system, anthropic_messages) = Self::messages_to_anthropic(messages);

        // Convert tools to Anthropic format
        let anthropic_tools: Option<Vec<AnthropicTool>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .filter_map(|tool| {
                        // Tool format: {"type": "function", "function": {"name": ..., "description": ..., "parameters": ...}}
                        if let Some(function) = tool.get("function") {
                            Some(AnthropicTool {
                                name: function.get("name")?.as_str()?.to_string(),
                                description: function.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                                input_schema: function.get("parameters").cloned().unwrap_or(serde_json::json!({})),
                            })
                        } else {
                            None
                        }
                    })
                    .collect()
            )
        };

        let request = AnthropicRequest {
            model: self.model.clone(),
            messages: anthropic_messages,
            max_tokens: 4096,
            system,
            tools: anthropic_tools,
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-dangerous-direct-browser-access", "true")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Anthropic API error: {}", error_text).into());
        }

        let anthropic_response: AnthropicResponse = response.json().await?;

        // Convert response to Message
        let mut blocks: Vec<ContentBlock> = Vec::new();

        for content in anthropic_response.content {
            match content {
                AnthropicResponseContent::Text { text } => {
                    blocks.push(ContentBlock::text(text));
                }
                AnthropicResponseContent::ToolUse { id, name, input } => {
                    blocks.push(ContentBlock::tool_use(id, name, input));
                }
            }
        }

        Ok(Message::assistant_with_tools(blocks))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn Error + Send + Sync>> {
        // For simplicity, use non-streaming for now
        let response = self.send_message(messages).await?;
        Ok(response.text_content())
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "claude-3-5-sonnet-20241022".to_string(),
            "claude-3-5-haiku-20241022".to_string(),
            "claude-3-opus-20240229".to_string(),
            "claude-3-sonnet-20240229".to_string(),
            "claude-3-haiku-20240307".to_string(),
        ]
    }
}
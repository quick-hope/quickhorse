//! OpenAI provider implementation

use crate::provider::{ContentBlock, Message, Provider};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Default OpenAI API URL
const DEFAULT_OPENAI_API_URL: &str = "https://api.openai.com/v1/chat/completions";

/// OpenAI provider
pub struct OpenAIProvider {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Model to use
    model: String,
    /// Base URL for API (custom endpoints like BaiLian, etc.)
    base_url: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider with default API URL
    pub fn new(api_key: String, model: String) -> Self {
        Self::new_with_base_url(api_key, model, DEFAULT_OPENAI_API_URL.to_string())
    }

    /// Create a new OpenAI provider with custom base URL
    pub fn new_with_base_url(api_key: String, model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
        }
    }

    /// Create with default model
    pub fn with_default_model(api_key: String) -> Self {
        Self::new(api_key, "gpt-4".to_string())
    }

    /// Convert messages to OpenAI format
    fn messages_to_openai(messages: &[Message]) -> Vec<serde_json::Value> {
        messages
            .iter()
            .map(|msg| {
                // OpenAI format: content is string for simple messages
                if msg.content.len() == 1 {
                    if let ContentBlock::Text { text } = &msg.content[0] {
                        return serde_json::json!({
                            "role": msg.role,
                            "content": text
                        });
                    }
                }

                // For complex messages, use the content array
                let content: Vec<serde_json::Value> = msg
                    .content
                    .iter()
                    .map(|block| {
                        match block {
                            ContentBlock::Text { text } => {
                                serde_json::json!({ "type": "text", "text": text })
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                serde_json::json!({
                                    "type": "function",
                                    "id": id,
                                    "function": {
                                        "name": name,
                                        "arguments": input.to_string()
                                    }
                                })
                            }
                            ContentBlock::ToolResult { tool_use_id, content, is_error: _ } => {
                                serde_json::json!({
                                    "role": "tool",
                                    "tool_call_id": tool_use_id,
                                    "content": content
                                })
                            }
                        }
                    })
                    .collect();

                serde_json::json!({
                    "role": msg.role,
                    "content": content
                })
            })
            .collect()
    }
}

/// OpenAI chat request
#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    stream: Option<bool>,
}

/// OpenAI chat response
#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct ResponseMessage {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Debug, Deserialize)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Deserialize)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[async_trait]
impl Provider for OpenAIProvider {
    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn Error + Send + Sync>> {
        self.send_message_with_tools(messages, &[]).await
    }

    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let openai_messages = Self::messages_to_openai(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: openai_messages,
            tools: if tools.is_empty() { None } else { Some(tools.to_vec()) },
            stream: None,
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {}", error_text).into());
        }

        let chat_response: ChatResponse = response.json().await?;

        let choice = chat_response
            .choices
            .first()
            .ok_or("No response from API")?;

        // Convert response to Message
        let _content_blocks: Vec<ContentBlock> = Vec::new();

        // Add text content if present
        let mut blocks = Vec::new();
        if let Some(text) = &choice.message.content {
            if !text.is_empty() {
                blocks.push(ContentBlock::text(text.clone()));
            }
        }

        // Add tool calls if present
        if let Some(tool_calls) = &choice.message.tool_calls {
            for tc in tool_calls {
                // Parse arguments from JSON string
                let input: serde_json::Value = serde_json::from_str(&tc.function.arguments)
                    .unwrap_or(serde_json::json!({ "raw": tc.function.arguments }));

                blocks.push(ContentBlock::tool_use(
                    tc.id.clone(),
                    tc.function.name.clone(),
                    input,
                ));
            }
        }

        Ok(Message::assistant_with_tools(blocks))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn Error + Send + Sync>> {
        let openai_messages = Self::messages_to_openai(messages);

        let request = ChatRequest {
            model: self.model.clone(),
            messages: openai_messages,
            tools: None,
            stream: Some(true),
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("OpenAI API error: {}", error_text).into());
        }

        let full_response = response.text().await?;

        // Parse SSE stream
        let mut content = String::new();
        for line in full_response.lines() {
            if line.starts_with("data: ") {
                let data = &line[6..];
                if data == "[DONE]" {
                    break;
                }

                if let Ok(stream_response) = serde_json::from_str::<StreamResponse>(data) {
                    if let Some(choice) = stream_response.choices.first() {
                        if let Some(delta) = &choice.delta {
                            if let Some(delta_content) = &delta.content {
                                content.push_str(delta_content);
                            }
                        }
                    }
                }
            }
        }

        Ok(content)
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "gpt-4".to_string(),
            "gpt-4-turbo".to_string(),
            "gpt-4o".to_string(),
            "gpt-4o-mini".to_string(),
            "gpt-3.5-turbo".to_string(),
        ]
    }
}

/// OpenAI streaming response
#[derive(Deserialize)]
struct StreamResponse {
    choices: Vec<StreamChoice>,
}

#[derive(Deserialize)]
struct StreamChoice {
    delta: Option<Delta>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
}
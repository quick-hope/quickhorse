//! Provider module - LLM provider implementations

mod anthropic;
mod gemini;
mod ollama;
mod openai;
mod stream;

pub use anthropic::AnthropicProvider;
pub use gemini::GeminiProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use stream::{StreamEvent, StreamReceiver, StreamSender, create_stream_channel};

use async_trait::async_trait;
use itertools::Itertools;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Content block types for messages
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ContentBlock {
    /// Text content
    Text { text: String },
    /// Tool use request from assistant
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
    },
    /// Tool result from user
    ToolResult {
        tool_use_id: String,
        content: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        is_error: Option<bool>,
    },
}

impl ContentBlock {
    /// Create a text block
    pub fn text(content: String) -> Self {
        ContentBlock::Text { text: content }
    }

    /// Create a tool use block
    pub fn tool_use(id: String, name: String, input: serde_json::Value) -> Self {
        ContentBlock::ToolUse { id, name, input }
    }

    /// Create a tool result block
    pub fn tool_result(tool_use_id: String, content: String, is_error: bool) -> Self {
        ContentBlock::ToolResult {
            tool_use_id,
            content,
            is_error: Some(is_error),
        }
    }
}

/// A chat message with content blocks
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    /// Role: "user", "assistant", or "system"
    pub role: String,
    /// Content - either a string or array of content blocks
    #[serde(with = "content_serde")]
    pub content: Vec<ContentBlock>,
}

/// Custom serialization for content (handles both string and array formats)
mod content_serde {
    use super::ContentBlock;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(content: &[ContentBlock], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // If only one text block, serialize as string for compatibility
        if content.len() == 1 {
            if let ContentBlock::Text { text } = &content[0] {
                return serializer.serialize_str(text);
            }
        }
        // Otherwise serialize as array
        content.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<ContentBlock>, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::Error;

        let value = serde_json::Value::deserialize(deserializer)?;
        match value {
            serde_json::Value::String(s) => Ok(vec![ContentBlock::text(s)]),
            serde_json::Value::Array(arr) => {
                arr.into_iter()
                    .map(|v| ContentBlock::deserialize(v).map_err(|e| D::Error::custom(e)))
                    .collect()
            }
            _ => Err(D::Error::custom("content must be string or array")),
        }
    }
}

impl Message {
    /// Create a simple user message
    pub fn user(content: String) -> Self {
        Self {
            role: "user".to_string(),
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create a simple assistant message
    pub fn assistant(content: String) -> Self {
        Self {
            role: "assistant".to_string(),
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create a system message
    pub fn system(content: String) -> Self {
        Self {
            role: "system".to_string(),
            content: vec![ContentBlock::text(content)],
        }
    }

    /// Create an assistant message with tool uses
    pub fn assistant_with_tools(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: blocks,
        }
    }

    /// Create a user message with tool results
    pub fn user_with_tool_results(blocks: Vec<ContentBlock>) -> Self {
        Self {
            role: "user".to_string(),
            content: blocks,
        }
    }

    /// Get text content (for simple messages)
    pub fn text_content(&self) -> String {
        self.content
            .iter()
            .filter_map(|block| {
                if let ContentBlock::Text { text } = block {
                    Some(text.clone())
                } else {
                    None
                }
            })
            .join("\n")
    }

    /// Get tool uses (for assistant messages)
    pub fn tool_uses(&self) -> Vec<&ContentBlock> {
        self.content
            .iter()
            .filter(|block| matches!(block, ContentBlock::ToolUse { .. }))
            .collect()
    }

    /// Check if message contains tool uses
    #[allow(dead_code)]
    pub fn has_tool_uses(&self) -> bool {
        self.content.iter().any(|block| matches!(block, ContentBlock::ToolUse { .. }))
    }

    /// Convert to OpenAI API format
    #[allow(dead_code)]
    pub fn to_openai_format(&self) -> serde_json::Value {
        let content: Vec<serde_json::Value> = self
            .content
            .iter()
            .map(|block| {
                match block {
                    ContentBlock::Text { text } => {
                        serde_json::json!({ "type": "text", "text": text })
                    }
                    ContentBlock::ToolUse { id, name, input } => {
                        serde_json::json!({
                            "type": "tool_use",
                            "id": id,
                            "name": name,
                            "input": input
                        })
                    }
                    ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                        serde_json::json!({
                            "type": "tool_result",
                            "tool_use_id": tool_use_id,
                            "content": content,
                            "is_error": is_error
                        })
                    }
                }
            })
            .collect();

        serde_json::json!({
            "role": self.role,
            "content": content
        })
    }
}

/// Provider trait for LLM implementations
#[async_trait]
#[allow(dead_code)]
pub trait Provider: Send + Sync {
    /// Get provider name
    fn name(&self) -> &str;

    /// Get current model name
    fn model(&self) -> &str;

    /// Set model name
    fn set_model(&mut self, model: String);

    /// Send a message and get a response (non-streaming)
    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn Error + Send + Sync>>;

    /// Send a message with tool support
    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn Error + Send + Sync>>;

    /// Send a message and stream the response (returns full accumulated content)
    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn Error + Send + Sync>>;

    /// Send a message with real-time streaming via channel
    /// Returns a receiver that yields StreamEvent
    async fn stream_message_channel(
        &self,
        messages: &[Message],
    ) -> Result<StreamReceiver, Box<dyn Error + Send + Sync>>;

    /// List available models
    fn list_models(&self) -> Vec<String>;
}

/// Provider types
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[allow(dead_code)]
pub enum ProviderType {
    OpenAI,
    Anthropic,
    Gemini,
    Ollama,
}

impl std::fmt::Display for ProviderType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProviderType::OpenAI => write!(f, "openai"),
            ProviderType::Anthropic => write!(f, "anthropic"),
            ProviderType::Gemini => write!(f, "gemini"),
            ProviderType::Ollama => write!(f, "ollama"),
        }
    }
}

impl std::str::FromStr for ProviderType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "openai" => Ok(ProviderType::OpenAI),
            "anthropic" => Ok(ProviderType::Anthropic),
            "gemini" => Ok(ProviderType::Gemini),
            "ollama" => Ok(ProviderType::Ollama),
            _ => Err(format!("Unknown provider: {}", s)),
        }
    }
}
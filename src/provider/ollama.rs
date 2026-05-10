//! Ollama (local models) provider implementation

use crate::provider::{ContentBlock, Message, Provider};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Default Ollama API URL
const DEFAULT_OLLAMA_API_URL: &str = "http://localhost:11434/api/chat";

/// Ollama provider
pub struct OllamaProvider {
    /// HTTP client
    client: Client,
    /// Model to use
    model: String,
    /// Base URL for API
    base_url: String,
}

impl OllamaProvider {
    /// Create a new Ollama provider with default API URL
    pub fn new(model: String) -> Self {
        Self::new_with_base_url(model, DEFAULT_OLLAMA_API_URL.to_string())
    }

    /// Create a new Ollama provider with custom base URL
    pub fn new_with_base_url(model: String, base_url: String) -> Self {
        Self {
            client: Client::new(),
            model,
            base_url,
        }
    }

    /// Create with default model (llama3)
    #[allow(dead_code)]
    pub fn with_default_model() -> Self {
        Self::new("llama3".to_string())
    }

    /// Convert messages to Ollama format
    fn messages_to_ollama(messages: &[Message]) -> Vec<OllamaMessage> {
        messages
            .iter()
            .filter(|msg| msg.role != "system")
            .map(|msg| {
                OllamaMessage {
                    role: msg.role.clone(),
                    content: msg.text_content(),
                }
            })
            .collect()
    }

    /// Get system prompt from messages
    fn get_system_prompt(messages: &[Message]) -> Option<String> {
        messages
            .iter()
            .filter(|msg| msg.role == "system")
            .map(|msg| msg.text_content())
            .next()
    }

    /// Convert tools to Ollama format
    fn tools_to_ollama(tools: &[serde_json::Value]) -> Vec<OllamaTool> {
        tools
            .iter()
            .filter_map(|t| {
                if let Some(func) = t.get("function") {
                    Some(OllamaTool {
                        tool_type: "function".to_string(),
                        function: OllamaFunction {
                            name: func.get("name")?.as_str()?.to_string(),
                            description: func.get("description").and_then(|d| d.as_str()).map(|s| s.to_string()),
                            parameters: func.get("parameters")?.clone(),
                        },
                    })
                } else {
                    None
                }
            })
            .collect()
    }
}

/// Ollama message format
#[derive(Serialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

/// Ollama request
#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<OllamaTool>>,
}

/// Ollama tool definition
#[derive(Serialize)]
struct OllamaTool {
    #[serde(rename = "type")]
    tool_type: String,
    function: OllamaFunction,
}

/// Ollama function definition
#[derive(Serialize)]
struct OllamaFunction {
    name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<String>,
    parameters: serde_json::Value,
}

/// Ollama response
#[derive(Deserialize)]
struct OllamaResponse {
    message: OllamaResponseMessage,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct OllamaResponseMessage {
    role: String,
    content: String,
    #[serde(default)]
    tool_calls: Option<Vec<OllamaToolCall>>,
}

#[derive(Debug, Deserialize)]
struct OllamaToolCall {
    function: OllamaFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OllamaFunctionCall {
    name: String,
    arguments: serde_json::Value,
}

#[async_trait]
impl Provider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            messages: Self::messages_to_ollama(messages),
            stream: false,
            system: Self::get_system_prompt(messages),
            tools: None,
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Ollama API error: {}", error_text).into());
        }

        let ollama_response: OllamaResponse = response.json().await?;

        let mut blocks: Vec<ContentBlock> = Vec::new();
        
        if !ollama_response.message.content.is_empty() {
            blocks.push(ContentBlock::text(ollama_response.message.content));
        }

        if let Some(tool_calls) = &ollama_response.message.tool_calls {
            for tc in tool_calls {
                blocks.push(ContentBlock::tool_use(
                    format!("call_{}", uuid::Uuid::new_v4()),
                    tc.function.name.clone(),
                    tc.function.arguments.clone(),
                ));
            }
        }

        Ok(Message::assistant_with_tools(blocks))
    }

    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let request = OllamaRequest {
            model: self.model.clone(),
            messages: Self::messages_to_ollama(messages),
            stream: false,
            system: Self::get_system_prompt(messages),
            tools: if tools.is_empty() { None } else { Some(Self::tools_to_ollama(tools)) },
        };

        let response = self
            .client
            .post(&self.base_url)
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Ollama API error: {}", error_text).into());
        }

        let ollama_response: OllamaResponse = response.json().await?;

        let mut blocks: Vec<ContentBlock> = Vec::new();
        
        if !ollama_response.message.content.is_empty() {
            blocks.push(ContentBlock::text(ollama_response.message.content));
        }

        if let Some(tool_calls) = &ollama_response.message.tool_calls {
            for tc in tool_calls {
                blocks.push(ContentBlock::tool_use(
                    format!("call_{}", uuid::Uuid::new_v4()),
                    tc.function.name.clone(),
                    tc.function.arguments.clone(),
                ));
            }
        }

        Ok(Message::assistant_with_tools(blocks))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn Error + Send + Sync>> {
        let response = self.send_message(messages).await?;
        Ok(response.text_content())
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "llama3".to_string(),
            "llama3:8b".to_string(),
            "llama2".to_string(),
            "mistral".to_string(),
            "codellama".to_string(),
            "deepseek-coder".to_string(),
            "qwen2".to_string(),
            "phi3".to_string(),
            "gemma".to_string(),
        ]
    }
}
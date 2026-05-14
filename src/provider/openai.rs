//! OpenAI provider implementation

use crate::provider::{ContentBlock, Message, Provider, StreamEvent, StreamReceiver, create_stream_channel, stream::sse};
use crate::error::{ErrorCode, QuickHorseError, classify_provider_error, classify_reqwest_error};
use async_trait::async_trait;
use futures::StreamExt;
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
    #[allow(dead_code)]
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
#[allow(dead_code)]
struct ChatResponse {
    choices: Vec<Choice>,
    #[serde(default)]
    usage: Option<Usage>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct Choice {
    message: ResponseMessage,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
#[allow(dead_code)]
struct ResponseMessage {
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    tool_calls: Option<Vec<ToolCall>>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
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
#[allow(dead_code)]
struct Usage {
    prompt_tokens: u64,
    completion_tokens: u64,
    total_tokens: u64,
}

#[async_trait]
impl Provider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
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
            .header("User-Agent", "quickhorse/0.1.0")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                let err = classify_reqwest_error(&e);
                Box::new(std::io::Error::new(std::io::ErrorKind::Other, err.to_user_message_full())) as Box<dyn Error + Send + Sync>
            })?;

        if !response.status().is_success() {
            let status = response.status().as_u16();
            let error_text = response.text().await.unwrap_or_default();
            let err = classify_provider_error("openai", status, &error_text);
            return Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, err.to_user_message_full())) as Box<dyn Error + Send + Sync>);
        }

        let chat_response: ChatResponse = response.json().await?;

        let choice = chat_response
            .choices
            .first()
            .ok_or("No response from API")?;

        // Convert response to Message
        let mut blocks = Vec::new();
        if let Some(text) = &choice.message.content {
            if !text.is_empty() {
                blocks.push(ContentBlock::text(text.clone()));
            }
        }

        // Add tool calls if present
        if let Some(tool_calls) = &choice.message.tool_calls {
            for tc in tool_calls {
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
        let (tx, mut rx) = create_stream_channel();

        // Start streaming in background
        let openai_messages = Self::messages_to_openai(messages);
        let request = ChatRequest {
            model: self.model.clone(),
            messages: openai_messages,
            tools: None,
            stream: Some(true),
        };

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        tokio::spawn(async move {
            Self::stream_task(client, base_url, api_key, request, tx).await;
        });

        // Accumulate content from stream
        let mut content = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::TextDelta(text) => content.push_str(&text),
                StreamEvent::Done => break,
                StreamEvent::Error(e) => {
                    return Err(e.into());
                }
                _ => {}
            }
        }

        Ok(content)
    }

    async fn stream_message_channel(
        &self,
        messages: &[Message],
    ) -> Result<StreamReceiver, Box<dyn Error + Send + Sync>> {
        let (tx, rx) = create_stream_channel();

        let openai_messages = Self::messages_to_openai(messages);
        let request = ChatRequest {
            model: self.model.clone(),
            messages: openai_messages,
            tools: None,
            stream: Some(true),
        };

        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();

        // Spawn background task for streaming
        tokio::spawn(async move {
            Self::stream_task(client, base_url, api_key, request, tx).await;
        });

        Ok(rx)
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

impl OpenAIProvider {
    /// Background streaming task
    async fn stream_task(
        client: Client,
        base_url: String,
        api_key: String,
        request: ChatRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) {
        let response = client
            .post(&base_url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .header("User-Agent", "quickhorse/0.1.0")
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let status = resp.status().as_u16();
                    let error_text = resp.text().await.unwrap_or_default();
                    let err = classify_provider_error("openai", status, &error_text);
                    // Send user-friendly error message
                    tx.send(StreamEvent::Error(err.to_user_message())).await.ok();
                    return;
                }

                // True streaming using bytes_stream()
                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();
                // Track tool call arguments accumulation
                let mut tool_call_args: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Process complete SSE lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line_str = buffer[..newline_pos].to_string();
                                let line = line_str.trim();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if line.is_empty() {
                                    continue;
                                }

                                // Parse SSE data line
                                if let Some(data) = sse::parse_data_line(line) {
                                    if sse::is_done(data) {
                                        // Send ToolCallComplete for all accumulated tool calls
                                        for (id, (name, args)) in &tool_call_args {
                                            tx.send(StreamEvent::ToolCallComplete {
                                                id: id.clone(),
                                                name: name.clone(),
                                                arguments: args.clone(),
                                            }).await.ok();
                                        }
                                        tx.send(StreamEvent::Done).await.ok();
                                        return;
                                    }

                                    // Parse JSON and extract delta
                                    if let Ok(stream_resp) = serde_json::from_str::<StreamResponse>(data) {
                                        if let Some(choice) = stream_resp.choices.first() {
                                            // Text delta
                                            if let Some(delta) = &choice.delta {
                                                // Skip reasoning_content (thinking process) - don't output
                                                // Handle content (actual response)
                                                if let Some(content) = &delta.content {
                                                    if !content.is_empty() {
                                                        tx.send(StreamEvent::TextDelta(content.clone())).await.ok();
                                                    }
                                                }
                                            }

                                            // Tool call delta
                                            if let Some(tool_calls) = &choice.delta_tool_calls {
                                                for tc in tool_calls {
                                                    if let Some(function) = &tc.function {
                                                        if let Some(name) = &function.name {
                                                            tx.send(StreamEvent::ToolCallStart {
                                                                id: tc.id.clone(),
                                                                name: name.clone(),
                                                            }).await.ok();
                                                            // Initialize tool call tracking
                                                            tool_call_args.insert(tc.id.clone(), (name.clone(), String::new()));
                                                        }
                                                        if let Some(args) = &function.arguments {
                                                            if !args.is_empty() {
                                                                tx.send(StreamEvent::ToolCallDelta {
                                                                    id: tc.id.clone(),
                                                                    arguments: args.clone(),
                                                                }).await.ok();
                                                                // Accumulate arguments
                                                                if let Some((_, accumulated)) = tool_call_args.get_mut(&tc.id) {
                                                                    accumulated.push_str(args);
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            // Network/stream error
                            let err = QuickHorseError::new(ErrorCode::STREAMING_ERROR)
                                .with_details(e.to_string())
                                .with_retryable(true);
                            tx.send(StreamEvent::Error(err.to_user_message())).await.ok();
                            return;
                        }
                    }
                }

                // Stream ended without explicit [DONE]
                // Send ToolCallComplete for all accumulated tool calls
                for (id, (name, args)) in &tool_call_args {
                    tx.send(StreamEvent::ToolCallComplete {
                        id: id.clone(),
                        name: name.clone(),
                        arguments: args.clone(),
                    }).await.ok();
                }
                tx.send(StreamEvent::Done).await.ok();
            }
            Err(e) => {
                // Use error classification
                let err = classify_reqwest_error(&e);
                tx.send(StreamEvent::Error(err.to_user_message())).await.ok();
            }
        }
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
    #[serde(default)]
    delta_tool_calls: Option<Vec<DeltaToolCall>>,
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct Delta {
    content: Option<String>,
    /// Reasoning content for models like qwen3.6-plus (thinking process)
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Deserialize)]
struct DeltaToolCall {
    index: Option<u32>,
    id: String,
    #[serde(rename = "type")]
    call_type: Option<String>,
    function: Option<DeltaFunction>,
}

#[derive(Deserialize)]
struct DeltaFunction {
    name: Option<String>,
    arguments: Option<String>,
}
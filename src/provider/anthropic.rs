//! Anthropic (Claude) provider implementation

use crate::provider::{ContentBlock, Message, Provider, StreamEvent, StreamReceiver, create_stream_channel, stream::sse};
use async_trait::async_trait;
use futures::StreamExt;
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
    #[allow(dead_code)]
    pub fn with_default_model(api_key: String) -> Self {
        Self::new(api_key, "claude-3-5-sonnet-20241022".to_string())
    }

    /// Convert messages to Anthropic format
    fn messages_to_anthropic(messages: &[Message]) -> (Option<String>, Vec<AnthropicMessage>) {
        let mut system_prompt: Option<String> = None;
        let mut anthropic_messages: Vec<AnthropicMessage> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
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

    /// Build streaming request
    fn build_stream_request(messages: &[Message]) -> AnthropicStreamRequest {
        let (system, anthropic_messages) = Self::messages_to_anthropic(messages);
        AnthropicStreamRequest {
            model: "claude-3-5-sonnet-20241022".to_string(),
            messages: anthropic_messages,
            max_tokens: 4096,
            stream: true,
            system,
        }
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

/// Anthropic streaming request
#[derive(Serialize)]
struct AnthropicStreamRequest {
    model: String,
    messages: Vec<AnthropicMessage>,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    system: Option<String>,
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
#[allow(dead_code)]
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
#[allow(dead_code)]
struct AnthropicUsage {
    input_tokens: u64,
    output_tokens: u64,
}

/// Anthropic streaming event types
#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamEvent {
    MessageStart { message: AnthropicStreamMessage },
    ContentBlockStart { index: u32, content_block: AnthropicStreamContentBlock },
    ContentBlockDelta { index: u32, delta: AnthropicStreamDelta },
    ContentBlockStop { index: u32 },
    MessageDelta { delta: AnthropicStreamMessageDelta, usage: Option<AnthropicStreamUsage> },
    MessageStop,
    Ping,
    Error { error: AnthropicStreamError },
}

#[derive(Deserialize)]
struct AnthropicStreamMessage {
    id: String,
    model: String,
    role: String,
    #[serde(default)]
    content: Vec<AnthropicResponseContent>,
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamContentBlock {
    Text { text: String },
    ToolUse { id: String, name: String },
}

#[derive(Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum AnthropicStreamDelta {
    TextDelta { text: String },
    InputJsonDelta { partial_json: String },
}

#[derive(Deserialize)]
struct AnthropicStreamMessageDelta {
    #[serde(default)]
    stop_reason: Option<String>,
}

#[derive(Deserialize)]
struct AnthropicStreamUsage {
    output_tokens: u64,
}

#[derive(Deserialize)]
struct AnthropicStreamError {
    #[serde(rename = "type")]
    error_type: String,
    message: String,
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

        let anthropic_tools: Option<Vec<AnthropicTool>> = if tools.is_empty() {
            None
        } else {
            Some(
                tools
                    .iter()
                    .filter_map(|tool| {
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
        let (tx, mut rx) = create_stream_channel();

        let request = Self::build_stream_request(messages);
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        tokio::spawn(async move {
            Self::stream_task(client, base_url, api_key, model, request, tx).await;
        });

        let mut content = String::new();
        while let Some(event) = rx.recv().await {
            match event {
                StreamEvent::TextDelta(text) => content.push_str(&text),
                StreamEvent::Done => break,
                StreamEvent::Error(e) => return Err(e.into()),
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

        let request = Self::build_stream_request(messages);
        let client = self.client.clone();
        let base_url = self.base_url.clone();
        let api_key = self.api_key.clone();
        let model = self.model.clone();

        tokio::spawn(async move {
            Self::stream_task(client, base_url, api_key, model, request, tx).await;
        });

        Ok(rx)
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

impl AnthropicProvider {
    /// Background streaming task for Anthropic
    async fn stream_task(
        client: Client,
        base_url: String,
        api_key: String,
        model: String,
        request: AnthropicStreamRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) {
        // Build request with the correct model
        let request = AnthropicStreamRequest {
            model,
            messages: request.messages,
            max_tokens: request.max_tokens,
            stream: true,
            system: request.system,
        };

        let response = client
            .post(&base_url)
            .header("x-api-key", &api_key)
            .header("Content-Type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-dangerous-direct-browser-access", "true")
            .json(&request)
            .send()
            .await;

        match response {
            Ok(resp) => {
                if !resp.status().is_success() {
                    let error_text = resp.text().await.unwrap_or_else(|_| "Unknown error".to_string());
                    tx.send(StreamEvent::Error(format!("API error: {}", error_text))).await.ok();
                    return;
                }

                // Anthropic uses SSE format
                let mut stream = resp.bytes_stream();
                let mut buffer = String::new();
                // Track tool call arguments accumulation
                let mut tool_call_args: std::collections::HashMap<String, (String, String)> = std::collections::HashMap::new();
                let mut current_tool_id: Option<String> = None;

                while let Some(chunk_result) = stream.next().await {
                    match chunk_result {
                        Ok(bytes) => {
                            buffer.push_str(&String::from_utf8_lossy(&bytes));

                            // Process lines
                            while let Some(newline_pos) = buffer.find('\n') {
                                let line_str = buffer[..newline_pos].to_string();
                                let line = line_str.trim();
                                buffer = buffer[newline_pos + 1..].to_string();

                                if line.is_empty() {
                                    continue;
                                }

                                // Parse event type
                                if let Some(_event_type) = sse::parse_event_line(line) {
                                    continue;
                                }

                                // Parse data
                                if let Some(data) = sse::parse_data_line(line) {
                                    if let Ok(event) = serde_json::from_str::<AnthropicStreamEvent>(data) {
                                        match event {
                                            AnthropicStreamEvent::ContentBlockDelta { index: _, delta } => {
                                                match delta {
                                                    AnthropicStreamDelta::TextDelta { text } => {
                                                        if !text.is_empty() {
                                                            tx.send(StreamEvent::TextDelta(text)).await.ok();
                                                        }
                                                    }
                                                    AnthropicStreamDelta::InputJsonDelta { partial_json } => {
                                                        // Tool arguments delta
                                                        if let Some(id) = &current_tool_id {
                                                            tx.send(StreamEvent::ToolCallDelta {
                                                                id: id.clone(),
                                                                arguments: partial_json.clone(),
                                                            }).await.ok();
                                                            // Accumulate arguments
                                                            if let Some((_, accumulated)) = tool_call_args.get_mut(id) {
                                                                accumulated.push_str(&partial_json);
                                                            }
                                                        }
                                                    }
                                                }
                                            }
                                            AnthropicStreamEvent::ContentBlockStart { index: _, content_block } => {
                                                match content_block {
                                                    AnthropicStreamContentBlock::ToolUse { id, name } => {
                                                        tx.send(StreamEvent::ToolCallStart { id: id.clone(), name: name.clone() }).await.ok();
                                                        // Initialize tool call tracking
                                                        tool_call_args.insert(id.clone(), (name.clone(), String::new()));
                                                        current_tool_id = Some(id);
                                                    }
                                                    _ => {}
                                                }
                                            }
                                            AnthropicStreamEvent::ContentBlockStop { index: _ } => {
                                                // Tool call complete for current block
                                                if let Some(id) = &current_tool_id {
                                                    if let Some((name, args)) = tool_call_args.get(id) {
                                                        tx.send(StreamEvent::ToolCallComplete {
                                                            id: id.clone(),
                                                            name: name.clone(),
                                                            arguments: args.clone(),
                                                        }).await.ok();
                                                    }
                                                }
                                                current_tool_id = None;
                                            }
                                            AnthropicStreamEvent::MessageStop => {
                                                // Send ToolCallComplete for any remaining tool calls
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
                                            AnthropicStreamEvent::Error { error } => {
                                                tx.send(StreamEvent::Error(format!("{}: {}", error.error_type, error.message))).await.ok();
                                                return;
                                            }
                                            _ => {}
                                        }
                                    }
                                }
                            }
                        }
                        Err(e) => {
                            tx.send(StreamEvent::Error(format!("Stream error: {}", e))).await.ok();
                            return;
                        }
                    }
                }

                // Stream ended without message_stop
                tx.send(StreamEvent::Done).await.ok();
            }
            Err(e) => {
                tx.send(StreamEvent::Error(format!("Request failed: {}", e))).await.ok();
            }
        }
    }
}
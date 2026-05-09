//! Gemini (Google AI) provider implementation

use crate::provider::{ContentBlock, Message, Provider};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Default Gemini API URL template (model name will be inserted)
const DEFAULT_GEMINI_API_TEMPLATE: &str = "https://generativelanguage.googleapis.com/v1beta/models";

/// Gemini provider
pub struct GeminiProvider {
    /// HTTP client
    client: Client,
    /// API key
    api_key: String,
    /// Model to use
    model: String,
    /// Base URL for API
    base_url: String,
}

impl GeminiProvider {
    /// Create a new Gemini provider with default API URL
    pub fn new(api_key: String, model: String) -> Self {
        Self::new_with_base_url(api_key, model, DEFAULT_GEMINI_API_TEMPLATE.to_string())
    }

    /// Create a new Gemini provider with custom base URL
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
        Self::new(api_key, "gemini-1.5-pro".to_string())
    }

    /// Build the full API URL for generateContent
    fn build_url(&self) -> String {
        format!("{}:generateContent?key={}", self.base_url, self.api_key)
    }

    /// Convert messages to Gemini format
    fn messages_to_gemini(messages: &[Message]) -> GeminiRequest {
        let mut system_instruction: Option<GeminiContent> = None;
        let mut contents: Vec<GeminiContent> = Vec::new();

        for msg in messages {
            if msg.role == "system" {
                // Gemini uses systemInstruction
                system_instruction = Some(GeminiContent {
                    role: "user".to_string(), // Gemini treats system as user
                    parts: vec![GeminiPart::Text { text: msg.text_content() }],
                });
            } else {
                let role = match msg.role.as_str() {
                    "user" => "user",
                    "assistant" => "model",
                    _ => msg.role.as_str(),
                };

                let parts: Vec<GeminiPart> = msg
                    .content
                    .iter()
                    .map(|block| {
                        match block {
                            ContentBlock::Text { text } => {
                                GeminiPart::Text { text: text.clone() }
                            }
                            ContentBlock::ToolUse { id, name, input } => {
                                GeminiPart::FunctionCall {
                                    name: name.clone(),
                                    args: input.clone(),
                                    id: Some(id.clone()),
                                }
                            }
                            ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                                GeminiPart::FunctionResponse {
                                    name: tool_use_id.clone(),
                                    response: serde_json::json!({
                                        "content": content,
                                        "is_error": is_error,
                                    }),
                                }
                            }
                        }
                    })
                    .collect();

                contents.push(GeminiContent {
                    role: role.to_string(),
                    parts,
                });
            }
        }

        GeminiRequest {
            contents,
            system_instruction,
            generation_config: Some(GeminiGenerationConfig {
                temperature: Some(1.0),
                max_output_tokens: Some(8192),
            }),
            tools: None,
        }
    }

    /// Convert tools to Gemini format
    fn tools_to_gemini(tools: &[serde_json::Value]) -> Option<GeminiTools> {
        if tools.is_empty() {
            return None;
        }

        let function_declarations: Vec<GeminiFunctionDeclaration> = tools
            .iter()
            .filter_map(|tool| {
                if let Some(function) = tool.get("function") {
                    Some(GeminiFunctionDeclaration {
                        name: function.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string(),
                        description: function.get("description").and_then(|d| d.as_str()).unwrap_or("").to_string(),
                        parameters: function.get("parameters").cloned().unwrap_or(serde_json::json!({})),
                    })
                } else {
                    None
                }
            })
            .collect();

        if function_declarations.is_empty() {
            None
        } else {
            Some(GeminiTools {
                function_declarations,
            })
        }
    }
}

/// Gemini request
#[derive(Serialize)]
struct GeminiRequest {
    contents: Vec<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    system_instruction: Option<GeminiContent>,
    #[serde(skip_serializing_if = "Option::is_none")]
    generation_config: Option<GeminiGenerationConfig>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<GeminiTools>,
}

/// Gemini content
#[derive(Serialize)]
struct GeminiContent {
    role: String,
    parts: Vec<GeminiPart>,
}

/// Gemini part
#[derive(Serialize)]
#[serde(untagged)]
enum GeminiPart {
    Text { text: String },
    FunctionCall {
        name: String,
        args: serde_json::Value,
        #[serde(skip_serializing_if = "Option::is_none")]
        id: Option<String>,
    },
    FunctionResponse {
        name: String,
        response: serde_json::Value,
    },
}

/// Gemini generation config
#[derive(Serialize)]
struct GeminiGenerationConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    max_output_tokens: Option<u32>,
}

/// Gemini tools
#[derive(Serialize)]
struct GeminiTools {
    function_declarations: Vec<GeminiFunctionDeclaration>,
}

/// Gemini function declaration
#[derive(Serialize)]
struct GeminiFunctionDeclaration {
    name: String,
    description: String,
    parameters: serde_json::Value,
}

/// Gemini response
#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
    #[serde(default)]
    usage_metadata: Option<GeminiUsageMetadata>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: GeminiResponseContent,
    #[serde(default)]
    finish_reason: Option<String>,
}

#[derive(Deserialize)]
struct GeminiResponseContent {
    role: String,
    parts: Vec<GeminiResponsePart>,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum GeminiResponsePart {
    Text { text: String },
    FunctionCall {
        function_call: GeminiFunctionCallResponse,
    },
}

#[derive(Deserialize)]
struct GeminiFunctionCallResponse {
    name: String,
    args: serde_json::Value,
}

#[derive(Deserialize)]
struct GeminiUsageMetadata {
    prompt_token_count: u64,
    candidates_token_count: u64,
    total_token_count: u64,
}

#[async_trait]
impl Provider for GeminiProvider {
    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn Error + Send + Sync>> {
        self.send_message_with_tools(messages, &[]).await
    }

    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn Error + Send + Sync>> {
        let mut request = Self::messages_to_gemini(messages);
        request.tools = Self::tools_to_gemini(tools);

        let response = self
            .client
            .post(self.build_url())
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            return Err(format!("Gemini API error: {}", error_text).into());
        }

        let gemini_response: GeminiResponse = response.json().await?;

        let candidate = gemini_response
            .candidates
            .first()
            .ok_or("No response from Gemini API")?;

        // Convert response to Message
        let mut blocks: Vec<ContentBlock> = Vec::new();

        for part in &candidate.content.parts {
            match part {
                GeminiResponsePart::Text { text } => {
                    blocks.push(ContentBlock::text(text.clone()));
                }
                GeminiResponsePart::FunctionCall { function_call } => {
                    blocks.push(ContentBlock::tool_use(
                        format!("call_{}", uuid::Uuid::new_v4()),
                        function_call.name.clone(),
                        function_call.args.clone(),
                    ));
                }
            }
        }

        Ok(Message::assistant_with_tools(blocks))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn Error + Send + Sync>> {
        // For simplicity, use non-streaming
        let response = self.send_message(messages).await?;
        Ok(response.text_content())
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "gemini-1.5-pro".to_string(),
            "gemini-1.5-flash".to_string(),
            "gemini-1.5-flash-8b".to_string(),
            "gemini-2.0-flash".to_string(),
            "gemini-2.0-flash-lite".to_string(),
            "gemini-2.5-pro-preview".to_string(),
        ]
    }
}
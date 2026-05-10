//! Provider error parsing - Parse provider-specific error formats

use super::{ErrorCode, QuickHorseError};
use super::api::from_http_status_with_body;

/// Parse API error body from different providers
pub fn parse_api_error_body(provider: &str, body: &str) -> Option<QuickHorseError> {
    let json: serde_json::Value = serde_json::from_str(body).ok()?;

    match provider {
        "openai" => parse_openai_error(&json, body),
        "anthropic" => parse_anthropic_error(&json, body),
        "gemini" => parse_gemini_error(&json, body),
        "ollama" => parse_ollama_error(&json, body),
        "bailian" | "dashscope" => parse_bailian_error(&json, body),
        _ => None,
    }
}

/// Parse OpenAI error format
fn parse_openai_error(json: &serde_json::Value, _body: &str) -> Option<QuickHorseError> {
    let error = json.get("error")?;

    let message = error.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("未知错误");

    let error_type = error.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let code = match error_type {
        "invalid_request_error" => ErrorCode::INVALID_REQUEST,
        "authentication_error" => ErrorCode::AUTHENTICATION_FAILED,
        "permission_denied_error" => ErrorCode::PERMISSION_DENIED,
        "not_found_error" => ErrorCode::MODEL_NOT_FOUND,
        "rate_limit_error" => ErrorCode::RATE_LIMIT,
        "api_error" => ErrorCode::SERVER_ERROR,
        "insufficient_quota" => ErrorCode::BILLING_ERROR,
        _ => ErrorCode::SERVER_ERROR,
    };

    // Check for specific patterns in message
    let final_code = if message.contains("context_length") || message.contains("token_limit") {
        ErrorCode::CONTEXT_LENGTH
    } else if message.contains("max_tokens") {
        ErrorCode::MAX_OUTPUT_TOKENS
    } else if message.contains("Invalid model") || message.contains("model_not_found") {
        ErrorCode::MODEL_NOT_FOUND
    } else {
        code
    };

    Some(QuickHorseError::new(final_code)
        .with_details(message.to_string())
        .with_source("openai".to_string()))
}

/// Parse Anthropic error format
fn parse_anthropic_error(json: &serde_json::Value, _body: &str) -> Option<QuickHorseError> {
    let error = json.get("error")?;

    let message = error.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("未知错误");

    let error_type = error.get("type")
        .and_then(|t| t.as_str())
        .unwrap_or("");

    let code = match error_type {
        "invalid_request_error" => ErrorCode::INVALID_REQUEST,
        "authentication_error" => ErrorCode::AUTHENTICATION_FAILED,
        "permission_denied_error" => ErrorCode::PERMISSION_DENIED,
        "not_found_error" => ErrorCode::MODEL_NOT_FOUND,
        "rate_limit_error" => ErrorCode::RATE_LIMIT,
        "api_error" => ErrorCode::SERVER_ERROR,
        "overloaded_error" => ErrorCode::SERVER_ERROR,
        "billing_error" => ErrorCode::BILLING_ERROR,
        _ => ErrorCode::SERVER_ERROR,
    };

    // Check for specific patterns
    let final_code = if message.contains("prompt is too long") {
        ErrorCode::CONTEXT_LENGTH
    } else if message.contains("max_tokens") {
        ErrorCode::MAX_OUTPUT_TOKENS
    } else if message.contains("organization has been disabled") {
        ErrorCode::ORGANIZATION_DISABLED
    } else if message.contains("Invalid API key") {
        ErrorCode::INVALID_API_KEY
    } else {
        code
    };

    Some(QuickHorseError::new(final_code)
        .with_details(message.to_string())
        .with_source("anthropic".to_string()))
}

/// Parse Gemini error format
fn parse_gemini_error(json: &serde_json::Value, _body: &str) -> Option<QuickHorseError> {
    let error = json.get("error")?;

    let message = error.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("未知错误");

    let status = error.get("status")
        .and_then(|s| s.as_str())
        .unwrap_or("");

    let http_code = error.get("code")
        .and_then(|c| c.as_u64())
        .unwrap_or(500) as u16;

    let code = match status {
        "INVALID_ARGUMENT" => ErrorCode::INVALID_REQUEST,
        "NOT_FOUND" => ErrorCode::MODEL_NOT_FOUND,
        "PERMISSION_DENIED" => ErrorCode::PERMISSION_DENIED,
        "RESOURCE_EXHAUSTED" => ErrorCode::RATE_LIMIT,
        "UNAVAILABLE" => ErrorCode::API_UNAVAILABLE,
        "UNAUTHENTICATED" => ErrorCode::AUTHENTICATION_FAILED,
        _ => ErrorCode::SERVER_ERROR,
    };

    Some(QuickHorseError::new(code)
        .with_status(http_code)
        .with_details(message.to_string())
        .with_source("gemini".to_string())
        .with_retryable(status == "RESOURCE_EXHAUSTED" || status == "UNAVAILABLE"))
}

/// Parse Ollama error format
fn parse_ollama_error(json: &serde_json::Value, _body: &str) -> Option<QuickHorseError> {
    let message = json.get("error")
        .and_then(|e| e.as_str())
        .unwrap_or("未知错误");

    let code = if message.contains("model") || message.contains("not found") {
        ErrorCode::MODEL_NOT_FOUND
    } else if message.contains("connection") {
        ErrorCode::API_CONNECTION_ERROR
    } else if message.contains("timeout") {
        ErrorCode::TIMEOUT
    } else {
        ErrorCode::SERVER_ERROR
    };

    Some(QuickHorseError::new(code)
        .with_details(message.to_string())
        .with_source("ollama".to_string())
        .with_retryable(code == ErrorCode::API_CONNECTION_ERROR || code == ErrorCode::TIMEOUT))
}

/// Parse BaiLian/DashScope error format
fn parse_bailian_error(json: &serde_json::Value, _body: &str) -> Option<QuickHorseError> {
    // BaiLian format: {"code": "xxx", "message": "...", "request_id": "..."}
    let message = json.get("message")
        .and_then(|m| m.as_str())
        .unwrap_or("未知错误");

    let error_code = json.get("code")
        .and_then(|c| c.as_str())
        .unwrap_or("");

    let code = match error_code {
        "InvalidApiKey" | "ApiKeyNotFound" => ErrorCode::AUTHENTICATION_FAILED,
        "RateLimit" | "Throttling" => ErrorCode::RATE_LIMIT,
        "ModelNotFound" | "ModelNotExists" => ErrorCode::MODEL_NOT_FOUND,
        "InvalidParameter" => ErrorCode::INVALID_REQUEST,
        "BillingNotEnough" | "InsufficientQuota" => ErrorCode::BILLING_ERROR,
        "InternalError" | "ServiceUnavailable" => ErrorCode::SERVER_ERROR,
        "CodingPlanUnavailable" => ErrorCode::API_UNAVAILABLE,
        _ => ErrorCode::SERVER_ERROR,
    };

    // Special handling for Coding Plan error
    let final_message = if message.contains("Coding Plan") {
        format!("{}\n建议: 使用 /model 切换到非 Coding Plan 模型，或使用标准 chat/completions 端点", message)
    } else {
        message.to_string()
    };

    Some(QuickHorseError::new(code)
        .with_details(final_message)
        .with_source("bailian".to_string())
        .with_retryable(code == ErrorCode::RATE_LIMIT || code == ErrorCode::SERVER_ERROR))
}

/// Classify provider-specific error with context
pub fn classify_provider_error(provider: &str, status: u16, body: &str) -> QuickHorseError {
    // Try parsing provider-specific format first
    if let Some(error) = parse_api_error_body(provider, body) {
        return error.with_status(status);
    }

    // Fall back to generic HTTP status handling
    from_http_status_with_body(status, body)
}

/// Create provider connection error
pub fn provider_connection_error(provider: &str, host: &str) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::API_CONNECTION_ERROR)
        .with_message(format!("无法连接到 {} ({})", provider, host))
        .with_source(provider.to_string())
        .with_retryable(true)
}

/// Create provider unavailable error
pub fn provider_unavailable_error(provider: &str) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::API_UNAVAILABLE)
        .with_message(format!("{} 服务暂时不可用", provider))
        .with_source(provider.to_string())
        .with_retryable(true)
}

/// Create model not found error with suggestion
pub fn model_not_found_error(provider: &str, model: &str, suggestion: Option<&str>) -> QuickHorseError {
    let mut msg = format!("模型 {} 在 {} 上不存在", model, provider);
    if let Some(s) = suggestion {
        msg.push_str(&format!("\n  可用模型: {}", s));
    }

    QuickHorseError::new(ErrorCode::MODEL_NOT_FOUND)
        .with_message(msg)
        .with_source(provider.to_string())
        .with_details(format!("尝试模型: {}", model))
}

/// Create API key error with context
pub fn api_key_error(provider: &str, context: &str) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED)
        .with_message(format!("{} API 密钥无效", provider))
        .with_source(provider.to_string())
        .with_details(context.to_string())
}
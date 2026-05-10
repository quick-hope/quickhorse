//! API error - HTTP status code mapping

use super::{ErrorCode, ErrorCategory, QuickHorseError};
use super::types::ApiCategory;

/// Create error from HTTP status code
pub fn from_http_status(status: u16, body: Option<&str>) -> QuickHorseError {
    match status {
        // Authentication errors
        401 => QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED)
            .with_status(status)
            .with_retryable(false),

        403 => QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED)
            .with_message("访问被拒绝".to_string())
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(false),

        // Billing/payment errors
        402 => QuickHorseError::new(ErrorCode::BILLING_ERROR)
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(false),

        // Rate limiting
        429 => QuickHorseError::new(ErrorCode::RATE_LIMIT)
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(true),

        // Not found
        404 => QuickHorseError::new(ErrorCode::MODEL_NOT_FOUND)
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(false),

        // Invalid request
        400 => {
            // Check for specific error types in body
            if let Some(body) = body {
                if body.contains("context_length") || body.contains("token_limit") {
                    return QuickHorseError::new(ErrorCode::CONTEXT_LENGTH)
                        .with_status(status)
                        .with_details(body.to_string())
                        .with_retryable(false);
                }
                if body.contains("max_tokens") {
                    return QuickHorseError::new(ErrorCode::MAX_OUTPUT_TOKENS)
                        .with_status(status)
                        .with_details(body.to_string())
                        .with_retryable(false);
                }
                if body.contains("invalid_model") || body.contains("model_not_found") {
                    return QuickHorseError::new(ErrorCode::MODEL_NOT_FOUND)
                        .with_status(status)
                        .with_details(body.to_string())
                        .with_retryable(false);
                }
            }

            QuickHorseError::new(ErrorCode::INVALID_REQUEST)
                .with_status(status)
                .with_details(body.unwrap_or("").to_string())
                .with_retryable(false)
        },

        // Request too large
        413 => QuickHorseError::new(ErrorCode::CONTEXT_LENGTH)
            .with_message("请求过大".to_string())
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(false),

        // Server overload
        529 => QuickHorseError::new(ErrorCode::RATE_LIMIT)
            .with_message("服务器过载".to_string())
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(true),

        // Server errors (retryable)
        500 | 502 | 503 => QuickHorseError::new(ErrorCode::SERVER_ERROR)
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(true),

        // Gateway timeout
        504 => QuickHorseError::new(ErrorCode::TIMEOUT)
            .with_message("网关超时".to_string())
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(true),

        // Unknown status
        _ => QuickHorseError::new(ErrorCode::SERVER_ERROR)
            .with_message(format!("未知 API 错误 (状态码: {})", status))
            .with_status(status)
            .with_details(body.unwrap_or("").to_string())
            .with_retryable(status >= 500),
    }
}

/// Create error from HTTP status with parsed body
pub fn from_http_status_with_body(status: u16, body: &str) -> QuickHorseError {
    let mut error = from_http_status(status, Some(body));

    // Try to extract message from JSON body
    if let Ok(json) = serde_json::from_str::<serde_json::Value>(body) {
        // OpenAI/Anthropic format: {"error": {"message": "..."}}
        if let Some(error_obj) = json.get("error") {
            if let Some(msg) = error_obj.get("message").and_then(|m| m.as_str()) {
                error = error.with_details(msg.to_string());
            }
            // Check error type
            if let Some(err_type) = error_obj.get("type").and_then(|t| t.as_str()) {
                match err_type {
                    "authentication_error" => {
                        error.code = ErrorCode::AUTHENTICATION_FAILED;
                        error.category = ErrorCategory::Api(ApiCategory::AuthenticationFailed);
                    }
                    "rate_limit_error" => {
                        error.code = ErrorCode::RATE_LIMIT;
                        error.category = ErrorCategory::Api(ApiCategory::RateLimit);
                    }
                    "invalid_request_error" => {
                        error.code = ErrorCode::INVALID_REQUEST;
                        error.category = ErrorCategory::Api(ApiCategory::InvalidRequest);
                    }
                    "overloaded_error" => {
                        error.code = ErrorCode::SERVER_ERROR;
                        error.retryable = true;
                    }
                    "context_length_exceeded" => {
                        error.code = ErrorCode::CONTEXT_LENGTH;
                        error.category = ErrorCategory::Api(ApiCategory::ContextLength);
                    }
                    _ => {}
                }
            }
        }

        // Gemini format: {"error": {"code": 400, "message": "..."}}
        if let Some(error_obj) = json.get("error") {
            if let Some(code) = error_obj.get("code").and_then(|c| c.as_u64()) {
                error = error.with_status(code as u16);
            }
            if let Some(msg) = error_obj.get("message").and_then(|m| m.as_str()) {
                error = error.with_details(msg.to_string());
            }
            if let Some(status_str) = error_obj.get("status").and_then(|s| s.as_str()) {
                match status_str {
                    "INVALID_ARGUMENT" => {
                        error.code = ErrorCode::INVALID_REQUEST;
                    }
                    "NOT_FOUND" => {
                        error.code = ErrorCode::MODEL_NOT_FOUND;
                    }
                    "PERMISSION_DENIED" => {
                        error.code = ErrorCode::PERMISSION_DENIED;
                    }
                    "RESOURCE_EXHAUSTED" => {
                        error.code = ErrorCode::RATE_LIMIT;
                        error.retryable = true;
                    }
                    "UNAVAILABLE" => {
                        error.code = ErrorCode::API_UNAVAILABLE;
                        error.retryable = true;
                    }
                    _ => {}
                }
            }
        }
    }

    error
}

/// Check if error is retryable based on status code
#[allow(dead_code)]
pub fn is_retryable_status(status: u16) -> bool {
    matches!(status, 429 | 500 | 502 | 503 | 504 | 529)
}

/// Get retry hint based on status code
#[allow(dead_code)]
pub fn get_retry_hint(status: u16) -> String {
    match status {
        429 => "等待几秒后重试",
        500 | 502 | 503 => "稍后重试，如持续出现请切换 Provider",
        504 => "请求超时，稍后重试",
        529 => "服务器过载，稍后重试",
        _ => "请检查配置",
    }.to_string()
}

/// Create timeout error with elapsed time
pub fn timeout_error(elapsed_ms: u64) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::TIMEOUT)
        .with_message(format!("请求超时 ({}ms)", elapsed_ms))
        .with_retryable(true)
}

/// Create streaming error
pub fn streaming_error(reason: &str) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::STREAMING_ERROR)
        .with_details(reason.to_string())
        .with_retryable(true)
}

/// Create context length exceeded error
pub fn context_length_error(actual: u64, limit: u64) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::CONTEXT_LENGTH)
        .with_message(format!("上下文超限: {} tokens > {} 限制", actual, limit))
        .with_retryable(false)
}
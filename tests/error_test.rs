//! Error module unit tests

use quickhorse::error::{ErrorCode, QuickHorseError, from_http_status, classify_io_error};

/// Test error code formatting
#[test]
fn test_error_code_formatting() {
    let code = ErrorCode::AUTHENTICATION_FAILED;
    assert_eq!(code.code(), "E001");
    assert_eq!(code.to_string(), "E001");

    let code = ErrorCode::RATE_LIMIT;
    assert_eq!(code.code(), "E003");

    let code = ErrorCode::FILE_NOT_FOUND;
    assert_eq!(code.code(), "E104");
}

/// Test QuickHorseError creation
#[test]
fn test_error_creation() {
    let err = QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED);
    assert_eq!(err.code, ErrorCode::AUTHENTICATION_FAILED);
    assert!(err.message.contains("认证失败"));
    assert!(err.recovery_hint.contains("login"));
    assert!(!err.retryable);
}

/// Test QuickHorseError with details
#[test]
fn test_error_with_details() {
    let err = QuickHorseError::new(ErrorCode::RATE_LIMIT)
        .with_details("Request rejected after 5 retries".to_string())
        .with_source("openai".to_string());

    assert!(err.details.is_some());
    assert!(err.source.is_some());
    assert!(err.retryable);
}

/// Test HTTP status code mapping
#[test]
fn test_http_status_mapping() {
    let err = from_http_status(401, None);
    assert_eq!(err.code, ErrorCode::AUTHENTICATION_FAILED);
    assert!(!err.retryable);

    let err = from_http_status(429, Some("Rate limit exceeded"));
    assert_eq!(err.code, ErrorCode::RATE_LIMIT);
    assert!(err.retryable);

    let err = from_http_status(500, None);
    assert_eq!(err.code, ErrorCode::SERVER_ERROR);
    assert!(err.retryable);

    let err = from_http_status(404, None);
    assert_eq!(err.code, ErrorCode::MODEL_NOT_FOUND);
    assert!(!err.retryable);
}

/// Test I/O error classification
#[test]
fn test_io_error_classification() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
    let err = classify_io_error(&io_err);
    assert_eq!(err.code, ErrorCode::FILE_NOT_FOUND);

    let io_err = std::io::Error::new(std::io::ErrorKind::PermissionDenied, "Permission denied");
    let err = classify_io_error(&io_err);
    assert_eq!(err.code, ErrorCode::PERMISSION_DENIED);

    let io_err = std::io::Error::new(std::io::ErrorKind::TimedOut, "Timeout");
    let err = classify_io_error(&io_err);
    assert_eq!(err.code, ErrorCode::TIMEOUT);
    assert!(err.retryable);
}

/// Test user message formatting
#[test]
fn test_user_message_formatting() {
    let err = QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED);
    let msg = err.to_user_message();
    assert!(msg.starts_with("[E001]"));
    assert!(msg.contains("认证失败"));

    let full_msg = err.to_user_message_full();
    assert!(full_msg.contains("建议"));
}

/// Test TUI message formatting
#[test]
fn test_tui_message_formatting() {
    let err = QuickHorseError::new(ErrorCode::RATE_LIMIT)
        .with_details("Too many requests".to_string());
    let lines = err.to_tui_message();

    assert!(lines[0].contains("错误"));
    assert!(lines.iter().any(|l| l.contains("E003")));
    assert!(lines.iter().any(|l| l.contains("建议")));
}

/// Test retryable errors
#[test]
fn test_retryable_errors() {
    // Retryable errors
    assert!(QuickHorseError::new(ErrorCode::RATE_LIMIT).retryable);
    assert!(QuickHorseError::new(ErrorCode::SERVER_ERROR).retryable);
    assert!(QuickHorseError::new(ErrorCode::TIMEOUT).retryable);
    assert!(QuickHorseError::new(ErrorCode::STREAMING_ERROR).retryable);
    assert!(QuickHorseError::new(ErrorCode::NETWORK_ERROR).retryable);

    // Non-retryable errors
    assert!(!QuickHorseError::new(ErrorCode::AUTHENTICATION_FAILED).retryable);
    assert!(!QuickHorseError::new(ErrorCode::FILE_NOT_FOUND).retryable);
    assert!(!QuickHorseError::new(ErrorCode::INVALID_REQUEST).retryable);
}

/// Test context length error from HTTP 400
#[test]
fn test_context_length_from_body() {
    let err = from_http_status(400, Some("prompt is too long: context_length exceeded"));
    assert_eq!(err.code, ErrorCode::CONTEXT_LENGTH);
}
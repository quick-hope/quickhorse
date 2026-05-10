//! Error classification - Convert raw errors to QuickHorseError

use super::{ErrorCode, QuickHorseError};
use super::api::from_http_status;
use std::io;

/// Classify I/O error
pub fn classify_io_error(err: &io::Error) -> QuickHorseError {
    match err.kind() {
        io::ErrorKind::NotFound => QuickHorseError::new(ErrorCode::FILE_NOT_FOUND)
            .with_details(err.to_string()),

        io::ErrorKind::PermissionDenied => QuickHorseError::new(ErrorCode::PERMISSION_DENIED)
            .with_details(err.to_string()),

        io::ErrorKind::AlreadyExists => QuickHorseError::new(ErrorCode::FILE_WRITE_ERROR)
            .with_message("文件已存在".to_string())
            .with_details(err.to_string()),

        io::ErrorKind::InvalidInput => QuickHorseError::new(ErrorCode::TOOL_PARAM_INVALID)
            .with_details(err.to_string()),

        io::ErrorKind::InvalidData => QuickHorseError::new(ErrorCode::FILE_READ_ERROR)
            .with_message("数据格式无效".to_string())
            .with_details(err.to_string()),

        io::ErrorKind::TimedOut => QuickHorseError::new(ErrorCode::TIMEOUT)
            .with_details(err.to_string())
            .with_retryable(true),

        io::ErrorKind::UnexpectedEof => QuickHorseError::new(ErrorCode::STREAMING_ERROR)
            .with_message("流式传输意外中断".to_string())
            .with_details(err.to_string())
            .with_retryable(true),

        io::ErrorKind::ConnectionRefused => QuickHorseError::new(ErrorCode::NETWORK_ERROR)
            .with_message("连接被拒绝".to_string())
            .with_details(err.to_string())
            .with_retryable(true),

        io::ErrorKind::ConnectionReset => QuickHorseError::new(ErrorCode::NETWORK_ERROR)
            .with_message("连接被重置".to_string())
            .with_details(err.to_string())
            .with_retryable(true),

        io::ErrorKind::ConnectionAborted => QuickHorseError::new(ErrorCode::NETWORK_ERROR)
            .with_message("连接中断".to_string())
            .with_details(err.to_string())
            .with_retryable(true),

        io::ErrorKind::AddrInUse | io::ErrorKind::AddrNotAvailable => {
            QuickHorseError::new(ErrorCode::API_CONNECTION_ERROR)
                .with_details(err.to_string())
        }

        io::ErrorKind::WriteZero => QuickHorseError::new(ErrorCode::FILE_WRITE_ERROR)
            .with_message("写入失败".to_string())
            .with_details(err.to_string()),

        _ => QuickHorseError::new(ErrorCode::TOOL_EXECUTION_ERROR)
            .with_message("I/O 操作失败".to_string())
            .with_details(err.to_string()),
    }
}

/// Classify reqwest HTTP error
pub fn classify_reqwest_error(err: &reqwest::Error) -> QuickHorseError {
    // Timeout
    if err.is_timeout() {
        return QuickHorseError::new(ErrorCode::TIMEOUT)
            .with_details(err.to_string())
            .with_retryable(true);
    }

    // Connection errors
    if err.is_connect() {
        return QuickHorseError::new(ErrorCode::API_CONNECTION_ERROR)
            .with_message("无法连接到服务器".to_string())
            .with_details(err.to_string())
            .with_retryable(true);
    }

    // Body error
    if err.is_body() {
        return QuickHorseError::new(ErrorCode::STREAMING_ERROR)
            .with_message("响应体读取失败".to_string())
            .with_details(err.to_string())
            .with_retryable(true);
    }

    // Decode error
    if err.is_decode() {
        return QuickHorseError::new(ErrorCode::INVALID_REQUEST)
            .with_message("响应解码失败".to_string())
            .with_details(err.to_string());
    }

    // Status code
    if let Some(status) = err.status() {
        return from_http_status(status.as_u16(), None);
    }

    // Generic network error
    QuickHorseError::new(ErrorCode::NETWORK_ERROR)
        .with_details(err.to_string())
        .with_retryable(true)
}

/// Classify JSON parsing error
pub fn classify_json_error(err: &serde_json::Error) -> QuickHorseError {
    // Syntax error
    if err.is_syntax() {
        return QuickHorseError::new(ErrorCode::INVALID_REQUEST)
            .with_message("JSON 格式无效".to_string())
            .with_details(err.to_string());
    }

    // Data error
    if err.is_data() {
        return QuickHorseError::new(ErrorCode::INVALID_REQUEST)
            .with_message("JSON 数据类型错误".to_string())
            .with_details(err.to_string());
    }

    // EOF error
    if err.is_eof() {
        return QuickHorseError::new(ErrorCode::STREAMING_ERROR)
            .with_message("JSON 数据不完整".to_string())
            .with_details(err.to_string());
    }

    QuickHorseError::new(ErrorCode::INVALID_REQUEST)
        .with_message("JSON 解析失败".to_string())
        .with_details(err.to_string())
}

/// Check if errno code indicates file system is inaccessible
#[allow(dead_code)]
pub fn is_fs_inaccessible(code: &str) -> bool {
    matches!(
        code,
        "ENOENT" | "EACCES" | "EPERM" | "ENOTDIR" | "ELOOP"
    )
}

/// Create error for file not found with suggestion
#[allow(dead_code)]
pub fn file_not_found_with_suggestion(path: &str, cwd: &str, suggestion: Option<&str>) -> QuickHorseError {
    let mut msg = format!("文件不存在: {}", path);
    if let Some(s) = suggestion {
        msg.push_str(&format!("\n  可能是: {}", s));
    }
    msg.push_str(&format!("\n  当前目录: {}", cwd));

    QuickHorseError::new(ErrorCode::FILE_NOT_FOUND)
        .with_message(msg)
}

/// Create error for command failed
#[allow(dead_code)]
pub fn command_failed(cmd: &str, exit_code: i32, stderr: &str) -> QuickHorseError {
    let mut details = format!("命令: {}\n退出码: {}", cmd, exit_code);
    if !stderr.is_empty() {
        details.push_str(&format!("\n错误输出: {}", stderr));
    }

    QuickHorseError::new(ErrorCode::COMMAND_FAILED)
        .with_details(details)
}

/// Create error for tool parameter missing
#[allow(dead_code)]
pub fn missing_param(param_name: &str, tool_name: &str) -> QuickHorseError {
    QuickHorseError::new(ErrorCode::TOOL_PARAM_INVALID)
        .with_message(format!("缺少必需参数: {}", param_name))
        .with_source(tool_name.to_string())
}
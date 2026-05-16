//! Error types - ErrorCode, ErrorCategory, QuickHorseError

#![allow(dead_code)] // Future use: error types integration

use crate::error::{classify_io_error, classify_json_error, classify_reqwest_error};
use std::fmt;

/// Error code for identification
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCode(u16);

impl ErrorCode {
    // =====================
    // API Errors (E001-E020)
    // =====================
    pub const AUTHENTICATION_FAILED: Self = Self(1);
    pub const BILLING_ERROR: Self = Self(2);
    pub const RATE_LIMIT: Self = Self(3);
    pub const INVALID_REQUEST: Self = Self(4);
    pub const MODEL_NOT_FOUND: Self = Self(5);
    pub const SERVER_ERROR: Self = Self(6);
    pub const MAX_OUTPUT_TOKENS: Self = Self(7);
    pub const CONTEXT_LENGTH: Self = Self(8);
    pub const STREAMING_ERROR: Self = Self(9);
    pub const TIMEOUT: Self = Self(10);
    pub const NETWORK_ERROR: Self = Self(11);
    pub const API_CONNECTION_ERROR: Self = Self(12);
    pub const API_UNAVAILABLE: Self = Self(13);
    pub const INVALID_API_KEY: Self = Self(14);
    pub const ORGANIZATION_DISABLED: Self = Self(15);

    // =====================
    // Tool Errors (E101-E120)
    // =====================
    pub const COMMAND_FAILED: Self = Self(101);
    pub const COMMAND_TIMEOUT: Self = Self(102);
    pub const PERMISSION_DENIED: Self = Self(103);
    pub const FILE_NOT_FOUND: Self = Self(104);
    pub const FILE_READ_ERROR: Self = Self(105);
    pub const FILE_WRITE_ERROR: Self = Self(106);
    pub const DIRECTORY_NOT_FOUND: Self = Self(107);
    pub const GREP_NO_MATCH: Self = Self(108);
    pub const GLOB_INVALID: Self = Self(109);
    pub const WEB_FETCH_FAILED: Self = Self(110);
    pub const BINARY_FILE: Self = Self(111);
    pub const FILE_TOO_LARGE: Self = Self(112);
    pub const TOOL_PARAM_INVALID: Self = Self(113);
    pub const TOOL_EXECUTION_ERROR: Self = Self(114);

    // =====================
    // Config Errors (E201-E210)
    // =====================
    pub const CONFIG_NOT_FOUND: Self = Self(201);
    pub const CONFIG_PARSE_ERROR: Self = Self(202);
    pub const API_KEY_MISSING: Self = Self(203);
    pub const PROVIDER_NOT_CONFIGURED: Self = Self(204);
    pub const INVALID_BASE_URL: Self = Self(205);
    pub const SESSION_DIR_ERROR: Self = Self(206);

    // =====================
    // Session Errors (E301-E310)
    // =====================
    pub const SESSION_NOT_FOUND: Self = Self(301);
    pub const SESSION_LOAD_FAILED: Self = Self(302);
    pub const SESSION_SAVE_FAILED: Self = Self(303);
    pub const SESSION_CORRUPTED: Self = Self(304);

    // =====================
    // MCP Errors (E401-E410)
    // =====================
    pub const MCP_CONNECTION_FAILED: Self = Self(401);
    pub const MCP_TOOL_NOT_FOUND: Self = Self(402);
    pub const MCP_TIMEOUT: Self = Self(403);
    pub const MCP_PROTOCOL_ERROR: Self = Self(404);
    pub const MCP_SERVER_ERROR: Self = Self(405);

    /// Format error code as E{code}
    pub fn code(&self) -> String {
        format!("E{:03}", self.0)
    }

    /// Get numeric value
    pub fn value(&self) -> u16 {
        self.0
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:03}", self.0)
    }
}

/// Error category for classification
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorCategory {
    Api(ApiCategory),
    Tool(ToolCategory),
    Config(ConfigCategory),
    Session(SessionCategory),
    Mcp(McpCategory),
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApiCategory {
    AuthenticationFailed,
    BillingError,
    RateLimit,
    InvalidRequest,
    ModelNotFound,
    ServerError,
    MaxOutputTokens,
    ContextLength,
    StreamingError,
    Timeout,
    NetworkError,
    ConnectionError,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolCategory {
    CommandFailed,
    CommandTimeout,
    PermissionDenied,
    FileNotFound,
    FileReadError,
    FileWriteError,
    DirectoryNotFound,
    GrepNoMatch,
    GlobInvalid,
    WebFetchFailed,
    BinaryFile,
    FileTooLarge,
    ParamInvalid,
    ExecutionError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigCategory {
    ConfigNotFound,
    ConfigParseError,
    ApiKeyMissing,
    ProviderNotConfigured,
    InvalidBaseUrl,
    SessionDirError,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SessionCategory {
    SessionNotFound,
    SessionLoadFailed,
    SessionSaveFailed,
    SessionCorrupted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpCategory {
    ConnectionFailed,
    ToolNotFound,
    Timeout,
    ProtocolError,
    ServerError,
}

/// User-friendly error with context and recovery hints
#[derive(Debug, Clone)]
pub struct QuickHorseError {
    /// Error code for identification
    pub code: ErrorCode,
    /// Error category
    pub category: ErrorCategory,
    /// User-facing message
    pub message: String,
    /// Technical details (for verbose mode)
    pub details: Option<String>,
    /// Recovery suggestion
    pub recovery_hint: String,
    /// Source (e.g., provider name, tool name)
    pub source: Option<String>,
    /// HTTP status code (if applicable)
    pub http_status: Option<u16>,
    /// Whether this error is retryable
    pub retryable: bool,
}

impl QuickHorseError {
    /// Create error with just code (uses default message and hint)
    pub fn new(code: ErrorCode) -> Self {
        let (category, message, recovery_hint, retryable) = Self::defaults_for_code(code);
        Self {
            code,
            category,
            message,
            details: None,
            recovery_hint,
            source: None,
            http_status: None,
            retryable,
        }
    }

    /// Create unknown error
    pub fn unknown(message: String) -> Self {
        Self {
            code: ErrorCode::SERVER_ERROR,
            category: ErrorCategory::Unknown,
            message,
            details: None,
            recovery_hint: "请检查相关配置或稍后重试".to_string(),
            source: None,
            http_status: None,
            retryable: false,
        }
    }

    /// Set custom message
    pub fn with_message(mut self, message: String) -> Self {
        self.message = message;
        self
    }

    /// Set details (for verbose mode)
    pub fn with_details(mut self, details: String) -> Self {
        self.details = Some(details);
        self
    }

    /// Set source
    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    /// Set HTTP status
    pub fn with_status(mut self, status: u16) -> Self {
        self.http_status = Some(status);
        self
    }

    /// Mark as retryable
    pub fn with_retryable(mut self, retryable: bool) -> Self {
        self.retryable = retryable;
        self
    }

    /// Get default values for error code
    fn defaults_for_code(code: ErrorCode) -> (ErrorCategory, String, String, bool) {
        match code {
            // API Errors
            ErrorCode::AUTHENTICATION_FAILED => (
                ErrorCategory::Api(ApiCategory::AuthenticationFailed),
                "认证失败，API 密钥无效或已过期".to_string(),
                "使用 /login 重新登录或检查 API 密钥配置".to_string(),
                false,
            ),
            ErrorCode::BILLING_ERROR => (
                ErrorCategory::Api(ApiCategory::BillingError),
                "账户余额不足或计费问题".to_string(),
                "检查账户余额，访问 provider 控制台充值".to_string(),
                false,
            ),
            ErrorCode::RATE_LIMIT => (
                ErrorCategory::Api(ApiCategory::RateLimit),
                "请求频率超限".to_string(),
                "等待几秒后重试，或升级账户配额".to_string(),
                true,
            ),
            ErrorCode::INVALID_REQUEST => (
                ErrorCategory::Api(ApiCategory::InvalidRequest),
                "请求参数无效".to_string(),
                "检查输入内容，使用 /model 确认模型名称正确".to_string(),
                false,
            ),
            ErrorCode::MODEL_NOT_FOUND => (
                ErrorCategory::Api(ApiCategory::ModelNotFound),
                "指定的模型不存在".to_string(),
                "使用 /model 切换到可用模型".to_string(),
                false,
            ),
            ErrorCode::SERVER_ERROR => (
                ErrorCategory::Api(ApiCategory::ServerError),
                "服务端临时错误".to_string(),
                "稍后重试，如持续出现请切换 Provider".to_string(),
                true,
            ),
            ErrorCode::MAX_OUTPUT_TOKENS => (
                ErrorCategory::Api(ApiCategory::MaxOutputTokens),
                "输出达到最大长度限制".to_string(),
                "简化请求内容或分多次对话".to_string(),
                false,
            ),
            ErrorCode::CONTEXT_LENGTH => (
                ErrorCategory::Api(ApiCategory::ContextLength),
                "上下文长度超限".to_string(),
                "使用 /clear 清理历史消息".to_string(),
                false,
            ),
            ErrorCode::STREAMING_ERROR => (
                ErrorCategory::Api(ApiCategory::StreamingError),
                "流式传输中断".to_string(),
                "检查网络连接，重试请求".to_string(),
                true,
            ),
            ErrorCode::TIMEOUT => (
                ErrorCategory::Api(ApiCategory::Timeout),
                "请求超时".to_string(),
                "检查网络状况，稍后重试".to_string(),
                true,
            ),
            ErrorCode::NETWORK_ERROR => (
                ErrorCategory::Api(ApiCategory::NetworkError),
                "网络连接失败".to_string(),
                "检查网络/代理配置，确认 API 地址可达".to_string(),
                true,
            ),
            ErrorCode::API_CONNECTION_ERROR => (
                ErrorCategory::Api(ApiCategory::ConnectionError),
                "无法连接到 API 服务".to_string(),
                "检查 API 地址和网络连接".to_string(),
                true,
            ),
            ErrorCode::API_UNAVAILABLE => (
                ErrorCategory::Api(ApiCategory::Unavailable),
                "API 服务暂时不可用".to_string(),
                "稍后重试".to_string(),
                true,
            ),
            ErrorCode::INVALID_API_KEY => (
                ErrorCategory::Api(ApiCategory::AuthenticationFailed),
                "API 密钥格式无效".to_string(),
                "检查 API 密钥格式是否正确".to_string(),
                false,
            ),
            ErrorCode::ORGANIZATION_DISABLED => (
                ErrorCategory::Api(ApiCategory::AuthenticationFailed),
                "组织账户已被禁用".to_string(),
                "联系管理员或更换 API 密钥".to_string(),
                false,
            ),

            // Tool Errors
            ErrorCode::COMMAND_FAILED => (
                ErrorCategory::Tool(ToolCategory::CommandFailed),
                "命令执行失败".to_string(),
                "检查命令语法和权限".to_string(),
                false,
            ),
            ErrorCode::COMMAND_TIMEOUT => (
                ErrorCategory::Tool(ToolCategory::CommandTimeout),
                "命令执行超时".to_string(),
                "使用更短的超时参数".to_string(),
                false,
            ),
            ErrorCode::PERMISSION_DENIED => (
                ErrorCategory::Tool(ToolCategory::PermissionDenied),
                "权限不足".to_string(),
                "检查文件/目录权限".to_string(),
                false,
            ),
            ErrorCode::FILE_NOT_FOUND => (
                ErrorCategory::Tool(ToolCategory::FileNotFound),
                "文件不存在".to_string(),
                "使用 /status 检查当前工作目录".to_string(),
                false,
            ),
            ErrorCode::FILE_READ_ERROR => (
                ErrorCategory::Tool(ToolCategory::FileReadError),
                "文件读取失败".to_string(),
                "检查文件格式和权限".to_string(),
                false,
            ),
            ErrorCode::FILE_WRITE_ERROR => (
                ErrorCategory::Tool(ToolCategory::FileWriteError),
                "文件写入失败".to_string(),
                "检查磁盘空间和写权限".to_string(),
                false,
            ),
            ErrorCode::DIRECTORY_NOT_FOUND => (
                ErrorCategory::Tool(ToolCategory::DirectoryNotFound),
                "目录不存在".to_string(),
                "创建目录或检查路径".to_string(),
                false,
            ),
            ErrorCode::GREP_NO_MATCH => (
                ErrorCategory::Tool(ToolCategory::GrepNoMatch),
                "未找到匹配内容".to_string(),
                "调整搜索模式".to_string(),
                false,
            ),
            ErrorCode::GLOB_INVALID => (
                ErrorCategory::Tool(ToolCategory::GlobInvalid),
                "Glob 模式无效".to_string(),
                "检查 glob 语法".to_string(),
                false,
            ),
            ErrorCode::WEB_FETCH_FAILED => (
                ErrorCategory::Tool(ToolCategory::WebFetchFailed),
                "Web 获取失败".to_string(),
                "检查 URL 可达性".to_string(),
                true,
            ),
            ErrorCode::BINARY_FILE => (
                ErrorCategory::Tool(ToolCategory::BinaryFile),
                "无法读取二进制文件".to_string(),
                "使用其他工具处理二进制文件".to_string(),
                false,
            ),
            ErrorCode::FILE_TOO_LARGE => (
                ErrorCategory::Tool(ToolCategory::FileTooLarge),
                "文件过大".to_string(),
                "使用 offset/limit 参数分段读取".to_string(),
                false,
            ),
            ErrorCode::TOOL_PARAM_INVALID => (
                ErrorCategory::Tool(ToolCategory::ParamInvalid),
                "工具参数无效".to_string(),
                "检查参数格式".to_string(),
                false,
            ),
            ErrorCode::TOOL_EXECUTION_ERROR => (
                ErrorCategory::Tool(ToolCategory::ExecutionError),
                "工具执行出错".to_string(),
                "检查工具配置".to_string(),
                false,
            ),

            // Config Errors
            ErrorCode::CONFIG_NOT_FOUND => (
                ErrorCategory::Config(ConfigCategory::ConfigNotFound),
                "配置文件不存在".to_string(),
                "运行 quickhorse --setup 创建配置".to_string(),
                false,
            ),
            ErrorCode::CONFIG_PARSE_ERROR => (
                ErrorCategory::Config(ConfigCategory::ConfigParseError),
                "配置解析失败".to_string(),
                "检查 config.toml 格式，或重新运行 setup".to_string(),
                false,
            ),
            ErrorCode::API_KEY_MISSING => (
                ErrorCategory::Config(ConfigCategory::ApiKeyMissing),
                "API 密钥缺失".to_string(),
                "设置环境变量或运行 /login".to_string(),
                false,
            ),
            ErrorCode::PROVIDER_NOT_CONFIGURED => (
                ErrorCategory::Config(ConfigCategory::ProviderNotConfigured),
                "Provider 未配置".to_string(),
                "运行 /setup 配置 Provider".to_string(),
                false,
            ),
            ErrorCode::INVALID_BASE_URL => (
                ErrorCategory::Config(ConfigCategory::InvalidBaseUrl),
                "无效的 API 地址".to_string(),
                "检查 base_url 配置".to_string(),
                false,
            ),
            ErrorCode::SESSION_DIR_ERROR => (
                ErrorCategory::Config(ConfigCategory::SessionDirError),
                "Session 目录错误".to_string(),
                "检查 ~/.quickhorse/sessions 目录权限".to_string(),
                false,
            ),

            // Session Errors
            ErrorCode::SESSION_NOT_FOUND => (
                ErrorCategory::Session(SessionCategory::SessionNotFound),
                "会话不存在".to_string(),
                "使用 /session 列出可用会话".to_string(),
                false,
            ),
            ErrorCode::SESSION_LOAD_FAILED => (
                ErrorCategory::Session(SessionCategory::SessionLoadFailed),
                "会话加载失败".to_string(),
                "会话文件可能损坏，创建新会话".to_string(),
                false,
            ),
            ErrorCode::SESSION_SAVE_FAILED => (
                ErrorCategory::Session(SessionCategory::SessionSaveFailed),
                "会话保存失败".to_string(),
                "检查磁盘空间和写权限".to_string(),
                false,
            ),
            ErrorCode::SESSION_CORRUPTED => (
                ErrorCategory::Session(SessionCategory::SessionCorrupted),
                "会话数据损坏".to_string(),
                "创建新会话".to_string(),
                false,
            ),

            // MCP Errors
            ErrorCode::MCP_CONNECTION_FAILED => (
                ErrorCategory::Mcp(McpCategory::ConnectionFailed),
                "MCP 服务器连接失败".to_string(),
                "检查 MCP 配置和服务器状态".to_string(),
                true,
            ),
            ErrorCode::MCP_TOOL_NOT_FOUND => (
                ErrorCategory::Mcp(McpCategory::ToolNotFound),
                "MCP 工具不存在".to_string(),
                "检查工具名称和 MCP 配置".to_string(),
                false,
            ),
            ErrorCode::MCP_TIMEOUT => (
                ErrorCategory::Mcp(McpCategory::Timeout),
                "MCP 请求超时".to_string(),
                "检查 MCP 服务器响应时间".to_string(),
                true,
            ),
            ErrorCode::MCP_PROTOCOL_ERROR => (
                ErrorCategory::Mcp(McpCategory::ProtocolError),
                "MCP 协议错误".to_string(),
                "检查 MCP 版本兼容性".to_string(),
                false,
            ),
            ErrorCode::MCP_SERVER_ERROR => (
                ErrorCategory::Mcp(McpCategory::ServerError),
                "MCP 服务器错误".to_string(),
                "检查 MCP 服务器日志".to_string(),
                true,
            ),

            _ => (
                ErrorCategory::Unknown,
                "未知错误".to_string(),
                "请检查相关配置或稍后重试".to_string(),
                false,
            ),
        }
    }

    /// Format for user display (simple version)
    pub fn to_user_message(&self) -> String {
        format!("[{}] {}", self.code, self.message)
    }

    /// Format for user display (full version with hint)
    pub fn to_user_message_full(&self) -> String {
        let mut msg = format!("[{}] {}", self.code, self.message);

        if let Some(details) = &self.details {
            msg.push_str(&format!("\n  详情: {}", details));
        }

        msg.push_str(&format!("\n  建议: {}", self.recovery_hint));

        msg
    }

    /// Format for TUI display (boxed style)
    pub fn to_tui_message(&self) -> Vec<String> {
        let mut lines = vec![
            "┌─ 错误 ────────────────────────────────".to_string(),
            format!("│ {}", self.message),
        ];

        if let Some(details) = &self.details {
            // Truncate long details
            let truncated = if details.len() > 100 {
                format!("{}...", &details[..100])
            } else {
                details.clone()
            };
            lines.push(format!("│ 详情: {}", truncated));
        }

        lines.push(format!("│ 代码: {}", self.code));
        lines.push(format!("│ 建议: {}", self.recovery_hint));

        if self.retryable {
            lines.push("│ 重试: 可重试".to_string());
        }

        lines.push("└───────────────────────────────────────".to_string());

        lines
    }

    /// Format for verbose/debug mode
    pub fn to_debug_message(&self) -> String {
        let mut msg = format!(
            "[{}] {} (category: {:?})",
            self.code, self.message, self.category
        );

        if let Some(source) = &self.source {
            msg.push_str(&format!("\n  来源: {}", source));
        }

        if let Some(status) = self.http_status {
            msg.push_str(&format!("\n  HTTP状态: {}", status));
        }

        if let Some(details) = &self.details {
            msg.push_str(&format!("\n  详情: {}", details));
        }

        msg.push_str(&format!("\n  建议: {}", self.recovery_hint));
        msg.push_str(&format!("\n  可重试: {}", self.retryable));

        msg
    }
}

impl fmt::Display for QuickHorseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_user_message_full())
    }
}

impl std::error::Error for QuickHorseError {}

/// Convert from std::io::Error
impl From<std::io::Error> for QuickHorseError {
    fn from(err: std::io::Error) -> Self {
        classify_io_error(&err)
    }
}

/// Convert from serde_json::Error
impl From<serde_json::Error> for QuickHorseError {
    fn from(err: serde_json::Error) -> Self {
        classify_json_error(&err)
    }
}

/// Convert from reqwest::Error
impl From<reqwest::Error> for QuickHorseError {
    fn from(err: reqwest::Error) -> Self {
        classify_reqwest_error(&err)
    }
}
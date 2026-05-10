//! Log types - Type definitions for logging system
//!
//! Reference: OpenClaude src/types/logs.ts

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Log level for diagnostic entries
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Error => write!(f, "ERROR"),
        }
    }
}

/// Diagnostic log entry (Reference: OpenClaude diagLogs.ts)
///
/// IMPORTANT: This must not contain any PII (file paths, project names, etc.)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiagnosticLogEntry {
    /// ISO 8601 timestamp
    pub timestamp: String,
    /// Log level
    pub level: LogLevel,
    /// Event name (e.g., "started", "mcp_connected", "tool_executed")
    pub event: String,
    /// Additional data (no PII)
    pub data: serde_json::Value,
}

impl DiagnosticLogEntry {
    /// Create a new diagnostic log entry
    pub fn new(level: LogLevel, event: String, data: serde_json::Value) -> Self {
        Self {
            timestamp: Utc::now().to_rfc3339(),
            level,
            event,
            data,
        }
    }

    /// Create a debug entry
    pub fn debug(event: String, data: serde_json::Value) -> Self {
        Self::new(LogLevel::Debug, event, data)
    }

    /// Create an info entry
    pub fn info(event: String, data: serde_json::Value) -> Self {
        Self::new(LogLevel::Info, event, data)
    }

    /// Create a warn entry
    pub fn warn(event: String, data: serde_json::Value) -> Self {
        Self::new(LogLevel::Warn, event, data)
    }

    /// Create an error entry
    pub fn error(event: String, data: serde_json::Value) -> Self {
        Self::new(LogLevel::Error, event, data)
    }

    /// Convert to JSON Lines format
    pub fn to_jsonl(&self) -> String {
        serde_json::to_string(self).unwrap_or_default() + "\n"
    }
}

/// Serialized message for session logs (Reference: OpenClaude logs.ts:SerializedMessage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedMessage {
    /// Message ID (UUID)
    pub id: String,
    /// Message role (user, assistant, system)
    pub role: String,
    /// Message content
    pub content: String,
    /// Timestamp (ISO 8601)
    pub timestamp: String,
    /// Session ID
    pub session_id: String,
    /// Current working directory
    pub cwd: Option<String>,
    /// Git branch
    pub git_branch: Option<String>,
    /// Version string
    pub version: String,
}

impl SerializedMessage {
    /// Create a new serialized message
    pub fn new(id: String, role: String, content: String, session_id: String) -> Self {
        Self {
            id,
            role,
            content,
            timestamp: Utc::now().to_rfc3339(),
            session_id,
            cwd: None,
            git_branch: None,
            version: env!("CARGO_PKG_VERSION").to_string(),
        }
    }

    /// Set working directory
    pub fn with_cwd(mut self, cwd: String) -> Self {
        self.cwd = Some(cwd);
        self
    }

    /// Set git branch
    pub fn with_git_branch(mut self, branch: String) -> Self {
        self.git_branch = Some(branch);
        self
    }
}

/// Log option for session metadata (Reference: OpenClaude logs.ts:LogOption)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogOption {
    /// Date identifier
    pub date: String,
    /// Message list
    pub messages: Vec<SerializedMessage>,
    /// Full file path
    #[serde(skip_serializing_if = "Option::is_none")]
    pub full_path: Option<String>,
    /// Index value
    pub value: usize,
    /// Created timestamp
    pub created: DateTime<Utc>,
    /// Modified timestamp
    pub modified: DateTime<Utc>,
    /// First user prompt (truncated to 50 chars)
    pub first_prompt: String,
    /// Message count
    pub message_count: usize,
    /// File size in bytes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_size: Option<u64>,
    /// Is sidechain
    pub is_sidechain: bool,
    /// Session ID
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    /// Custom title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_title: Option<String>,
    /// AI-generated title
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ai_title: Option<String>,
    /// Session summary
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    /// Git branch
    #[serde(skip_serializing_if = "Option::is_none")]
    pub git_branch: Option<String>,
}

impl LogOption {
    /// Create a new log option
    pub fn new(date: String, messages: Vec<SerializedMessage>) -> Self {
        let message_count = messages.len();
        let first_prompt = messages
            .first()
            .filter(|m| m.role == "user")
            .map(|m| {
                let text = m.content.lines().next().unwrap_or("");
                if text.len() > 50 {
                    format!("{}…", &text[..50])
                } else {
                    text.to_string()
                }
            })
            .unwrap_or_else(|| "No prompt".to_string());

        Self {
            date,
            messages,
            full_path: None,
            value: 0,
            created: Utc::now(),
            modified: Utc::now(),
            first_prompt,
            message_count,
            file_size: None,
            is_sidechain: false,
            session_id: None,
            custom_title: None,
            ai_title: None,
            summary: None,
            git_branch: None,
        }
    }

    /// Get display title (Reference: OpenClaude getLogDisplayTitle)
    pub fn display_title(&self) -> String {
        // Priority: custom_title > ai_title > summary > first_prompt > session_id
        self.custom_title
            .clone()
            .or_else(|| self.ai_title.clone())
            .or_else(|| self.summary.clone())
            .or_else(|| {
                if !self.first_prompt.is_empty() && !self.first_prompt.starts_with('<') {
                    Some(self.first_prompt.clone())
                } else {
                    None
                }
            })
            .or_else(|| self.session_id.clone())
            .unwrap_or_else(|| "".to_string())
    }
}

/// Sort logs by modified date (newest first)
pub fn sort_logs(logs: &mut [LogOption]) {
    logs.sort_by(|a, b| {
        // Sort by modified (newest first)
        b.modified.cmp(&a.modified)
            .then_with(|| b.created.cmp(&a.created))
    });
}

/// Convert date to filename format (Reference: OpenClaude dateToFilename)
pub fn date_to_filename(date: &DateTime<Utc>) -> String {
    date.to_rfc3339()
        .replace(':', "-")
        .replace('.', "-")
}
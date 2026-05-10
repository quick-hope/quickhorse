//! Log sink - Sink abstraction for logging
//!
//! Reference: OpenClaude src/utils/log.ts ErrorLogSink

use std::path::PathBuf;
use std::io::Write;
use crate::log::types::DiagnosticLogEntry;
use crate::log::config::LogConfig;

/// Log sink trait (Reference: OpenClaude ErrorLogSink)
pub trait LogSink: Send + Sync {
    /// Log an error
    fn log_error(&self, error: &str);

    /// Log MCP error
    fn log_mcp_error(&self, server_name: &str, error: &str);

    /// Log MCP debug info
    fn log_mcp_debug(&self, server_name: &str, message: &str);

    /// Get errors log path
    fn get_errors_path(&self) -> PathBuf;

    /// Get MCP logs path for a specific server
    fn get_mcp_logs_path(&self, server_name: &str) -> PathBuf;
}

/// File-based log sink implementation
pub struct FileLogSink {
    log_dir: PathBuf,
}

impl FileLogSink {
    /// Create a new file log sink
    pub fn new(log_dir: PathBuf) -> Self {
        // Ensure directories exist
        std::fs::create_dir_all(&log_dir).ok();
        std::fs::create_dir_all(log_dir.join("mcp")).ok();

        Self { log_dir }
    }

    /// Create from log config
    pub fn from_config(config: &LogConfig) -> Self {
        Self::new(config.log_dir.clone())
    }

    /// Append entry to log file
    fn append_to_log(&self, path: &PathBuf, entry: &DiagnosticLogEntry) {
        let line = entry.to_jsonl();

        // Try to append directly
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            if file.write_all(line.as_bytes()).is_ok() {
                return;
            }
        }

        // If failed, try creating parent directory first
        if let Some(parent) = path.parent() {
            if std::fs::create_dir_all(parent).is_ok() {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(path)
                {
                    file.write_all(line.as_bytes()).ok();
                }
            }
        }
    }
}

impl LogSink for FileLogSink {
    fn log_error(&self, error: &str) {
        let entry = DiagnosticLogEntry::error(
            "error".to_string(),
            serde_json::json!({
                "message": error,
            }),
        );
        self.append_to_log(&self.get_errors_path(), &entry);
    }

    fn log_mcp_error(&self, server_name: &str, error: &str) {
        let entry = DiagnosticLogEntry::error(
            "mcp_error".to_string(),
            serde_json::json!({
                "server": server_name,
                "error": error,
            }),
        );
        self.append_to_log(&self.get_mcp_logs_path(server_name), &entry);
    }

    fn log_mcp_debug(&self, server_name: &str, message: &str) {
        let entry = DiagnosticLogEntry::debug(
            "mcp_debug".to_string(),
            serde_json::json!({
                "server": server_name,
                "message": message,
            }),
        );
        self.append_to_log(&self.get_mcp_logs_path(server_name), &entry);
    }

    fn get_errors_path(&self) -> PathBuf {
        self.log_dir.join("errors.jsonl")
    }

    fn get_mcp_logs_path(&self, server_name: &str) -> PathBuf {
        self.log_dir.join("mcp").join(format!("{}.jsonl", server_name))
    }
}

/// In-memory log sink (for testing)
pub struct InMemoryLogSink {
    errors: std::sync::RwLock<Vec<String>>,
    mcp_errors: std::sync::RwLock<Vec<(String, String)>>,
    mcp_debug: std::sync::RwLock<Vec<(String, String)>>,
}

impl InMemoryLogSink {
    /// Create a new in-memory log sink
    pub fn new() -> Self {
        Self {
            errors: std::sync::RwLock::new(Vec::new()),
            mcp_errors: std::sync::RwLock::new(Vec::new()),
            mcp_debug: std::sync::RwLock::new(Vec::new()),
        }
    }

    /// Get all errors
    pub fn get_errors(&self) -> Vec<String> {
        self.errors.read().unwrap().clone()
    }

    /// Get all MCP errors
    pub fn get_mcp_errors(&self) -> Vec<(String, String)> {
        self.mcp_errors.read().unwrap().clone()
    }

    /// Get all MCP debug messages
    pub fn get_mcp_debug(&self) -> Vec<(String, String)> {
        self.mcp_debug.read().unwrap().clone()
    }

    /// Clear all logs
    pub fn clear(&self) {
        self.errors.write().unwrap().clear();
        self.mcp_errors.write().unwrap().clear();
        self.mcp_debug.write().unwrap().clear();
    }
}

impl Default for InMemoryLogSink {
    fn default() -> Self {
        Self::new()
    }
}

impl LogSink for InMemoryLogSink {
    fn log_error(&self, error: &str) {
        self.errors.write().unwrap().push(error.to_string());
    }

    fn log_mcp_error(&self, server_name: &str, error: &str) {
        self.mcp_errors.write().unwrap().push((server_name.to_string(), error.to_string()));
    }

    fn log_mcp_debug(&self, server_name: &str, message: &str) {
        self.mcp_debug.write().unwrap().push((server_name.to_string(), message.to_string()));
    }

    fn get_errors_path(&self) -> PathBuf {
        PathBuf::from("memory://errors")
    }

    fn get_mcp_logs_path(&self, server_name: &str) -> PathBuf {
        PathBuf::from(format!("memory://mcp/{}", server_name))
    }
}
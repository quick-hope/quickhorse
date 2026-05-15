//! Log module - Logging system for QuickHorse
//!
//! Provides tracing-based logging with file and terminal output.
//! Reference: OpenClaude src/types/logs.ts, src/utils/log.ts, src/utils/diagLogs.ts

#![allow(dead_code)] // Future use: full logging integration

mod types;
mod config;
mod sink;
mod diagnostic;
mod init;

pub use types::{LogLevel, DiagnosticLogEntry, SerializedMessage, LogOption};
pub use config::LogConfig;
pub use sink::{LogSink, FileLogSink, InMemoryLogSink};
pub use diagnostic::{
    log_for_diagnostics,
    log_diagnostic_info,
    log_diagnostic_debug,
    log_diagnostic_warn,
    log_diagnostic_error,
    with_diagnostics_timing,
    DiagnosticTimer,
    log_app_started,
    log_mcp_connected,
    log_tool_executed,
    log_api_request,
    log_api_response,
};
pub use init::{
    init_logging,
    init_default,
    init_verbose,
    init_debug,
    init_from_cli,
};

use std::sync::{Arc, RwLock};

/// In-memory error log for recent errors
const MAX_IN_MEMORY_ERRORS: usize = 100;

/// Global in-memory error log
static IN_MEMORY_ERROR_LOG: RwLock<Vec<ErrorEntry>> = RwLock::new(Vec::new());

/// Error entry for in-memory log
#[derive(Debug, Clone)]
pub struct ErrorEntry {
    pub error: String,
    pub timestamp: String,
}

/// Add error to in-memory log
pub fn add_to_in_memory_error_log(error: String) {
    let timestamp = chrono::Utc::now().to_rfc3339();

    if let Ok(mut log) = IN_MEMORY_ERROR_LOG.write() {
        if log.len() >= MAX_IN_MEMORY_ERRORS {
            log.remove(0); // Remove oldest
        }
        log.push(ErrorEntry { error, timestamp });
    }
}

/// Get in-memory errors
pub fn get_in_memory_errors() -> Vec<ErrorEntry> {
    if let Ok(log) = IN_MEMORY_ERROR_LOG.read() {
        log.clone()
    } else {
        Vec::new()
    }
}

/// Clear in-memory errors (for testing)
pub fn clear_in_memory_errors() {
    if let Ok(mut log) = IN_MEMORY_ERROR_LOG.write() {
        log.clear();
    }
}

/// Global log sink
static LOG_SINK: RwLock<Option<Arc<dyn LogSink>>> = RwLock::new(None);

/// Attach log sink
pub fn attach_log_sink(sink: Arc<dyn LogSink>) {
    if let Ok(mut global_sink) = LOG_SINK.write() {
        if global_sink.is_none() {
            *global_sink = Some(sink);
        }
    }
}

/// Log an error to the sink
pub fn log_error(error: &str) {
    // Always add to in-memory log
    add_to_in_memory_error_log(error.to_string());

    // Try to send to sink
    if let Ok(sink_ref) = LOG_SINK.read() {
        if let Some(sink) = sink_ref.as_ref() {
            sink.log_error(error);
        }
    }
}

/// Log MCP error
pub fn log_mcp_error(server_name: &str, error: &str) {
    if let Ok(sink_ref) = LOG_SINK.read() {
        if let Some(sink) = sink_ref.as_ref() {
            sink.log_mcp_error(server_name, error);
        }
    }
}

/// Log MCP debug info
pub fn log_mcp_debug(server_name: &str, message: &str) {
    if let Ok(sink_ref) = LOG_SINK.read() {
        if let Some(sink) = sink_ref.as_ref() {
            sink.log_mcp_debug(server_name, message);
        }
    }
}
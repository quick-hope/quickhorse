//! Diagnostic logging helpers
//!
//! Provides utilities for logging diagnostic events without PII.
//! Reference: OpenClaude src/utils/diagLogs.ts

use crate::log::types::{DiagnosticLogEntry, LogLevel};
use std::time::Instant;

/// Log an event for diagnostics (no PII)
pub fn log_for_diagnostics(level: LogLevel, event: String, data: serde_json::Value) {
    let entry = DiagnosticLogEntry::new(level, event, data);

    // Log to tracing
    match level {
        LogLevel::Debug => tracing::debug!(
            event = %entry.event,
            data = ?entry.data,
            "Diagnostic event"
        ),
        LogLevel::Info => tracing::info!(
            event = %entry.event,
            data = ?entry.data,
            "Diagnostic event"
        ),
        LogLevel::Warn => tracing::warn!(
            event = %entry.event,
            data = ?entry.data,
            "Diagnostic event"
        ),
        LogLevel::Error => tracing::error!(
            event = %entry.event,
            data = ?entry.data,
            "Diagnostic event"
        ),
    }
}

/// Log a diagnostic info event
pub fn log_diagnostic_info(event: String, data: serde_json::Value) {
    log_for_diagnostics(LogLevel::Info, event, data);
}

/// Log a diagnostic debug event
pub fn log_diagnostic_debug(event: String, data: serde_json::Value) {
    log_for_diagnostics(LogLevel::Debug, event, data);
}

/// Log a diagnostic warning event
pub fn log_diagnostic_warn(event: String, data: serde_json::Value) {
    log_for_diagnostics(LogLevel::Warn, event, data);
}

/// Log a diagnostic error event
pub fn log_diagnostic_error(event: String, data: serde_json::Value) {
    log_for_diagnostics(LogLevel::Error, event, data);
}

/// Time a diagnostic operation and log the duration
pub struct DiagnosticTimer {
    event: String,
    start: Instant,
}

impl DiagnosticTimer {
    /// Start timing an operation
    pub fn new(event: String) -> Self {
        Self {
            event,
            start: Instant::now(),
        }
    }

    /// Finish timing and log the duration
    pub fn finish(self, data: serde_json::Value) {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        let merged_data = if data.is_object() {
            let mut obj = data.as_object().cloned().unwrap_or_default();
            obj.insert("duration_ms".to_string(), serde_json::json!(duration_ms));
            serde_json::Value::Object(obj)
        } else {
            serde_json::json!({
                "data": data,
                "duration_ms": duration_ms
            })
        };

        log_diagnostic_info(self.event, merged_data);
    }

    /// Finish timing with error
    pub fn finish_with_error(self, error: String) {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        log_diagnostic_error(self.event, serde_json::json!({
            "error": error,
            "duration_ms": duration_ms
        }));
    }
}

/// Create a diagnostic timer for an operation
pub fn with_diagnostics_timing(event: String) -> DiagnosticTimer {
    DiagnosticTimer::new(event)
}

/// Log application started event
pub fn log_app_started(version: &str, provider: &str, model: &str) {
    log_diagnostic_info(
        "app_started".to_string(),
        serde_json::json!({
            "version": version,
            "provider": provider,
            "model": model,
        }),
    );
}

/// Log MCP server connected event
pub fn log_mcp_connected(server_name: &str, tools_count: usize) {
    log_diagnostic_info(
        "mcp_connected".to_string(),
        serde_json::json!({
            "server": server_name,
            "tools_count": tools_count,
        }),
    );
}

/// Log tool executed event
pub fn log_tool_executed(tool_name: &str, success: bool, duration_ms: u64) {
    let level = if success { LogLevel::Info } else { LogLevel::Warn };
    log_for_diagnostics(
        level,
        "tool_executed".to_string(),
        serde_json::json!({
            "tool": tool_name,
            "success": success,
            "duration_ms": duration_ms,
        }),
    );
}

/// Log API request event
pub fn log_api_request(provider: &str, method: &str) {
    log_diagnostic_debug(
        "api_request_started".to_string(),
        serde_json::json!({
            "provider": provider,
            "method": method,
        }),
    );
}

/// Log API response event
pub fn log_api_response(provider: &str, status_code: u16, duration_ms: u64) {
    let level = if status_code >= 400 { LogLevel::Warn } else { LogLevel::Debug };
    log_for_diagnostics(
        level,
        "api_response".to_string(),
        serde_json::json!({
            "provider": provider,
            "status_code": status_code,
            "duration_ms": duration_ms,
        }),
    );
}
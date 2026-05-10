//! Log configuration

use std::path::PathBuf;

/// Log configuration options
#[derive(Debug, Clone)]
pub struct LogConfig {
    /// Enable verbose mode (info level)
    pub verbose: bool,
    /// Enable debug mode (debug level)
    pub debug: bool,
    /// Log directory path
    pub log_dir: PathBuf,
    /// Write to file
    pub write_to_file: bool,
    /// Write to terminal
    pub write_to_terminal: bool,
    /// Max in-memory errors
    pub max_in_memory_errors: usize,
}

impl Default for LogConfig {
    fn default() -> Self {
        Self {
            verbose: false,
            debug: false,
            log_dir: PathBuf::from(".quickhorse/logs"),
            write_to_file: true,
            write_to_terminal: false,
            max_in_memory_errors: 100,
        }
    }
}

impl LogConfig {
    /// Create with verbose mode
    pub fn verbose() -> Self {
        Self {
            verbose: true,
            debug: false,
            ..Self::default()
        }
    }

    /// Create with debug mode
    pub fn debug() -> Self {
        Self {
            verbose: true,
            debug: true,
            ..Self::default()
        }
    }

    /// Create with custom log directory
    pub fn with_log_dir(mut self, path: PathBuf) -> Self {
        self.log_dir = path;
        self
    }

    /// Enable terminal output
    pub fn with_terminal(mut self) -> Self {
        self.write_to_terminal = true;
        self
    }

    /// Disable file output
    pub fn without_file(mut self) -> Self {
        self.write_to_file = false;
        self
    }

    /// Get log file path
    pub fn log_file_path(&self) -> PathBuf {
        self.log_dir.join("latest.log")
    }

    /// Get errors log path
    pub fn errors_path(&self) -> PathBuf {
        self.log_dir.join("errors.jsonl")
    }

    /// Get sessions log directory
    pub fn sessions_dir(&self) -> PathBuf {
        self.log_dir.join("sessions")
    }

    /// Get MCP logs directory
    pub fn mcp_dir(&self) -> PathBuf {
        self.log_dir.join("mcp")
    }

    /// Get tracing filter level
    pub fn tracing_level(&self) -> &'static str {
        if self.debug {
            "debug"
        } else if self.verbose {
            "info"
        } else {
            "warn"
        }
    }
}
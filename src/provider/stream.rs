//! Streaming support for LLM providers
//!
//! Provides real-time streaming output using tokio channels.

#![allow(dead_code)] // Future use: SSE streaming

use tokio::sync::mpsc;

/// Streaming event types
#[derive(Debug, Clone)]
pub enum StreamEvent {
    /// Text content delta (incremental)
    TextDelta(String),
    /// Tool call started
    ToolCallStart {
        id: String,
        name: String,
    },
    /// Tool call arguments delta (incremental JSON)
    ToolCallDelta {
        id: String,
        arguments: String,
    },
    /// Tool call completed (all arguments received)
    ToolCallComplete {
        id: String,
        name: String,
        arguments: String,
    },
    /// Streaming finished
    Done,
    /// Error occurred
    Error(String),
}

impl StreamEvent {
    /// Check if this is a text delta event
    pub fn is_text(&self) -> bool {
        matches!(self, StreamEvent::TextDelta(_))
    }

    /// Check if this is an error event
    pub fn is_error(&self) -> bool {
        matches!(self, StreamEvent::Error(_))
    }

    /// Check if streaming is done
    pub fn is_done(&self) -> bool {
        matches!(self, StreamEvent::Done)
    }

    /// Get text content if this is a TextDelta
    pub fn text(&self) -> Option<&str> {
        match self {
            StreamEvent::TextDelta(text) => Some(text),
            _ => None,
        }
    }

    /// Get error message if this is an Error
    pub fn error_message(&self) -> Option<&str> {
        match self {
            StreamEvent::Error(msg) => Some(msg),
            _ => None,
        }
    }
}

/// Type alias for stream receiver
pub type StreamReceiver = mpsc::Receiver<StreamEvent>;

/// Type alias for stream sender (used by providers)
pub type StreamSender = mpsc::Sender<StreamEvent>;

/// Create a new streaming channel with default buffer size
pub fn create_stream_channel() -> (StreamSender, StreamReceiver) {
    mpsc::channel(100)
}

/// SSE (Server-Sent Events) parsing utilities
pub mod sse {
    /// Parse a SSE line and extract the data portion
    pub fn parse_data_line(line: &str) -> Option<&str> {
        if line.starts_with("data: ") {
            Some(&line[6..])
        } else {
            None
        }
    }

    /// Check if SSE data indicates stream end
    pub fn is_done(data: &str) -> bool {
        data == "[DONE]"
    }

    /// Parse event type from SSE line
    pub fn parse_event_line(line: &str) -> Option<&str> {
        if line.starts_with("event: ") {
            Some(&line[7..])
        } else {
            None
        }
    }
}
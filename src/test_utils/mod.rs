//! Integration test utilities for QuickHorse
//!
//! Provides mock implementations, test helpers, and fixtures for integration tests.
//! Reference: OpenClaude tests/sdk/helpers/mock-engine.ts

use crate::provider::{ContentBlock, Message, Provider, StreamEvent, StreamReceiver, create_stream_channel};
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Mock provider for testing - returns predetermined responses
pub struct MockProvider {
    /// Provider name
    name: String,
    /// Current model
    model: String,
    /// Pre-configured responses
    responses: HashMap<String, String>,
    /// Streaming responses
    streaming_responses: HashMap<String, Vec<String>>,
    /// Tool responses
    tool_responses: HashMap<String, Message>,
    /// Track messages sent
    messages_sent: RwLock<Vec<Message>>,
}

impl MockProvider {
    /// Create a new mock provider
    pub fn new(name: String, model: String) -> Self {
        Self {
            name,
            model,
            responses: HashMap::new(),
            streaming_responses: HashMap::new(),
            tool_responses: HashMap::new(),
            messages_sent: RwLock::new(Vec::new()),
        }
    }

    /// Add a response for a specific prompt pattern
    pub fn add_response(&mut self, pattern: String, response: String) {
        self.responses.insert(pattern, response);
    }

    /// Add a streaming response (sequence of text chunks)
    pub fn add_streaming_response(&mut self, pattern: String, chunks: Vec<String>) {
        self.streaming_responses.insert(pattern, chunks);
    }

    /// Add a tool response
    pub fn add_tool_response(&mut self, tool_name: String, response: Message) {
        self.tool_responses.insert(tool_name, response);
    }

    /// Get all messages sent to this provider
    pub fn get_messages_sent(&self) -> Vec<Message> {
        self.messages_sent.read().unwrap().clone()
    }

    /// Clear tracked messages
    pub fn clear_messages(&mut self) {
        self.messages_sent.write().unwrap().clear();
    }

    /// Find matching response for a prompt
    fn find_response(&self, messages: &[Message]) -> Option<String> {
        let last_user_msg = messages.iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.text_content());

        if let Some(text) = last_user_msg {
            for (pattern, response) in &self.responses {
                if text.contains(pattern) {
                    return Some(response.clone());
                }
            }
        }

        // Default response
        Some("Mock response from provider".to_string())
    }
}

#[async_trait]
impl Provider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn set_model(&mut self, model: String) {
        self.model = model;
    }

    async fn send_message(&self, messages: &[Message]) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        // Track messages
        self.messages_sent.write().unwrap().extend(messages.iter().cloned());

        let response = self.find_response(messages)
            .unwrap_or_else(|| "Mock response".to_string());

        Ok(Message::assistant(response))
    }

    async fn send_message_with_tools(
        &self,
        messages: &[Message],
        _tools: &[serde_json::Value],
    ) -> Result<Message, Box<dyn std::error::Error + Send + Sync>> {
        // Track messages
        self.messages_sent.write().unwrap().extend(messages.iter().cloned());

        // Check if there's a tool response configured
        if let Some(last_msg) = messages.iter().rev().find(|m| m.role == "assistant") {
            for block in &last_msg.content {
                if let ContentBlock::ToolUse { name, .. } = block {
                    if let Some(response) = self.tool_responses.get(name) {
                        return Ok(response.clone());
                    }
                }
            }
        }

        // Return default response with potential tool use
        let response = self.find_response(messages)
            .unwrap_or_else(|| "Mock response".to_string());

        Ok(Message::assistant(response))
    }

    async fn stream_message(&self, messages: &[Message]) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        // Track messages
        self.messages_sent.write().unwrap().extend(messages.iter().cloned());

        let chunks = self.find_streaming_chunks(messages);
        Ok(chunks.join(""))
    }

    async fn stream_message_channel(
        &self,
        messages: &[Message],
    ) -> Result<StreamReceiver, Box<dyn std::error::Error + Send + Sync>> {
        // Track messages
        self.messages_sent.write().unwrap().extend(messages.iter().cloned());

        let (tx, rx) = create_stream_channel();
        let chunks = self.find_streaming_chunks(messages);

        // Spawn task to send chunks
        tokio::spawn(async move {
            for chunk in chunks {
                tx.send(StreamEvent::TextDelta(chunk)).await.ok();
            }
            tx.send(StreamEvent::Done).await.ok();
        });

        Ok(rx)
    }

    fn list_models(&self) -> Vec<String> {
        vec![
            "mock-model-1".to_string(),
            "mock-model-2".to_string(),
        ]
    }
}

impl MockProvider {
    fn find_streaming_chunks(&self, messages: &[Message]) -> Vec<String> {
        let last_user_msg = messages.iter()
            .rev()
            .find(|m| m.role == "user")
            .map(|m| m.text_content());

        if let Some(text) = last_user_msg {
            for (pattern, chunks) in &self.streaming_responses {
                if text.contains(pattern) {
                    return chunks.clone();
                }
            }
        }

        // Default streaming response
        vec!["Mock ".to_string(), "streaming ".to_string(), "response".to_string()]
    }
}

/// Create a simple mock provider for basic tests
pub fn simple_mock_provider() -> Arc<RwLock<MockProvider>> {
    let provider = MockProvider::new("mock".to_string(), "mock-model".to_string());
    Arc::new(RwLock::new(provider))
}

/// Create a mock provider with predetermined responses
pub fn mock_provider_with_responses(responses: HashMap<String, String>) -> Arc<RwLock<MockProvider>> {
    let mut provider = MockProvider::new("mock".to_string(), "mock-model".to_string());
    for (pattern, response) in responses {
        provider.add_response(pattern, response);
    }
    Arc::new(RwLock::new(provider))
}

/// Test fixture for creating test sessions
pub struct TestSessionFixture {
    /// Temporary directory for session files
    pub temp_dir: tempfile::TempDir,
    /// Session ID
    pub session_id: String,
}

impl TestSessionFixture {
    /// Create a new test session fixture
    pub fn new() -> Self {
        let temp_dir = tempfile::TempDir::new().expect("Failed to create temp dir");
        let session_id = uuid::Uuid::new_v4().to_string();

        Self {
            temp_dir,
            session_id,
        }
    }

    /// Get session directory path
    pub fn session_path(&self) -> std::path::PathBuf {
        self.temp_dir.path().join("sessions").join(&self.session_id)
    }
}

/// Test fixture for creating test files
pub struct TestFileFixture {
    /// Temporary directory
    pub temp_dir: tempfile::TempDir,
}

impl TestFileFixture {
    /// Create a new test file fixture
    pub fn new() -> Self {
        Self {
            temp_dir: tempfile::TempDir::new().expect("Failed to create temp dir"),
        }
    }

    /// Create a test file with content
    pub fn create_file(&self, name: &str, content: &str) -> std::path::PathBuf {
        let path = self.temp_dir.path().join(name);
        std::fs::write(&path, content).expect("Failed to write file");
        path
    }

    /// Create a test directory
    pub fn create_dir(&self, name: &str) -> std::path::PathBuf {
        let path = self.temp_dir.path().join(name);
        std::fs::create_dir_all(&path).expect("Failed to create directory");
        path
    }

    /// Get the temp directory path
    pub fn path(&self) -> &std::path::Path {
        self.temp_dir.path()
    }
}

/// Drain all events from a stream receiver
pub async fn drain_stream(mut rx: StreamReceiver) -> Vec<StreamEvent> {
    let mut events = Vec::new();
    while let Some(event) = rx.recv().await {
        let is_done = event.is_done();
        events.push(event);
        if is_done {
            break;
        }
    }
    events
}

/// Collect streaming text from events
pub fn collect_stream_text(events: &[StreamEvent]) -> String {
    events.iter()
        .filter_map(|e| e.text())
        .collect()
}

/// Assert that stream contains text delta events
pub fn assert_stream_has_text(events: &[StreamEvent]) {
    assert!(events.iter().any(|e| e.is_text()), "Stream should contain text events");
}

/// Assert that stream ends with done event
pub fn assert_stream_ends_with_done(events: &[StreamEvent]) {
    assert!(events.iter().any(|e| e.is_done()), "Stream should end with done event");
}

/// Create a test message sequence
pub fn create_test_messages() -> Vec<Message> {
    vec![
        Message::system("You are a helpful assistant".to_string()),
        Message::user("Hello, how are you?".to_string()),
    ]
}

/// Create a test conversation with multiple turns
pub fn create_multi_turn_conversation() -> Vec<Message> {
    vec![
        Message::system("You are a helpful coding assistant".to_string()),
        Message::user("What is Rust?".to_string()),
        Message::assistant("Rust is a systems programming language focused on safety, speed, and concurrency.".to_string()),
        Message::user("Can you show me an example?".to_string()),
        Message::assistant("Here's a simple Rust program:\n\n```rust\nfn main() {\n    println!(\"Hello, world!\");\n}\n```".to_string()),
    ]
}

/// Create test tool input for Bash
pub fn create_bash_input(command: &str) -> serde_json::Value {
    serde_json::json!({
        "command": command,
        "timeout": 5
    })
}

/// Create test tool input for FileRead
pub fn create_read_input(file_path: &str) -> serde_json::Value {
    serde_json::json!({
        "file_path": file_path,
        "offset": 1,
        "limit": 100
    })
}

/// Create test tool input for Glob
pub fn create_glob_input(pattern: &str) -> serde_json::Value {
    serde_json::json!({
        "pattern": pattern
    })
}
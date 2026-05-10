//! Integration tests for Provider implementations
//!
//! Tests cover:
//! - Mock Provider behavior
//! - Streaming functionality
//! - Tool use integration
//! - Error handling

use quickhorse::provider::{Provider, Message, StreamEvent, ContentBlock};
use quickhorse::test_utils::{
    MockProvider, simple_mock_provider, mock_provider_with_responses,
    drain_stream, collect_stream_text, assert_stream_has_text, assert_stream_ends_with_done,
    create_test_messages, create_multi_turn_conversation,
};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// ============================================================================
// Mock Provider Tests
// ============================================================================

#[tokio::test]
async fn test_mock_provider_basic() {
    let provider = simple_mock_provider();

    let messages = create_test_messages();

    let guard = provider.read().unwrap();
    let result = guard.send_message(&messages).await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert_eq!(response.role, "assistant");
    assert!(!response.text_content().is_empty());
}

#[tokio::test]
async fn test_mock_provider_tracks_messages() {
    let provider = simple_mock_provider();

    let messages = create_test_messages();

    {
        let guard = provider.read().unwrap();
        guard.send_message(&messages).await.ok();
    }

    let guard = provider.read().unwrap();
    let sent = guard.get_messages_sent();
    assert!(!sent.is_empty());
}

#[tokio::test]
async fn test_mock_provider_pattern_response() {
    let mut responses = HashMap::new();
    responses.insert("hello".to_string(), "Hello! I'm a mock assistant.".to_string());
    responses.insert("code".to_string(), "Here's some code for you.".to_string());

    let provider = mock_provider_with_responses(responses);

    // Test pattern matching
    let messages = vec![Message::user("hello world".to_string())];
    {
        let guard = provider.read().unwrap();
        let result = guard.send_message(&messages).await.unwrap();
        assert!(result.text_content().contains("Hello!"));
    }

    // Test another pattern
    let messages = vec![Message::user("write code".to_string())];
    {
        let guard = provider.read().unwrap();
        let result = guard.send_message(&messages).await.unwrap();
        assert!(result.text_content().contains("code"));
    }
}

#[tokio::test]
async fn test_mock_provider_model_switch() {
    let provider = simple_mock_provider();

    {
        let mut guard = provider.write().unwrap();
        assert_eq!(guard.model(), "mock-model");
        guard.set_model("mock-model-2".to_string());
        assert_eq!(guard.model(), "mock-model-2");
    }

    let guard = provider.read().unwrap();
    assert_eq!(guard.model(), "mock-model-2");
}

// ============================================================================
// Streaming Tests
// ============================================================================

#[tokio::test]
async fn test_mock_provider_streaming() {
    let provider = simple_mock_provider();

    let messages = create_test_messages();

    let guard = provider.read().unwrap();
    let rx = guard.stream_message_channel(&messages).await.unwrap();

    let events = drain_stream(rx).await;

    assert_stream_has_text(&events);
    assert_stream_ends_with_done(&events);
}

#[tokio::test]
async fn test_stream_text_collection() {
    let provider = simple_mock_provider();

    let messages = create_test_messages();

    let guard = provider.read().unwrap();
    let rx = guard.stream_message_channel(&messages).await.unwrap();

    let events = drain_stream(rx).await;
    let text = collect_stream_text(&events);

    assert!(!text.is_empty());
}

#[tokio::test]
async fn test_stream_event_types() {
    // Test StreamEvent creation and methods
    let text_event = StreamEvent::TextDelta("Hello".to_string());
    assert!(text_event.is_text());
    assert!(!text_event.is_error());
    assert!(!text_event.is_done());

    let error_event = StreamEvent::Error("Connection failed".to_string());
    assert!(!error_event.is_text());
    assert!(error_event.is_error());

    let done_event = StreamEvent::Done;
    assert!(done_event.is_done());
}

// ============================================================================
// Multi-turn Conversation Tests
// ============================================================================

#[tokio::test]
async fn test_multi_turn_conversation() {
    let provider = simple_mock_provider();

    let messages = create_multi_turn_conversation();

    let guard = provider.read().unwrap();
    let result = guard.send_message(&messages).await.unwrap();

    assert_eq!(result.role, "assistant");
    // Verify messages were tracked
    let sent = guard.get_messages_sent();
    assert!(sent.len() >= 5); // Should have tracked all messages
}

// ============================================================================
// Tool Use Tests
// ============================================================================

#[tokio::test]
async fn test_provider_with_tools() {
    let provider = simple_mock_provider();

    let messages = create_test_messages();
    let tools = vec![
        serde_json::json!({
            "type": "function",
            "function": {
                "name": "bash",
                "description": "Execute shell commands",
                "parameters": {}
            }
        })
    ];

    let guard = provider.read().unwrap();
    let result = guard.send_message_with_tools(&messages, &tools).await;

    assert!(result.is_ok());
}

// ============================================================================
// Provider Trait Tests
// ============================================================================

#[test]
fn test_provider_name() {
    let provider = MockProvider::new("custom-mock".to_string(), "test-model".to_string());
    assert_eq!(provider.name(), "custom-mock");
}

#[test]
fn test_provider_model() {
    let provider = MockProvider::new("mock".to_string(), "gpt-4".to_string());
    assert_eq!(provider.model(), "gpt-4");
}

#[test]
fn test_provider_list_models() {
    let provider = MockProvider::new("mock".to_string(), "test".to_string());
    let models = provider.list_models();
    assert!(!models.is_empty());
}

// ============================================================================
// Message Tests
// ============================================================================

#[test]
fn test_message_creation() {
    let user_msg = Message::user("Hello".to_string());
    assert_eq!(user_msg.role, "user");
    assert_eq!(user_msg.text_content(), "Hello");

    let assistant_msg = Message::assistant("Hi there".to_string());
    assert_eq!(assistant_msg.role, "assistant");

    let system_msg = Message::system("Be helpful".to_string());
    assert_eq!(system_msg.role, "system");
}

#[test]
fn test_message_with_tools() {
    let tool_use = ContentBlock::tool_use(
        "call_123".to_string(),
        "bash".to_string(),
        serde_json::json!({"command": "ls"}),
    );

    let msg = Message::assistant_with_tools(vec![
        ContentBlock::text("Running command".to_string()),
        tool_use,
    ]);

    assert_eq!(msg.role, "assistant");
    assert!(msg.tool_uses().len() > 0);
}

#[test]
fn test_tool_result_block() {
    let result = ContentBlock::tool_result(
        "call_123".to_string(),
        "Command output".to_string(),
        false,
    );

    match result {
        ContentBlock::ToolResult { tool_use_id, content, is_error } => {
            assert_eq!(tool_use_id, "call_123");
            assert_eq!(content, "Command output");
            assert_eq!(is_error, Some(false));
        }
        _ => panic!("Expected ToolResult"),
    }
}
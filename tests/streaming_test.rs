//! Unit tests for streaming functionality

use quickhorse::provider::{StreamEvent, create_stream_channel};

#[test]
fn test_stream_event_types() {
    // Test TextDelta
    let event = StreamEvent::TextDelta("Hello".to_string());
    assert!(event.is_text());
    assert!(!event.is_error());
    assert!(!event.is_done());
    assert_eq!(event.text(), Some("Hello"));

    // Test Error
    let error_event = StreamEvent::Error("Connection failed".to_string());
    assert!(!error_event.is_text());
    assert!(error_event.is_error());
    assert!(!error_event.is_done());
    assert_eq!(error_event.error_message(), Some("Connection failed"));

    // Test Done
    let done_event = StreamEvent::Done;
    assert!(!done_event.is_text());
    assert!(!done_event.is_error());
    assert!(done_event.is_done());
    assert_eq!(done_event.text(), None);
}

#[test]
fn test_stream_channel_creation() {
    let (tx, mut rx) = create_stream_channel();

    // Send events
    tx.try_send(StreamEvent::TextDelta("Hello".to_string())).unwrap();
    tx.try_send(StreamEvent::TextDelta(" world".to_string())).unwrap();
    tx.try_send(StreamEvent::Done).unwrap();

    // Receive events
    let event1 = rx.try_recv().unwrap();
    assert_eq!(event1.text(), Some("Hello"));

    let event2 = rx.try_recv().unwrap();
    assert_eq!(event2.text(), Some(" world"));

    let event3 = rx.try_recv().unwrap();
    assert!(event3.is_done());
}

#[test]
fn test_stream_channel_buffer_size() {
    let (tx, _rx) = create_stream_channel();

    // Channel should accept multiple events without blocking
    for i in 0..50 {
        tx.try_send(StreamEvent::TextDelta(format!("Message {}", i))).unwrap();
    }
}

#[tokio::test]
async fn test_stream_channel_async() {
    let (tx, mut rx) = create_stream_channel();

    // Send in async context
    tokio::spawn(async move {
        tx.send(StreamEvent::TextDelta("Async message".to_string())).await.ok();
        tx.send(StreamEvent::Done).await.ok();
    });

    // Receive in async context
    let event1 = rx.recv().await.expect("Should receive event");
    assert_eq!(event1.text(), Some("Async message"));

    let event2 = rx.recv().await.expect("Should receive Done");
    assert!(event2.is_done());
}

#[test]
fn test_tool_call_events() {
    let start_event = StreamEvent::ToolCallStart {
        id: "call_123".to_string(),
        name: "Bash".to_string(),
    };
    assert!(!start_event.is_text());
    assert!(!start_event.is_done());

    let delta_event = StreamEvent::ToolCallDelta {
        id: "call_123".to_string(),
        arguments: "{\"command\": \"ls\"}".to_string(),
    };
    assert!(!delta_event.is_text());
    assert!(!delta_event.is_done());
}
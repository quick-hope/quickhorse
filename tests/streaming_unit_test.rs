//! Comprehensive unit tests for streaming functionality
//!
//! Tests cover:
//! - StreamEvent types and methods
//! - SSE parsing utilities
//! - Channel operations (sync/async)
//! - Error handling
//! - Edge cases (empty content, large content, etc.)

use quickhorse::provider::{StreamEvent, create_stream_channel, stream::sse};

// ============================================================================
// StreamEvent Tests
// ============================================================================

#[test]
fn test_text_delta_event() {
    let event = StreamEvent::TextDelta("Hello, world!".to_string());
    assert!(event.is_text());
    assert!(!event.is_error());
    assert!(!event.is_done());
    assert_eq!(event.text(), Some("Hello, world!"));
    assert_eq!(event.error_message(), None);
}

#[test]
fn test_empty_text_delta() {
    let event = StreamEvent::TextDelta("".to_string());
    assert!(event.is_text());
    assert_eq!(event.text(), Some(""));
}

#[test]
fn test_unicode_text_delta() {
    // Test CJK characters
    let event = StreamEvent::TextDelta("你好世界 🎉".to_string());
    assert!(event.is_text());
    assert_eq!(event.text(), Some("你好世界 🎉"));

    // Test emoji
    let emoji_event = StreamEvent::TextDelta("😀🎉🚀".to_string());
    assert_eq!(emoji_event.text(), Some("😀🎉🚀"));
}

#[test]
fn test_error_event() {
    let event = StreamEvent::Error("Connection timeout".to_string());
    assert!(!event.is_text());
    assert!(event.is_error());
    assert!(!event.is_done());
    assert_eq!(event.error_message(), Some("Connection timeout"));
    assert_eq!(event.text(), None);
}

#[test]
fn test_done_event() {
    let event = StreamEvent::Done;
    assert!(!event.is_text());
    assert!(!event.is_error());
    assert!(event.is_done());
    assert_eq!(event.text(), None);
    assert_eq!(event.error_message(), None);
}

#[test]
fn test_tool_call_start_event() {
    let event = StreamEvent::ToolCallStart {
        id: "call_abc123".to_string(),
        name: "bash".to_string(),
    };
    assert!(!event.is_text());
    assert!(!event.is_error());
    assert!(!event.is_done());
    assert_eq!(event.text(), None);
}

#[test]
fn test_tool_call_delta_event() {
    let event = StreamEvent::ToolCallDelta {
        id: "call_xyz789".to_string(),
        arguments: "{\"command\": \"ls -la\"}".to_string(),
    };
    assert!(!event.is_text());
    assert!(!event.is_error());
    assert!(!event.is_done());
    assert_eq!(event.text(), None);
}

#[test]
fn test_stream_event_clone() {
    let event = StreamEvent::TextDelta("Original".to_string());
    let cloned = event.clone();
    assert_eq!(event.text(), cloned.text());
}

#[test]
fn test_stream_event_debug() {
    let event = StreamEvent::TextDelta("Test".to_string());
    let debug_str = format!("{:?}", event);
    assert!(debug_str.contains("TextDelta"));
    assert!(debug_str.contains("Test"));
}

// ============================================================================
// SSE Parsing Tests
// ============================================================================

#[test]
fn test_sse_parse_data_line() {
    // Standard SSE data line
    assert_eq!(sse::parse_data_line("data: hello world"), Some("hello world"));

    // JSON data
    assert_eq!(sse::parse_data_line("data: {\"key\": \"value\"}"), Some("{\"key\": \"value\"}"));

    // Empty data
    assert_eq!(sse::parse_data_line("data: "), Some(""));
}

#[test]
fn test_sse_parse_data_line_edge_cases() {
    // No prefix
    assert_eq!(sse::parse_data_line("hello world"), None);

    // Wrong prefix
    assert_eq!(sse::parse_data_line("error: something"), None);

    // Multiple spaces after prefix
    assert_eq!(sse::parse_data_line("data:   spaced"), Some("  spaced"));

    // Unicode data
    assert_eq!(sse::parse_data_line("data: 你好"), Some("你好"));
}

#[test]
fn test_sse_is_done() {
    // Standard [DONE] marker
    assert!(sse::is_done("[DONE]"));

    // Not done
    assert!(!sse::is_done("some data"));
    assert!(!sse::is_done("{\"content\": \"text\"}"));
    assert!(!sse::is_done(""));
}

#[test]
fn test_sse_parse_event_line() {
    // Standard event line
    assert_eq!(sse::parse_event_line("event: message"), Some("message"));
    assert_eq!(sse::parse_event_line("event: error"), Some("error"));
    assert_eq!(sse::parse_event_line("event: ping"), Some("ping"));

    // No prefix
    assert_eq!(sse::parse_event_line("message"), None);

    // Wrong prefix
    assert_eq!(sse::parse_event_line("type: message"), None);
}

// ============================================================================
// Channel Tests (Synchronous)
// ============================================================================

#[test]
fn test_channel_basic_send_receive() {
    let (tx, mut rx) = create_stream_channel();

    // Send events
    tx.try_send(StreamEvent::TextDelta("First".to_string())).unwrap();
    tx.try_send(StreamEvent::TextDelta("Second".to_string())).unwrap();
    tx.try_send(StreamEvent::Done).unwrap();

    // Receive in order
    assert_eq!(rx.try_recv().unwrap().text(), Some("First"));
    assert_eq!(rx.try_recv().unwrap().text(), Some("Second"));
    assert!(rx.try_recv().unwrap().is_done());
}

#[test]
fn test_channel_send_multiple_events() {
    let (tx, _rx) = create_stream_channel();

    // Send many events without blocking
    for i in 0..100 {
        tx.try_send(StreamEvent::TextDelta(format!("Msg {}", i))).unwrap();
    }
}

#[test]
fn test_channel_buffer_capacity() {
    let (tx, _rx) = create_stream_channel();

    // Channel should handle buffer size (100)
    let sent_count = (0..150)
        .filter_map(|i| tx.try_send(StreamEvent::TextDelta(format!("{}", i))).ok())
        .count();

    // Should succeed for at least buffer size
    assert!(sent_count >= 100);
}

#[test]
fn test_channel_mixed_event_types() {
    let (tx, mut rx) = create_stream_channel();

    // Send mixed events
    tx.try_send(StreamEvent::TextDelta("Hello".to_string())).unwrap();
    tx.try_send(StreamEvent::ToolCallStart {
        id: "call_1".to_string(),
        name: "read_file".to_string(),
    }).unwrap();
    tx.try_send(StreamEvent::ToolCallDelta {
        id: "call_1".to_string(),
        arguments: "{\"path\": \"/test\"}".to_string(),
    }).unwrap();
    tx.try_send(StreamEvent::TextDelta("Result".to_string())).unwrap();
    tx.try_send(StreamEvent::Done).unwrap();

    // Verify order preserved
    assert!(rx.try_recv().unwrap().is_text());
    assert!(!rx.try_recv().unwrap().is_text()); // ToolCallStart
    assert!(!rx.try_recv().unwrap().is_text()); // ToolCallDelta
    assert!(rx.try_recv().unwrap().is_text());
    assert!(rx.try_recv().unwrap().is_done());
}

#[test]
fn test_channel_error_propagation() {
    let (tx, mut rx) = create_stream_channel();

    tx.try_send(StreamEvent::TextDelta("Start".to_string())).unwrap();
    tx.try_send(StreamEvent::Error("Something went wrong".to_string())).unwrap();

    let first = rx.try_recv().unwrap();
    assert!(first.is_text());

    let error = rx.try_recv().unwrap();
    assert!(error.is_error());
    assert_eq!(error.error_message(), Some("Something went wrong"));
}

// ============================================================================
// Channel Tests (Async)
// ============================================================================

#[tokio::test]
async fn test_async_channel_send_receive() {
    let (tx, mut rx) = create_stream_channel();

    // Send in spawned task
    tokio::spawn(async move {
        tx.send(StreamEvent::TextDelta("Async hello".to_string())).await.ok();
        tx.send(StreamEvent::Done).await.ok();
    });

    // Receive in async context
    let event1 = rx.recv().await.expect("Should receive first event");
    assert_eq!(event1.text(), Some("Async hello"));

    let event2 = rx.recv().await.expect("Should receive Done");
    assert!(event2.is_done());
}

#[tokio::test]
async fn test_async_channel_large_content() {
    let (tx, mut rx) = create_stream_channel();

    // Generate large content (simulating streaming response)
    let large_text = "X".repeat(10000);
    let text_clone = large_text.clone();

    tokio::spawn(async move {
        // Send in chunks
        for chunk in text_clone.as_bytes().chunks(100) {
            tx.send(StreamEvent::TextDelta(String::from_utf8_lossy(chunk).to_string())).await.ok();
        }
        tx.send(StreamEvent::Done).await.ok();
    });

    // Accumulate all chunks
    let mut accumulated = String::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::TextDelta(text) => accumulated.push_str(&text),
            StreamEvent::Done => break,
            _ => {}
        }
    }

    assert_eq!(accumulated.len(), 10000);
}

#[tokio::test]
async fn test_async_channel_concurrent_senders() {
    let (tx, mut rx) = create_stream_channel();
    let tx1 = tx.clone();
    let tx2 = tx.clone();

    // Two concurrent senders
    let h1 = tokio::spawn(async move {
        tx1.send(StreamEvent::TextDelta("Sender1".to_string())).await.ok();
    });

    let h2 = tokio::spawn(async move {
        tx2.send(StreamEvent::TextDelta("Sender2".to_string())).await.ok();
    });

    // Wait for senders
    h1.await.ok();
    h2.await.ok();
    tx.send(StreamEvent::Done).await.ok();

    // Receive both messages
    let mut received = Vec::new();
    while let Some(event) = rx.recv().await {
        match event {
            StreamEvent::TextDelta(text) => received.push(text),
            StreamEvent::Done => break,
            _ => {}
        }
    }

    assert_eq!(received.len(), 2);
}

#[tokio::test]
async fn test_async_channel_timeout() {
    use tokio::time::{timeout, Duration};

    let (tx, mut rx) = create_stream_channel();

    // No sender - receiver should timeout
    let result = timeout(Duration::from_millis(100), rx.recv()).await;
    assert!(result.is_err()); // Timeout occurred
}

// ============================================================================
// Edge Cases and Stress Tests
// ============================================================================

#[test]
fn test_empty_channel() {
    let (_tx, mut rx) = create_stream_channel();

    // Empty channel - try_recv should fail
    assert!(rx.try_recv().is_err());
}

#[test]
fn test_channel_closed_sender() {
    let (tx, mut rx) = create_stream_channel();

    tx.try_send(StreamEvent::TextDelta("Last".to_string())).unwrap();
    drop(tx); // Close sender

    // Should still be able to receive
    assert!(rx.try_recv().is_ok());

    // But no more messages
    assert!(rx.try_recv().is_err());
}

#[tokio::test]
async fn test_async_channel_all_event_types() {
    let (tx, mut rx) = create_stream_channel();

    tokio::spawn(async move {
        // Send all event types
        tx.send(StreamEvent::TextDelta("text".to_string())).await.ok();
        tx.send(StreamEvent::ToolCallStart {
            id: "id1".to_string(),
            name: "tool".to_string(),
        }).await.ok();
        tx.send(StreamEvent::ToolCallDelta {
            id: "id1".to_string(),
            arguments: "args".to_string(),
        }).await.ok();
        tx.send(StreamEvent::Error("err".to_string())).await.ok();
        tx.send(StreamEvent::Done).await.ok();
    });

    // Count received events
    let mut count = 0;
    while let Some(_event) = rx.recv().await {
        count += 1;
        if count >= 5 {
            break;
        }
    }

    assert_eq!(count, 5);
}

#[test]
fn test_text_accumulation_simulation() {
    let (tx, mut rx) = create_stream_channel();

    // Simulate streaming response accumulation
    let chunks = vec!["The ", "answer ", "is ", "42."];
    for chunk in chunks {
        tx.try_send(StreamEvent::TextDelta(chunk.to_string())).unwrap();
    }
    tx.try_send(StreamEvent::Done).unwrap();

    // Accumulate like a consumer would
    let mut text = String::new();
    loop {
        match rx.try_recv() {
            Ok(event) => match event {
                StreamEvent::TextDelta(t) => text.push_str(&t),
                StreamEvent::Done => break,
                _ => {}
            },
            Err(_) => break,
        }
    }

    assert_eq!(text, "The answer is 42.");
}

// ============================================================================
// JSON Response Parsing Simulation
// ============================================================================

#[test]
fn test_openai_stream_response_simulation() {
    // Simulate OpenAI SSE data format
    let sse_data = vec![
        "data: {\"choices\": [{\"delta\": {\"content\": \"Hello\"}}]}",
        "data: {\"choices\": [{\"delta\": {\"content\": \" world\"}}]}",
        "data: [DONE]",
    ];

    let mut accumulated = String::new();
    for line in sse_data {
        if let Some(data) = sse::parse_data_line(line) {
            if sse::is_done(data) {
                break;
            }
            // Parse JSON (simplified)
            if data.contains("\"content\":") {
                // Extract content (mock parsing)
                if data.contains("Hello") {
                    accumulated.push_str("Hello");
                } else if data.contains("world") {
                    accumulated.push_str(" world");
                }
            }
        }
    }

    assert_eq!(accumulated, "Hello world");
}

#[test]
fn test_anthropic_stream_event_simulation() {
    // Simulate Anthropic SSE format
    let sse_lines = vec![
        "event: content_block_delta",
        "data: {\"type\": \"content_block_delta\", \"delta\": {\"type\": \"text_delta\", \"text\": \"Hi\"}}",
        "event: content_block_delta",
        "data: {\"type\": \"content_block_delta\", \"delta\": {\"type\": \"text_delta\", \"text\": \" there\"}}",
        "event: message_stop",
        "data: {}",
    ];

    let mut text = String::new();
    let mut i = 0;
    while i < sse_lines.len() {
        let line = sse_lines[i];

        // Check for event type
        if let Some(event_type) = sse::parse_event_line(line) {
            if event_type == "message_stop" {
                break;
            }
        }

        // Parse data
        if let Some(data) = sse::parse_data_line(line) {
            if data.contains("\"text\":") {
                // Mock extraction
                if data.contains("Hi") && !data.contains("there") {
                    text.push_str("Hi");
                } else if data.contains("there") {
                    text.push_str(" there");
                }
            }
        }

        i += 1;
    }

    assert_eq!(text, "Hi there");
}
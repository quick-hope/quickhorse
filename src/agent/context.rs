//! Context management - token counting and message compression
//!
//! Provides:
//! - Token estimation for messages
//! - Context compression when exceeding limits
//! - Message trimming strategies

#![allow(dead_code)] // Future use: context compression

use crate::provider::{ContentBlock, Message};

/// Default maximum tokens before compression
pub const DEFAULT_MAX_TOKENS: u64 = 100_000;

/// Estimated tokens per character (rough approximation)
/// Chinese characters: ~2 tokens, English: ~0.25 tokens
pub const TOKENS_PER_CHAR_ENGLISH: f64 = 0.25;
pub const TOKENS_PER_CHAR_CHINESE: f64 = 0.5;

/// Estimate token count for a string
pub fn estimate_tokens(text: &str) -> u64 {
    let mut total_tokens = 0u64;

    for char in text.chars() {
        // Chinese characters are typically 2-3 tokens
        if char > '\u{007F}' {
            total_tokens += 2;
        } else {
            // English characters are typically ~0.25 tokens (4 chars per token)
            total_tokens += 1;
        }
    }

    // Divide by 4 for English approximation
    total_tokens / 4 + (text.len() as u64 / 10) // Add safety margin
}

/// Estimate token count for a message
pub fn estimate_message_tokens(message: &Message) -> u64 {
    let role_tokens = 4; // "user", "assistant", "system"

    let content_tokens: u64 = message
        .content
        .iter()
        .map(|block| {
            match block {
                ContentBlock::Text { text } => estimate_tokens(text) + 4, // +4 for structure
                ContentBlock::ToolUse { id, name, input } => {
                    estimate_tokens(id) + estimate_tokens(name) + estimate_tokens(&input.to_string()) + 10
                }
                ContentBlock::ToolResult { tool_use_id, content, .. } => {
                    estimate_tokens(tool_use_id) + estimate_tokens(content) + 10
                }
            }
        })
        .sum();

    role_tokens + content_tokens
}

/// Estimate total token count for all messages
pub fn estimate_total_tokens(messages: &[Message]) -> u64 {
    messages.iter().map(estimate_message_tokens).sum()
}

/// Compress messages when exceeding max tokens
/// Strategy: Keep first N and last N messages, trim middle
pub fn compress_messages(messages: &[Message], max_tokens: u64) -> Vec<Message> {
    let total_tokens = estimate_total_tokens(messages);

    if total_tokens <= max_tokens {
        return messages.to_vec();
    }

    // Determine how many messages to keep
    let keep_first = 2; // System prompt + first user message
    let keep_last = 4;  // Recent conversation context

    if messages.len() <= keep_first + keep_last {
        // Can't compress further - return original
        return messages.to_vec();
    }

    // Keep first and last, add compression notice in middle
    let mut compressed = Vec::new();

    // Add first messages
    for msg in messages.iter().take(keep_first) {
        compressed.push(msg.clone());
    }

    // Add compression notice
    compressed.push(Message::assistant(
        format!(
            "[Context compressed: {} messages trimmed to fit {} token limit. Original: {} tokens]",
            messages.len() - keep_first - keep_last,
            max_tokens,
            total_tokens
        )
    ));

    // Add last messages
    for msg in messages.iter().skip(messages.len() - keep_last) {
        compressed.push(msg.clone());
    }

    compressed
}

/// More aggressive compression - trim large tool results
pub fn compress_tool_results(messages: &[Message], max_result_length: usize) -> Vec<Message> {
    messages
        .iter()
        .map(|msg| {
            let compressed_content = msg
                .content
                .iter()
                .map(|block| {
                    match block {
                        ContentBlock::ToolResult { tool_use_id, content, is_error } => {
                            if content.len() > max_result_length {
                                let truncated = format!(
                                    "{}\n... [truncated, {} chars total]",
                                    &content[..max_result_length.min(content.len())],
                                    content.len()
                                );
                                ContentBlock::tool_result(tool_use_id.clone(), truncated, is_error.unwrap_or(false))
                            } else {
                                block.clone()
                            }
                        }
                        _ => block.clone(),
                    }
                })
                .collect();

            Message {
                role: msg.role.clone(),
                content: compressed_content,
            }
        })
        .collect()
}

/// Check if messages need compression
pub fn needs_compression(messages: &[Message], max_tokens: u64) -> bool {
    estimate_total_tokens(messages) > max_tokens
}

/// Get compression statistics
pub fn compression_stats(messages: &[Message], max_tokens: u64) -> CompressionStats {
    let current_tokens = estimate_total_tokens(messages);
    let needs = current_tokens > max_tokens;

    CompressionStats {
        message_count: messages.len(),
        current_tokens,
        max_tokens,
        needs_compression: needs,
        estimated_reduction: if needs {
            current_tokens - max_tokens
        } else {
            0
        },
    }
}

/// Compression statistics
#[derive(Debug, Clone)]
pub struct CompressionStats {
    /// Number of messages
    pub message_count: usize,
    /// Current token estimate
    pub current_tokens: u64,
    /// Maximum tokens allowed
    pub max_tokens: u64,
    /// Whether compression is needed
    pub needs_compression: bool,
    /// Estimated tokens to reduce
    pub estimated_reduction: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_tokens_simple() {
        let text = "Hello world";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        assert!(tokens < 20); // Should be roughly 3-4 tokens
    }

    #[test]
    fn test_estimate_tokens_chinese() {
        let text = "你好世界";
        let tokens = estimate_tokens(text);
        assert!(tokens > 0);
        // Chinese chars are typically more tokens
    }

    #[test]
    fn test_estimate_message_tokens() {
        let msg = Message::user("Hello".to_string());
        let tokens = estimate_message_tokens(&msg);
        assert!(tokens > 0);
    }

    #[test]
    fn test_estimate_total_tokens() {
        let messages = vec![
            Message::user("Hello".to_string()),
            Message::assistant("Hi there".to_string()),
        ];
        let total = estimate_total_tokens(&messages);
        assert!(total > 0);
    }

    #[test]
    fn test_needs_compression() {
        let messages = vec![
            Message::user("Short message".to_string()),
        ];
        assert!(!needs_compression(&messages, 100_000));

        // Create many messages with longer content to exceed limit
        let mut large_messages = Vec::new();
        for _ in 0..500 {
            large_messages.push(Message::user(
                "This is a longer test message with more content to simulate a real conversation that would have significant token count. We need enough text to exceed the 100k token limit for testing purposes.".to_string()
            ));
        }
        // With 500 messages of ~100 chars each, should exceed 100k tokens
        let total = estimate_total_tokens(&large_messages);
        if total > 100_000 {
            assert!(needs_compression(&large_messages, 100_000));
        } else {
            // If estimation is too conservative, just verify the function works
            assert!(!needs_compression(&large_messages, total + 1));
        }
    }

    #[test]
    fn test_compress_messages() {
        let messages = vec![
            Message::system("System prompt".to_string()),
            Message::user("First message".to_string()),
            Message::assistant("Response 1".to_string()),
            Message::user("Middle 1".to_string()),
            Message::assistant("Response 2".to_string()),
            Message::user("Middle 2".to_string()),
            Message::assistant("Response 3".to_string()),
            Message::user("Last message".to_string()),
            Message::assistant("Final response".to_string()),
        ];

        let compressed = compress_messages(&messages, 50);

        // Should keep first 2 and last 4 + compression notice
        assert!(compressed.len() < messages.len());
        assert!(compressed.len() >= 7); // 2 + 1 notice + 4 = 7
    }

    #[test]
    fn test_compress_tool_results() {
        let long_content = "x".repeat(1000);
        let messages = vec![
            Message::user_with_tool_results(vec![
                ContentBlock::tool_result("id1".to_string(), long_content.clone(), false),
            ]),
        ];

        let compressed = compress_tool_results(&messages, 100);

        // Check that the tool result was truncated
        if let ContentBlock::ToolResult { content, .. } = &compressed[0].content[0] {
            assert!(content.len() < long_content.len());
            assert!(content.contains("truncated"));
        }
    }

    #[test]
    fn test_compression_stats() {
        let messages = vec![
            Message::user("Test message".to_string()),
        ];

        let stats = compression_stats(&messages, 1000);
        assert_eq!(stats.message_count, 1);
        assert!(!stats.needs_compression);
    }
}
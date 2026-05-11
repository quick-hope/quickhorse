//! Integration tests for Agent workflow
//!
//! Tests cover:
//! - Simple query execution
//! - Multi-turn conversations
//! - Tool execution cycles
//! - Error handling in agent loop

use quickhorse::agent::{Agent, AgentConfig};
use quickhorse::permissions::PermissionMode;
use quickhorse::provider::{Message};
use quickhorse::test_utils::{
    MockProvider, simple_mock_provider, mock_provider_with_responses,
    create_test_messages,
};
use std::collections::HashMap;

// ============================================================================
// Agent Creation Tests
// ============================================================================

#[test]
fn test_agent_creation() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();

    let agent = Agent::new(provider, config);

    assert!(agent.messages().is_empty());
}

#[test]
fn test_agent_with_custom_config() {
    let provider = simple_mock_provider();
    let config = AgentConfig {
        max_iterations: 5,
        system_prompt: "Custom prompt".to_string(),
        permission_mode: PermissionMode::Default,
    };

    let agent = Agent::new(provider, config);

    // Agent should be created with custom config
    assert!(agent.messages().is_empty());
}

// ============================================================================
// Message Management Tests
// ============================================================================

#[test]
fn test_agent_add_message() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();
    let mut agent = Agent::new(provider, config);

    agent.add_message(Message::user("Hello".to_string()));

    assert_eq!(agent.messages().len(), 1);
    assert_eq!(agent.messages()[0].role, "user");
}

#[test]
fn test_agent_clear_messages() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();
    let mut agent = Agent::new(provider, config);

    agent.add_message(Message::user("Test 1".to_string()));
    agent.add_message(Message::assistant("Response 1".to_string()));
    agent.add_message(Message::user("Test 2".to_string()));

    assert_eq!(agent.messages().len(), 3);

    agent.clear();

    assert!(agent.messages().is_empty());
}

#[test]
fn test_agent_sync_messages() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();
    let mut agent = Agent::new(provider, config);

    let messages = create_test_messages();
    for msg in messages {
        agent.add_message(msg);
    }

    assert_eq!(agent.messages().len(), 2);
}

// ============================================================================
// Agent Query Tests
// ============================================================================

#[tokio::test]
async fn test_agent_simple_query() {
    let mut responses = HashMap::new();
    responses.insert("hello".to_string(), "Hello! How can I help?".to_string());

    let provider = mock_provider_with_responses(responses);
    let config = AgentConfig::default();
    let mut agent = Agent::new(provider, config);

    agent.add_message(Message::user("hello".to_string()));

    let result = agent.process_last_user_message().await;

    assert!(result.is_ok());
    let response = result.unwrap();
    assert!(response.contains("Hello"));
}

#[tokio::test]
async fn test_agent_multi_turn() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();
    let mut agent = Agent::new(provider, config);

    // First turn
    agent.add_message(Message::user("What is Rust?".to_string()));
    let result1 = agent.process_last_user_message().await;
    assert!(result1.is_ok());

    // Second turn
    agent.add_message(Message::user("Tell me more".to_string()));
    let result2 = agent.process_last_user_message().await;
    assert!(result2.is_ok());

    // Check message history
    assert!(agent.messages().len() >= 4);
}

// ============================================================================
// Tool Execution Tests
// ============================================================================

#[tokio::test]
async fn test_agent_with_tools() {
    let provider = simple_mock_provider();
    let system_prompt = "You are a helpful assistant with tools.".to_string();
    let config = AgentConfig {
        max_iterations: 5,
        system_prompt: system_prompt.clone(),
        permission_mode: PermissionMode::BypassPermissions,
    };
    let mut agent = Agent::new(provider, config);

    // Add system message
    agent.add_message(Message::system(system_prompt));
    agent.add_message(Message::user("List the files".to_string()));

    // Agent should process and potentially use tools
    let result = agent.process_last_user_message().await;

    // Result may or may not use tools depending on mock response
    assert!(result.is_ok());
}

// ============================================================================
// Error Handling Tests
// ============================================================================

#[tokio::test]
async fn test_agent_max_iterations() {
    let provider = simple_mock_provider();
    let config = AgentConfig {
        max_iterations: 3,
        system_prompt: "Test".to_string(),
        permission_mode: PermissionMode::Default,
    };
    let mut agent = Agent::new(provider, config);

    agent.add_message(Message::user("Complex task".to_string()));

    // Process should not exceed max iterations
    let result = agent.process_last_user_message().await;
    assert!(result.is_ok());
}

// ============================================================================
// Configuration Tests
// ============================================================================

#[test]
fn test_agent_config_default() {
    let config = AgentConfig::default();

    assert!(config.max_iterations > 0);
    assert!(!config.system_prompt.is_empty());
    assert_eq!(config.permission_mode, PermissionMode::Default);
}

#[test]
fn test_agent_config_custom() {
    let config = AgentConfig {
        max_iterations: 100,
        system_prompt: "Custom system prompt for testing".to_string(),
        permission_mode: PermissionMode::AcceptEdits,
    };

    assert_eq!(config.max_iterations, 100);
    assert!(config.system_prompt.contains("Custom"));
    assert_eq!(config.permission_mode, PermissionMode::AcceptEdits);
}

// ============================================================================
// Provider Access Tests
// ============================================================================

#[test]
fn test_agent_provider_access() {
    let provider = simple_mock_provider();
    let config = AgentConfig::default();
    let agent = Agent::new(provider, config);

    // Should be able to get provider reference
    let provider_ref = agent.provider();
    assert!(provider_ref.read().unwrap().name() == "mock");
}
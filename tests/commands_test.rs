//! Unit tests for CommandRegistry and commands

use quickhorse::commands::{CommandRegistry, Command, HelpCommand, ClearCommand};
use quickhorse::config::Config;
use quickhorse::provider::{OpenAIProvider, Message};
use std::sync::{Arc, RwLock};

fn create_test_context() -> quickhorse::commands::CommandContext {
    let provider = Arc::new(RwLock::new(OpenAIProvider::new("test_key".to_string(), "gpt-4".to_string())));
    let config = Config::default();
    quickhorse::commands::CommandContext::new(provider, config)
}

#[test]
fn test_registry_is_command() {
    assert!(CommandRegistry::is_command("/help"));
    assert!(CommandRegistry::is_command("/provider"));
    assert!(CommandRegistry::is_command("/clear"));
    assert!(!CommandRegistry::is_command("hello"));
    assert!(!CommandRegistry::is_command(""));
}

#[test]
fn test_registry_parse_input() {
    // Valid commands
    let (cmd, args) = CommandRegistry::parse_input("/help").unwrap();
    assert_eq!(cmd, "help");
    assert_eq!(args.len(), 0);

    let (cmd, args) = CommandRegistry::parse_input("/provider ollama").unwrap();
    assert_eq!(cmd, "provider");
    assert_eq!(args.len(), 1);
    assert_eq!(args[0], "ollama");

    let (cmd, args) = CommandRegistry::parse_input("/model gpt-4o").unwrap();
    assert_eq!(cmd, "model");
    assert_eq!(args.len(), 1);

    // Invalid commands
    assert!(CommandRegistry::parse_input("").is_none());
    assert!(CommandRegistry::parse_input("/").is_none());
    assert!(CommandRegistry::parse_input("not a command").is_none());
}

#[test]
fn test_help_command_properties() {
    let cmd = HelpCommand;
    assert_eq!(cmd.name(), "help");
    assert_eq!(cmd.description(), "Show available commands");
}

#[tokio::test]
async fn test_help_command_execute() {
    let cmd = HelpCommand;
    let mut ctx = create_test_context();
    let result = cmd.execute(&[], &mut ctx).await;

    assert!(!result.clear_history);
    assert!(!result.provider_changed);
    assert!(result.output.contains("Available Commands"));
    assert!(result.output.contains("/help"));
    assert!(result.output.contains("/clear"));
}

#[test]
fn test_clear_command_properties() {
    let cmd = ClearCommand;
    assert_eq!(cmd.name(), "clear");
    assert_eq!(cmd.description(), "Clear message history");
}

#[tokio::test]
async fn test_clear_command_execute() {
    let cmd = ClearCommand;
    let mut ctx = create_test_context();

    // Add some messages
    ctx.messages.push(Message::user("test".to_string()));
    ctx.messages.push(Message::assistant("response".to_string()));
    assert_eq!(ctx.messages.len(), 2);

    let result = cmd.execute(&[], &mut ctx).await;

    assert!(result.clear_history);
    assert!(!result.provider_changed);
    assert_eq!(result.output, "Message history cleared");
    assert_eq!(ctx.messages.len(), 0); // Messages were cleared
}

#[tokio::test]
async fn test_registry_list_commands() {
    let registry = CommandRegistry::new();
    let commands = registry.list_commands();

    assert!(commands.contains(&"help"));
    assert!(commands.contains(&"clear"));
    assert!(commands.contains(&"provider"));
    assert!(commands.contains(&"model"));
    assert!(commands.contains(&"status"));
    assert!(commands.contains(&"session"));
}

#[tokio::test]
async fn test_registry_get_all_help() {
    let registry = CommandRegistry::new();
    let help_texts = registry.get_all_help();

    assert!(!help_texts.is_empty());
    for text in help_texts {
        assert!(text.starts_with("/"));
    }
}
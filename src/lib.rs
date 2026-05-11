//! QuickHorse - A CLI coding agent written in Rust
//!
//! This crate provides a lightweight coding agent with:
//! - Multiple LLM providers (OpenAI, Anthropic, Gemini, Ollama)
//! - Tools (Bash, FileRead, FileEdit, Glob, Grep, WebFetch)
//! - MCP protocol support
//! - Session management
//! - Slash commands
//! - Structured logging with tracing
//! - User-friendly error classification
//! - Permission control with whitelist/blacklist
//! - Secure API key storage (macOS Keychain, encrypted file)

pub mod agent;
pub mod commands;
pub mod config;
pub mod error;
pub mod log;
pub mod mcp;
pub mod permissions;
pub mod provider;
pub mod secure_storage;
pub mod session;
pub mod tools;
pub mod tui;

/// Test utilities module for integration tests
pub mod test_utils;
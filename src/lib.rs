//! QuickHorse - A CLI coding agent written in Rust
//!
//! This crate provides a lightweight coding agent with:
//! - Multiple LLM providers (OpenAI, Anthropic, Gemini, Ollama)
//! - Tools (Bash, FileRead, FileEdit, Glob, Grep, WebFetch)
//! - MCP protocol support
//! - Session management
//! - Slash commands
//! - Structured logging with tracing

pub mod agent;
pub mod commands;
pub mod config;
pub mod log;
pub mod mcp;
pub mod provider;
pub mod session;
pub mod tools;
pub mod tui;
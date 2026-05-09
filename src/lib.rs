//! QuickHorse - A CLI coding agent written in Rust
//!
//! This crate provides a lightweight coding agent with:
//! - Multiple LLM providers (OpenAI, Anthropic, Gemini, Ollama)
//! - Tools (Bash, FileRead, FileEdit, Glob, Grep)
//! - MCP protocol support
//! - Session management

pub mod agent;
pub mod config;
pub mod mcp;
pub mod provider;
pub mod session;
pub mod tools;
pub mod tui;
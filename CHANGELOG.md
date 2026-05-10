# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-05-11

### Added

#### Streaming Output
- Real-time LLM response streaming for all providers
- `StreamEvent` type system (TextDelta, ToolStart, ToolComplete, Error, Done)
- `stream_message_channel()` method in Provider trait
- SSE/JSON lines format parsing for Anthropic, Gemini, Ollama
- `bytes_stream()` implementation for true streaming (not buffered)

#### Progress Indicators
- Spinner animation component (`Spinner`)
- Unicode block progress bar (`ProgressBar`)
- Tool execution progress tracking (`ToolProgress`)
- Unified progress management (`ProgressManager`)
- TUI integration with streaming output

#### Error Classification System
- `ErrorCode` enum with 33 error codes (E001-E404)
- `ErrorCategory` enum (Network, Authentication, Parameter, etc.)
- `QuickHorseError` with user-friendly messages and recovery hints
- Provider-specific error parsing (OpenAI, Anthropic, Gemini, Ollama, BaiLian)
- HTTP status code mapping (401→AUTH, 429→RATE_LIMIT, 500→SERVER)
- TUI boxed error display with Unicode width support

#### Logging System
- `tracing` crate integration for structured logging
- `--verbose` and `--debug` CLI flags
- File logging with rotation
- In-memory log sink for diagnostics
- `DiagnosticLogEntry` with event tracking

#### Integration Test Framework
- `MockProvider` for testing with pre-configured responses
- `TestSessionFixture` and `TestFileFixture` for temporary test environments
- Helper functions (`drain_stream`, `collect_stream_text`, `create_test_messages`)
- Provider integration tests (15 tests)
- Agent workflow tests (13 tests)
- Session management tests (15 tests)
- GitHub Actions CI with coverage reporting (cargo-tarpaulin)

### Changed
- Provider trait now requires `name()`, `model()`, and `set_model()` methods
- Agent uses `Arc<RwLock<dyn Provider>>` for dynamic switching
- Session API updated: `Session::new(provider, model)` signature
- `SessionManager` now has `sessions_dir()` accessor

### Fixed
- Streaming tests now properly handle `mut rx` and event cloning
- Unicode width calculation in TUI error display

### Test Results
- Unit tests: 21 passed
- Integration tests: 43 passed (Provider: 15, Agent: 13, Session: 15)
- Total: 125+ tests passing

---

## [0.1.0] - 2026-05-10

### Added

#### Core Framework
- CLI framework with clap argument parsing
- TUI implementation with ratatui
- Setup wizard for first-time configuration
- Slash commands (`/help`, `/provider`, `/model`, `/clear`, `/status`, `/session`)

#### Provider Support
- OpenAI provider (GPT-4, GPT-4o, GPT-3.5-turbo)
- Anthropic provider (Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Haiku)
- Gemini provider (Gemini 1.5 Pro, Gemini 1.5 Flash, Gemini 2.0)
- Ollama provider (Llama3, Mistral, Qwen2, DeepSeek-Coder)
- Compatible API support (BaiLian, DeepSeek, Moonshot via base_url)

#### Tools
- BashTool - Execute shell commands
- FileReadTool - Read file content
- FileEditTool - Edit files (find and replace)
- GlobTool - File pattern matching
- GrepTool - Content search with regex
- WebFetchTool - Fetch web page content

#### MCP Protocol
- MCP Server implementation (JSON-RPC 2.0)
- MCP Client for connecting external servers
- Tool/resource/prompt handlers
- Initialization handshake

#### Session Management
- Session persistence (`.quickhorse/sessions/`)
- Session restoration
- SessionMetadata tracking

#### Configuration
- Config management (`~/.quickhorse/config.toml`)
- Environment variable support for API keys
- Dynamic provider/model switching

#### Tests
- TextEditor tests (13 tests)
- CommandRegistry tests (8 tests)
- Config tests (13 tests)
- Tools tests (9 tests)
- Streaming tests

### Security
- Basic tool permission categories (Read, Write, Network, Bash)
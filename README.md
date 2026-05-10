# QuickHorse

A fast, lightweight coding-agent CLI written in Rust.

## Features

- **Multiple LLM providers** - OpenAI, Anthropic (Claude), Gemini, Ollama
- **MCP (Model Context Protocol)** - Server and client implementation
- **Tool execution** - Bash, File Read/Edit, Glob, Grep, WebFetch
- **Session management** - Persistent sessions with JSON storage
- **Single binary deployment** - Zero dependencies, musl static compilation
- **Low memory footprint** - 10-50MB runtime memory
- **Cross-platform** - Linux, macOS, ARM, embedded devices

## Install

### From Source

```bash
# Clone the repository
git clone https://github.com/quick-hope/quickhorse.git
cd quickhorse

# Build release binary
cargo build --release

# The binary will be at target/release/quickhorse
# Copy to your PATH
cp target/release/quickhorse ~/.local/bin/
```

### Pre-built Binaries

Download from GitHub Releases (coming soon):

```bash
# Linux (x86_64, musl static)
curl -L https://github.com/quick-hope/quickhorse/releases/latest/download/quickhorse-linux-x86_64 -o quickhorse
chmod +x quickhorse
sudo mv quickhorse /usr/local/bin/

# macOS (x86_64)
curl -L https://github.com/quick-hope/quickhorse/releases/latest/download/quickhorse-macos-x86_64 -o quickhorse
chmod +x quickhorse
sudo mv quickhorse /usr/local/bin/

# macOS (ARM)
curl -L https://github.com/quick-hope/quickhorse/releases/latest/download/quickhorse-macos-aarch64 -o quickhorse
chmod +x quickhorse
sudo mv quickhorse /usr/local/bin/
```

### Cross-compile for Embedded/Linux Legacy

```bash
# Add musl target (for CentOS 5+, Alpine, embedded)
rustup target add x86_64-unknown-linux-musl

# Build static binary
cargo build --target x86_64-unknown-linux-musl --release

# ARM embedded
rustup target add aarch64-unknown-linux-gnu
cargo build --target aarch64-unknown-linux-gnu --release
```

## Usage

```bash
# Start interactive session
quickhorse

# With specific provider
quickhorse --provider openai --model gpt-4o

# With Ollama (local)
quickhorse --provider ollama --model llama3

# Resume previous session
quickhorse --session <session-id>
```

## Providers

| Provider | Models | API Key |
|----------|--------|---------|
| OpenAI | GPT-4, GPT-4o, GPT-3.5-turbo | `OPENAI_API_KEY` |
| Anthropic | Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Haiku | `ANTHROPIC_API_KEY` |
| Gemini | Gemini 1.5 Pro, Gemini 1.5 Flash, Gemini 2.0 | `GEMINI_API_KEY` |
| Ollama | Llama3, Mistral, Qwen2, DeepSeek-Coder | Local (no key) |

### Configuration

Set environment variables or create config file:

```bash
# Environment variables
export OPENAI_API_KEY="sk-..."
export ANTHROPIC_API_KEY="sk-ant-..."
export GEMINI_API_KEY="..."

# Config file (~/.quickhorse/config.toml)
[default]
provider = "openai"
model = "gpt-4o"

[providers.openai]
api_key = "sk-..."
base_url = "https://api.openai.com/v1"

[providers.anthropic]
api_key = "sk-ant-..."

[providers.ollama]
base_url = "http://localhost:11434"
```

## Tools

| Tool | Description |
|------|-------------|
| Bash | Execute shell commands |
| Read | Read file contents |
| Edit | Edit files (find/replace) |
| Glob | Find files by pattern |
| Grep | Search file contents (regex) |
| WebFetch | Fetch web content |

## MCP (Model Context Protocol)

QuickHorse implements MCP for tool/resource integration:

```bash
# Run as MCP server
quickhorse mcp-server

# Connect to MCP server
quickhorse mcp-client --command "node mcp-server.js"
```

## Session Management

Sessions are stored in `.quickhorse/sessions/`:

```bash
# List sessions
quickhorse sessions list

# Resume session
quickhorse sessions resume <id>

# Delete session
quickhorse sessions delete <id>
```

## Development

```bash
# Run tests
cargo test

# Check compilation
cargo check

# Build release
cargo build --release

# Run with logs
RUST_LOG=debug cargo run
```

## Architecture

```
quickhorse/
├── src/
│   ├── main.rs          # CLI entry
│   ├── agent/           # Agent core
│   ├── provider/        # LLM providers
│   │   ├── openai.rs
│   │   ├── anthropic.rs
│   │   ├── gemini.rs
│   │   └── ollama.rs
│   ├── tools/           # Tool implementations
│   │   ├── bash.rs
│   │   ├── file_read.rs
│   │   ├── file_edit.rs
│   │   ├── glob.rs
│   │   ├── grep.rs
│   │   └── web_fetch.rs
│   ├── mcp/             # MCP protocol
│   │   ├── protocol.rs
│   │   ├── server.rs
│   │   └── client.rs
│   ├── session/         # Session management
│   └── config/          # Configuration
├── Cargo.toml
└── README.md
```

## License

MIT
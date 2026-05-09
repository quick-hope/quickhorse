//! QuickHorse - A CLI coding agent written in Rust
//!
//! Features:
//! - Multiple LLM providers (OpenAI, Anthropic, Gemini, Ollama)
//! - Tools (Bash, FileRead, FileWrite, Grep, Glob, WebFetch)
//! - MCP protocol support
//! - Session management

mod agent;
mod config;
mod mcp;
mod provider;
mod session;
mod tools;
mod tui;

use agent::{Agent, AgentConfig};
use clap::Parser;
use config::Config;
use provider::{Message, OpenAIProvider, Provider};
use std::sync::Arc;
use std::time::Duration;
use tui::{App, EventHandler, Event, init_terminal, render, restore_terminal};

#[derive(Parser, Debug)]
#[command(name = "quickhorse")]
#[command(about = "A CLI coding agent", long_about = None)]
struct Args {
    /// Provider to use (openai, anthropic, gemini, ollama)
    #[arg(short, long, default_value = "openai")]
    provider: String,

    /// Model to use
    #[arg(short = 'm', long)]
    model: Option<String>,

    /// API key (or use env variable)
    #[arg(short, long)]
    api_key: Option<String>,

    /// Custom base URL for API (for BaiLian, DeepSeek, etc.)
    #[arg(long)]
    base_url: Option<String>,

    /// Run in non-interactive mode (just print version)
    #[arg(short = 'V', long)]
    version: bool,

    /// Print available tools
    #[arg(long)]
    list_tools: bool,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    if args.version {
        println!("QuickHorse v0.1.0");
        return Ok(());
    }

    // List tools mode
    if args.list_tools {
        let registry = tools::ToolRegistry::with_default_tools();
        println!("Available tools:");
        for tool in registry.all() {
            println!("  - {}: {}", tool.name(), tool.description());
        }
        return Ok(());
    }

    // Load configuration
    let config = Config::load().unwrap_or_else(|e| {
        eprintln!("Warning: Could not load config: {}", e);
        Config::default()
    });

    // Determine provider and model
    let provider_name = args.provider;
    let model = args.model.unwrap_or_else(|| config.get_model(&provider_name));
    let api_key = args.api_key.or_else(|| config.get_api_key(&provider_name));
    let base_url = args.base_url.or_else(|| config.get_base_url());

    // Initialize provider
    let provider: Arc<dyn Provider> = match api_key {
        Some(key) => {
            match base_url {
                Some(url) => Arc::new(OpenAIProvider::new_with_base_url(key, model.clone(), url)),
                None => Arc::new(OpenAIProvider::new(key, model.clone())),
            }
        }
        None => {
            eprintln!("Error: No API key provided. Set OPENAI_API_KEY environment variable or use --api-key");
            std::process::exit(1);
        }
    };

    // System prompt
    let system_prompt = config.agent.system_prompt.clone().unwrap_or_else(|| {
        AgentConfig::default().system_prompt.clone()
    });

    // Run TUI
    run_tui(provider, system_prompt)?;

    Ok(())
}

fn run_tui(provider: Arc<dyn Provider>, system_prompt: String) -> anyhow::Result<()> {
    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create app and event handler
    let mut app = App::new();
    let events = EventHandler::new(Duration::from_millis(250));

    // Create agent with tool support
    let agent_config = AgentConfig {
        max_iterations: 10,
        system_prompt: system_prompt.clone(),
    };
    let mut agent = Agent::new(provider, agent_config);

    // Add system message
    app.messages.push(Message::system(system_prompt));
    app.messages.push(Message::assistant(
        "Hello! I'm QuickHorse, your CLI coding assistant with tool support.\nI can execute commands, read files, and help with programming tasks.\nHow can I help you today?".to_string()
    ));

    // Main loop
    loop {
        // Draw UI
        terminal.draw(|f| render(f, &app))?;

        // Handle events
        if let Ok(event) = events.recv() {
            match event {
                Event::Key(key) => {
                    app.handle_key(key);
                }
                Event::Resize(_, _) => {
                    // Terminal resized, will be handled on next draw
                }
                Event::Mouse(_) => {
                    // Mouse events not handled
                }
                Event::Tick => {
                    // Check if we need to process a pending message
                    if app.is_loading {
                        // Get user input
                        let last_user_idx = app.messages.iter().rposition(|m| m.role == "user");
                        let last_assistant_idx = app.messages.iter().rposition(|m| m.role == "assistant");

                        let needs_response = match (last_user_idx, last_assistant_idx) {
                            (Some(user_idx), Some(assistant_idx)) => user_idx > assistant_idx,
                            (Some(_), None) => true,
                            (None, _) => false,
                        };

                        if needs_response {
                            // Get the last user message text
                            let user_text = if let Some(idx) = last_user_idx {
                                app.messages[idx].text_content()
                            } else {
                                String::new()
                            };

                            // Sync agent messages with app messages
                            agent.clear();
                            for msg in &app.messages {
                                agent.add_message(msg.clone());
                            }

                            // Process with agent (includes tool loop)
                            let response = tokio::runtime::Handle::current().block_on(async {
                                // Skip re-processing, just get response for new input
                                agent.process(user_text).await
                            });

                            match response {
                                Ok(_content) => {
                                    // Add the conversation history from agent
                                    app.messages = agent.messages().to_vec();
                                    app.is_loading = false;
                                }
                                Err(e) => {
                                    app.add_assistant_message(format!("Error: {}", e));
                                }
                            }
                        }
                    }
                }
            }
        }

        if app.should_quit {
            break;
        }
    }

    // Restore terminal
    restore_terminal()?;

    Ok(())
}
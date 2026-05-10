//! QuickHorse - A CLI coding agent written in Rust
//!
//! Features:
//! - Multiple LLM providers (OpenAI, Anthropic, Gemini, Ollama)
//! - Tools (Bash, FileRead, FileWrite, Grep, Glob, WebFetch)
//! - MCP protocol support
//! - Session management
//! - Slash commands

mod agent;
mod commands;
mod config;
mod mcp;
mod provider;
mod session;
mod tools;
mod tui;

use agent::{Agent, AgentConfig};
use clap::Parser;
use config::{Config, ConfigState, SetupWizard};
use provider::{Message, OpenAIProvider, Provider};
use std::sync::{Arc, RwLock};
use std::time::Duration;
use tui::{EventHandler, Event, init_terminal, render, restore_terminal};

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

    /// Run setup wizard to configure QuickHorse
    #[arg(long)]
    setup: bool,

    /// Show current configuration
    #[arg(long)]
    config_show: bool,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Version flag
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

    // Show configuration mode
    if args.config_show {
        match Config::load_existing() {
            Ok(config) => SetupWizard::show_config(&config),
            Err(e) => {
                println!("Configuration not found or invalid: {}", e);
                println!("Run 'quickhorse --setup' to create configuration.");
            }
        }
        return Ok(());
    }

    // Setup wizard mode
    if args.setup {
        let config = SetupWizard::run()?;
        run_tui_with_args(args, config)?;
        return Ok(());
    }

    // Check configuration state for first-run detection
    let config_state = Config::check_state();

    match config_state {
        ConfigState::NotExists => {
            // First run - no configuration exists
            println!("QuickHorse v0.1.0 - First Run Setup");
            println!();
            println!("No configuration found. Running setup wizard...");
            println!();
            let config = SetupWizard::run()?;
            run_tui_with_args(args, config)?;
        }
        ConfigState::NoApiKey => {
            // Configuration exists but no API key for default provider
            println!("QuickHorse v0.1.0");
            println!();
            println!("Configuration found but no API key for default provider.");
            println!();
            println!("Options:");
            println!("  1) Run setup wizard:    quickhorse --setup");
            println!("  2) Set environment var: OPENAI_API_KEY=xxx quickhorse");
            println!("  3) Edit config file:    ~/.quickhorse/config.toml");
            println!();

            // Try to load config and show which provider needs key
            if let Ok(config) = Config::load_existing() {
                println!("Default provider '{}' requires an API key.", config.default_provider);
                println!();

                // For Ollama, no key is needed
                if config.default_provider == "ollama" {
                    println!("Starting with Ollama (no API key required)...");
                    run_tui_with_args(args, config)?;
                    return Ok(());
                }
            }

            // Prompt to run setup
            println!("Press Enter to run setup wizard, or Ctrl+C to exit.");
            let mut input = String::new();
            std::io::stdin().read_line(&mut input).ok();

            let config = SetupWizard::run()?;
            run_tui_with_args(args, config)?;
        }
        ConfigState::Ready => {
            // Configuration complete - normal startup
            let config = Config::load()?;
            run_tui_with_args(args, config)?;
        }
    }

    Ok(())
}

/// Run TUI with arguments and loaded configuration
fn run_tui_with_args(args: Args, config: Config) -> anyhow::Result<()> {
    // Determine provider and model from args or config
    let provider_name = args.provider;
    let model = args.model.unwrap_or_else(|| config.get_model(&provider_name));
    let api_key = args.api_key.or_else(|| config.get_api_key(&provider_name));
    let base_url = args.base_url.or_else(|| config.get_base_url());

    // System prompt from config
    let system_prompt = config.agent.system_prompt.clone().unwrap_or_else(|| {
        AgentConfig::default().system_prompt.clone()
    });

    // Initialize provider
    let provider: Arc<RwLock<dyn Provider>> = match provider_name.as_str() {
        "ollama" => {
            // Ollama doesn't need API key
            let url = base_url.unwrap_or_else(|| config.providers.ollama.url.clone());
            Arc::new(RwLock::new(provider::OllamaProvider::new_with_base_url(model.clone(), url)))
        }
        _ => {
            // Other providers need API key
            match api_key {
                Some(key) => {
                    match base_url {
                        Some(url) => Arc::new(RwLock::new(OpenAIProvider::new_with_base_url(key, model.clone(), url))),
                        None => Arc::new(RwLock::new(OpenAIProvider::new(key, model.clone()))),
                    }
                }
                None => {
                    eprintln!("Error: No API key for provider '{}'.", provider_name);
                    eprintln!("Set {}_API_KEY environment variable or run 'quickhorse --setup'.",
                        provider_name.to_uppercase());
                    std::process::exit(1);
                }
            }
        }
    };

    run_tui(provider, config, system_prompt)?;
    Ok(())
}

fn run_tui(provider: Arc<RwLock<dyn Provider>>, config: Config, system_prompt: String) -> anyhow::Result<()> {
    // Create tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;

    // Initialize terminal
    let mut terminal = init_terminal()?;

    // Create app with provider and event handler
    let mut app = tui::App::with_provider(provider.clone(), config);
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
        "Hello! I'm QuickHorse, your CLI coding assistant with tool support.\n\
         I can execute commands, read files, and help with programming tasks.\n\
         Type /help to see available commands.\n\
         How can I help you today?".to_string()
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
                            // Sync agent messages with app messages (user message already added)
                            agent.clear();
                            for msg in &app.messages {
                                agent.add_message(msg.clone());
                            }

                            // Process with agent (includes tool loop) using runtime
                            // Don't add user message again - use process_last_user_message
                            let response = rt.block_on(async {
                                agent.process_last_user_message().await
                            });

                            match response {
                                Ok(_content) => {
                                    // Sync conversation history from agent back to app
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
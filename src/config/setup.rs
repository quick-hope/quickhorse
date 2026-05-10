//! Setup wizard for first-run configuration

use crate::config::{Config, ConfigError};
use std::io::{self, Write};

/// Setup wizard for interactive configuration
pub struct SetupWizard;

impl SetupWizard {
    /// Run the setup wizard interactively
    pub fn run() -> Result<Config, SetupError> {
        println!();
        println!("╔══════════════════════════════════════════════════════════╗");
        println!("║       QuickHorse - First Run Configuration               ║");
        println!("╚══════════════════════════════════════════════════════════╝");
        println!();
        println!("Welcome to QuickHorse! Let's set up your configuration.");
        println!();

        // 1. Select Provider
        let provider = Self::select_provider()?;

        // 2. Input API Key
        let api_key = Self::input_api_key(&provider)?;

        // 3. Input Base URL (for OpenAI-compatible providers)
        let base_url = if provider == "openai" {
            Self::input_base_url()?
        } else if provider == "ollama" {
            Self::input_ollama_url()?
        } else {
            None
        };

        // 4. Select Model
        let model = Self::select_model(&provider)?;

        // 5. Build and save configuration
        let config = Self::build_config(provider, api_key, base_url, model);

        // Save configuration
        config.save().map_err(SetupError::Save)?;

        println!();
        println!("✓ Configuration saved to ~/.quickhorse/config.toml");
        println!();

        Ok(config)
    }

    /// Select provider from list
    fn select_provider() -> Result<String, SetupError> {
        println!("Select your default provider:");
        println!();
        println!("  1) OpenAI / BaiLian / DeepSeek / other compatible APIs");
        println!("  2) Anthropic (Claude)");
        println!("  3) Gemini (Google AI)");
        println!("  4) Ollama (Local models - no API key required)");
        println!();

        loop {
            print!("Enter selection [1-4]: ");
            io::stdout().flush().map_err(SetupError::IO)?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

            match input.trim() {
                "1" => return Ok("openai".to_string()),
                "2" => return Ok("anthropic".to_string()),
                "3" => return Ok("gemini".to_string()),
                "4" => return Ok("ollama".to_string()),
                _ => println!("Invalid selection. Please enter 1, 2, 3, or 4."),
            }
        }
    }

    /// Input API key for provider
    fn input_api_key(provider: &str) -> Result<Option<String>, SetupError> {
        if provider == "ollama" {
            println!();
            println!("Ollama runs locally and doesn't require an API key.");
            return Ok(None);
        }

        println!();
        println!("Enter API key for {}:", provider);
        println!("(Leave empty to use environment variable {}_API_KEY)", provider.to_uppercase());
        println!();

        loop {
            print!("API key: ");
            io::stdout().flush().map_err(SetupError::IO)?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

            let key = input.trim();
            if key.is_empty() {
                println!("Using environment variable for API key.");
                return Ok(None);
            }
            // Basic validation - key should not be too short
            if key.len() < 10 {
                println!("API key seems too short. Please enter a valid key.");
                continue;
            }
            return Ok(Some(key.to_string()));
        }
    }

    /// Input base URL for OpenAI-compatible APIs
    fn input_base_url() -> Result<Option<String>, SetupError> {
        println!();
        println!("Enter base URL (optional, for alternative APIs):");
        println!();
        println!("Common base URLs:");
        println!("  • OpenAI (default):  https://api.openai.com/v1");
        println!("  • BaiLian (阿里):    https://dashscope.aliyuncs.com/compatible-mode/v1");
        println!("  • DeepSeek:          https://api.deepseek.com");
        println!("  • Moonshot (Kimi):   https://api.moonshot.cn/v1");
        println!("  • SiliconCloud:      https://api.siliconflow.cn/v1");
        println!("  • Leave empty for default OpenAI API");
        println!();

        print!("Base URL: ");
        io::stdout().flush().map_err(SetupError::IO)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

        let url = input.trim();
        if url.is_empty() {
            Ok(None)
        } else {
            // Basic URL validation
            if url.starts_with("http://") || url.starts_with("https://") {
                Ok(Some(url.to_string()))
            } else {
                println!("URL should start with http:// or https://");
                Self::input_base_url()
            }
        }
    }

    /// Input Ollama server URL
    fn input_ollama_url() -> Result<Option<String>, SetupError> {
        println!();
        println!("Enter Ollama server URL:");
        println!("(Default: http://localhost:11434)");
        println!();

        print!("Ollama URL: ");
        io::stdout().flush().map_err(SetupError::IO)?;

        let mut input = String::new();
        io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

        let url = input.trim();
        if url.is_empty() {
            Ok(Some("http://localhost:11434".to_string()))
        } else {
            Ok(Some(url.to_string()))
        }
    }

    /// Select model for provider
    fn select_model(provider: &str) -> Result<String, SetupError> {
        let models = Self::get_models_for_provider(provider);

        println!();
        println!("Select default model for {}:", provider);
        println!();

        for (i, model) in models.iter().enumerate() {
            println!("  {}) {}", i + 1, model);
        }
        println!("  {}) Custom model name", models.len() + 1);
        println!();

        loop {
            print!("Enter selection [1-{}]: ", models.len() + 1);
            io::stdout().flush().map_err(SetupError::IO)?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

            let idx: usize = input.trim().parse().ok().unwrap_or(0);

            if idx > 0 && idx <= models.len() {
                // Extract model name (remove description in parentheses)
                let model = models[idx - 1];
                let model_name = model.split(' ').next().unwrap_or(model);
                return Ok(model_name.to_string());
            } else if idx == models.len() + 1 {
                return Self::input_custom_model();
            } else {
                println!("Invalid selection. Please enter 1-{}.", models.len() + 1);
            }
        }
    }

    /// Get available models for a provider
    fn get_models_for_provider(provider: &str) -> Vec<&'static str> {
        match provider {
            "openai" => vec![
                "gpt-4 (Recommended for complex tasks)",
                "gpt-4-turbo (Faster, cheaper GPT-4)",
                "gpt-3.5-turbo (Fast, economical)",
                "qwen-plus (BaiLian/通义千问)",
                "qwen-turbo (BaiLian/通义千问-快速)",
                "deepseek-chat (DeepSeek)",
                "deepseek-coder (DeepSeek-代码专用)",
            ],
            "anthropic" => vec![
                "claude-3-5-sonnet-20241022 (Recommended)",
                "claude-3-opus-20240229 (Most capable)",
                "claude-3-haiku-20240307 (Fastest)",
            ],
            "gemini" => vec![
                "gemini-1.5-pro (Recommended)",
                "gemini-1.5-flash (Faster)",
                "gemini-2.0-flash-exp (Latest)",
            ],
            "ollama" => vec![
                "llama3 (Recommended)",
                "llama2",
                "mistral",
                "codellama",
                "qwen2",
            ],
            _ => vec!["gpt-4"],
        }
    }

    /// Input custom model name
    fn input_custom_model() -> Result<String, SetupError> {
        println!();
        println!("Enter custom model name:");
        println!();

        loop {
            print!("Model name: ");
            io::stdout().flush().map_err(SetupError::IO)?;

            let mut input = String::new();
            io::stdin().read_line(&mut input).map_err(SetupError::IO)?;

            let model = input.trim();
            if model.is_empty() {
                println!("Model name cannot be empty.");
                continue;
            }
            return Ok(model.to_string());
        }
    }

    /// Build configuration from inputs
    fn build_config(
        provider: String,
        api_key: Option<String>,
        base_url: Option<String>,
        model: String,
    ) -> Config {
        let mut config = Config::default();
        config.set_default_provider(provider.clone());
        config.set_model(&provider, model.clone());

        if let Some(key) = api_key {
            config.set_api_key(&provider, key);
        }

        if provider == "openai" {
            if let Some(url) = base_url {
                config.set_base_url(url);
            }
        } else if provider == "ollama" {
            if let Some(url) = base_url {
                config.providers.ollama.url = url;
            }
        }

        config
    }

    /// Show current configuration
    pub fn show_config(config: &Config) {
        println!();
        println!("Current Configuration (~/.quickhorse/config.toml):");
        println!();
        println!("  Default Provider: {}", config.default_provider);
        println!();

        match config.default_provider.as_str() {
            "openai" => {
                println!("  API Key: {}",
                    config.providers.openai.api_key.as_ref()
                        .map(|k| Self::mask_key(k))
                        .unwrap_or_else(|| "Not set (using env var)".to_string())
                );
                println!("  Base URL: {}",
                    config.providers.openai.base_url.as_ref()
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| "Default (OpenAI)".to_string())
                );
                println!("  Model: {}", config.providers.openai.model);
            }
            "anthropic" => {
                println!("  API Key: {}",
                    config.providers.anthropic.api_key.as_ref()
                        .map(|k| Self::mask_key(k))
                        .unwrap_or_else(|| "Not set (using env var)".to_string())
                );
                println!("  Model: {}", config.providers.anthropic.model);
            }
            "gemini" => {
                println!("  API Key: {}",
                    config.providers.gemini.api_key.as_ref()
                        .map(|k| Self::mask_key(k))
                        .unwrap_or_else(|| "Not set (using env var)".to_string())
                );
                println!("  Model: {}", config.providers.gemini.model);
            }
            "ollama" => {
                println!("  URL: {}", config.providers.ollama.url);
                println!("  Model: {}", config.providers.ollama.model);
            }
            _ => {}
        }
        println!();
    }

    /// Mask API key for display (show first 8 and last 4 chars)
    fn mask_key(key: &str) -> String {
        if key.len() <= 12 {
            "****".to_string()
        } else {
            format!("{}...{}", &key[..8], &key[key.len()-4..])
        }
    }
}

/// Setup wizard errors
#[derive(Debug)]
pub enum SetupError {
    /// IO error (reading input)
    IO(io::Error),
    /// Failed to save configuration
    Save(ConfigError),
}

impl std::fmt::Display for SetupError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SetupError::IO(e) => write!(f, "IO error: {}", e),
            SetupError::Save(e) => write!(f, "Failed to save configuration: {}", e),
        }
    }
}

impl std::error::Error for SetupError {}
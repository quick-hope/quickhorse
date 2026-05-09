//! Configuration management module

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Main configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Default provider to use
    pub default_provider: String,
    /// Provider configurations
    pub providers: Providers,
    /// Agent settings
    #[serde(default)]
    pub agent: AgentConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            providers: Providers::default(),
            agent: AgentConfig::default(),
        }
    }
}

/// Provider-specific configurations
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Providers {
    /// OpenAI configuration
    #[serde(default)]
    pub openai: OpenAIConfig,
    /// Anthropic configuration
    #[serde(default)]
    pub anthropic: AnthropicConfig,
    /// Gemini configuration
    #[serde(default)]
    pub gemini: GeminiConfig,
    /// Ollama configuration
    #[serde(default)]
    pub ollama: OllamaConfig,
}

impl Default for Providers {
    fn default() -> Self {
        Self {
            openai: OpenAIConfig::default(),
            anthropic: AnthropicConfig::default(),
            gemini: GeminiConfig::default(),
            ollama: OllamaConfig::default(),
        }
    }
}

/// OpenAI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenAIConfig {
    /// API key (or use OPENAI_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Default model
    #[serde(default = "default_openai_model")]
    pub model: String,
    /// Custom base URL (or use OPENAI_BASE_URL env var)
    /// Useful for alternative OpenAI-compatible APIs like BaiLian, DeepSeek, etc.
    #[serde(default)]
    pub base_url: Option<String>,
}

fn default_openai_model() -> String {
    "gpt-4".to_string()
}

impl Default for OpenAIConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_openai_model(),
            base_url: None,
        }
    }
}

/// Anthropic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnthropicConfig {
    /// API key (or use ANTHROPIC_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Default model
    #[serde(default = "default_anthropic_model")]
    pub model: String,
}

fn default_anthropic_model() -> String {
    "claude-3-opus-20240229".to_string()
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_anthropic_model(),
        }
    }
}

/// Gemini configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    /// API key (or use GEMINI_API_KEY env var)
    #[serde(default)]
    pub api_key: Option<String>,
    /// Default model
    #[serde(default = "default_gemini_model")]
    pub model: String,
}

fn default_gemini_model() -> String {
    "gemini-pro".to_string()
}

impl Default for GeminiConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_gemini_model(),
        }
    }
}

/// Ollama configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OllamaConfig {
    /// Ollama server URL
    #[serde(default = "default_ollama_url")]
    pub url: String,
    /// Default model
    #[serde(default = "default_ollama_model")]
    pub model: String,
}

fn default_ollama_url() -> String {
    "http://localhost:11434".to_string()
}

fn default_ollama_model() -> String {
    "llama2".to_string()
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            url: default_ollama_url(),
            model: default_ollama_model(),
        }
    }
}

/// Agent configuration
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AgentConfig {
    /// System prompt
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Maximum tokens in response
    #[serde(default)]
    pub max_tokens: Option<u32>,
    /// Temperature
    #[serde(default)]
    pub temperature: Option<f32>,
}

impl Config {
    /// Load configuration from file, creating default if not exists
    pub fn load() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            let config = Self::default();
            config.save()?;
            return Ok(config);
        }

        let content = fs::read_to_string(&config_path)
            .map_err(|e| ConfigError::Read(config_path.clone(), e))?;

        toml::from_str(&content).map_err(ConfigError::Parse)
    }

    /// Save configuration to file
    pub fn save(&self) -> Result<(), ConfigError> {
        let config_path = Self::config_path()?;

        // Ensure parent directory exists
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent).map_err(|e| ConfigError::Create(parent.to_path_buf(), e))?;
        }

        let content =
            toml::to_string_pretty(self).map_err(|e| ConfigError::Serialize(Box::new(e)))?;

        fs::write(&config_path, content).map_err(|e| ConfigError::Write(config_path, e))
    }

    /// Get the configuration file path (~/.quickhorse/config.toml)
    pub fn config_path() -> Result<PathBuf, ConfigError> {
        let home = env::var("HOME")
            .or_else(|_| env::var("USERPROFILE"))
            .map_err(|_| ConfigError::HomeNotFound)?;

        Ok(PathBuf::from(home).join(".quickhorse").join("config.toml"))
    }

    /// Get API key for a provider (from config or env)
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        match provider {
            "openai" => self.providers.openai.api_key.clone().or_else(|| {
                env::var("OPENAI_API_KEY").ok()
            }),
            "anthropic" => self.providers.anthropic.api_key.clone().or_else(|| {
                env::var("ANTHROPIC_API_KEY").ok()
            }),
            "gemini" => self.providers.gemini.api_key.clone().or_else(|| {
                env::var("GEMINI_API_KEY").ok()
            }),
            _ => None,
        }
    }

    /// Get model for a provider
    pub fn get_model(&self, provider: &str) -> String {
        match provider {
            "openai" => self.providers.openai.model.clone(),
            "anthropic" => self.providers.anthropic.model.clone(),
            "gemini" => self.providers.gemini.model.clone(),
            "ollama" => self.providers.ollama.model.clone(),
            _ => "gpt-4".to_string(),
        }
    }

    /// Get base URL for OpenAI provider (from config or env)
    pub fn get_base_url(&self) -> Option<String> {
        self.providers.openai.base_url.clone().or_else(|| {
            env::var("OPENAI_BASE_URL").ok()
        })
    }
}

/// Configuration errors
#[derive(Debug)]
pub enum ConfigError {
    /// Home directory not found
    HomeNotFound,
    /// Failed to create directory
    Create(PathBuf, std::io::Error),
    /// Failed to read config file
    Read(PathBuf, std::io::Error),
    /// Failed to write config file
    Write(PathBuf, std::io::Error),
    /// Failed to parse config
    Parse(toml::de::Error),
    /// Failed to serialize config
    Serialize(Box<dyn std::error::Error>),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::HomeNotFound => write!(f, "Could not find home directory"),
            ConfigError::Create(path, e) => {
                write!(f, "Failed to create directory {}: {}", path.display(), e)
            }
            ConfigError::Read(path, e) => {
                write!(f, "Failed to read config file {}: {}", path.display(), e)
            }
            ConfigError::Write(path, e) => {
                write!(f, "Failed to write config file {}: {}", path.display(), e)
            }
            ConfigError::Parse(e) => write!(f, "Failed to parse config: {}", e),
            ConfigError::Serialize(e) => write!(f, "Failed to serialize config: {}", e),
        }
    }
}

impl std::error::Error for ConfigError {}
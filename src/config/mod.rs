//! Configuration management module

mod setup;

pub use setup::SetupWizard;
use crate::permissions::{PermissionConfig, PermissionUpdate, RuleBehavior, RuleSource, RuleValue};
use crate::secure_storage::{get_secure_storage, SecureStorageData};

use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::PathBuf;

/// Configuration state for first-run detection
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ConfigState {
    /// Configuration file does not exist
    NotExists,
    /// Configuration exists but no API key for default provider
    NoApiKey,
    /// Configuration complete, ready to start
    Ready,
}

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
    /// Permission settings
    #[serde(default)]
    pub permissions: PermissionConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            default_provider: "openai".to_string(),
            providers: Providers::default(),
            agent: AgentConfig::default(),
            permissions: PermissionConfig::default(),
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
    /// API key (stored in SecureStorage, not config file)
    /// This field is for backwards compatibility but should not be serialized
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Default model
    #[serde(default = "default_openai_model")]
    pub model: String,
    /// Custom base URL (or use OPENAI_BASE_URL env var)
    /// Useful for alternative OpenAI-compatible APIs like BaiLian, DeepSeek, etc.
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    /// API key (stored in SecureStorage, not config file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    /// Default model
    #[serde(default = "default_anthropic_model")]
    pub model: String,
    /// Custom base URL (for BaiLian Coding Plan, etc.)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_url: Option<String>,
}

fn default_anthropic_model() -> String {
    "claude-3-opus-20240229".to_string()
}

impl Default for AnthropicConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            model: default_anthropic_model(),
            base_url: None,
        }
    }
}

/// Gemini configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiConfig {
    /// API key (stored in SecureStorage, not config file)
    #[serde(default, skip_serializing_if = "Option::is_none")]
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
    /// Load configuration from file (creates default if not exists)
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

    /// Load configuration from file without creating default
    pub fn load_existing() -> Result<Self, ConfigError> {
        let config_path = Self::config_path()?;

        if !config_path.exists() {
            return Err(ConfigError::Read(config_path, std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Configuration file not found"
            )));
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

    /// Get API key for a provider (from secure storage, config, or env)
    /// Priority: SecureStorage > config file > env var
    pub fn get_api_key(&self, provider: &str) -> Option<String> {
        // Try secure storage first
        let storage = get_secure_storage();
        if let Ok(Some(data)) = storage.read() {
            if let Some(key) = data.api_keys.get(provider) {
                return Some(key.clone());
            }
        }

        // Fall back to config file (deprecated, but still supported)
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

    /// Check if configuration file exists
    #[allow(dead_code)]
    pub fn exists() -> bool {
        Self::config_path().map(|p| p.exists()).unwrap_or(false)
    }

    /// Check configuration state for first-run detection
    pub fn check_state() -> ConfigState {
        let config_path = match Self::config_path() {
            Ok(p) => p,
            Err(_) => return ConfigState::NotExists,
        };

        // Check if file exists (don't create default)
        if !config_path.exists() {
            return ConfigState::NotExists;
        }

        // Try to load config
        let content = match fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return ConfigState::NotExists,
        };

        let config: Config = match toml::from_str(&content) {
            Ok(c) => c,
            Err(_) => return ConfigState::NotExists,
        };

        if config.default_provider_has_api_key() {
            ConfigState::Ready
        } else {
            ConfigState::NoApiKey
        }
    }

    /// Check if default provider has API key configured
    fn default_provider_has_api_key(&self) -> bool {
        // Ollama doesn't require API key
        if self.default_provider == "ollama" {
            return true;
        }
        self.get_api_key(&self.default_provider).is_some()
    }

    /// Set API key for a provider (stores in secure storage only)
    pub fn set_api_key(&mut self, provider: &str, key: String) {
        // Store in secure storage - no fallback to config file
        let storage = get_secure_storage();
        let mut data = storage.read().ok().flatten().unwrap_or_default();
        data.api_keys.insert(provider.to_string(), key);
        if let Err(e) = storage.write(&data) {
            eprintln!("Error: Failed to store API key in secure storage: {}", e);
            eprintln!("API key for {} was not saved. Please check secure storage availability.", provider);
        }
        // Note: We intentionally do NOT store in config.api_key fields
        // API keys must only be stored in SecureStorage
    }

    /// Set model for a provider
    pub fn set_model(&mut self, provider: &str, model: String) {
        match provider {
            "openai" => self.providers.openai.model = model,
            "anthropic" => self.providers.anthropic.model = model,
            "gemini" => self.providers.gemini.model = model,
            "ollama" => self.providers.ollama.model = model,
            _ => {}
        }
    }

    /// Set base URL for OpenAI provider
    pub fn set_base_url(&mut self, url: String) {
        self.providers.openai.base_url = Some(url);
    }

    /// Set default provider
    pub fn set_default_provider(&mut self, provider: String) {
        self.default_provider = provider;
    }

    /// Apply a permission update and save to config
    pub fn apply_permission_update(&mut self, update: &PermissionUpdate) -> Result<(), ConfigError> {
        match update {
            PermissionUpdate::AddRules { destination, rules, behavior } => {
                // Only support UserSettings destination for now
                if *destination != RuleSource::UserSettings {
                    return Ok(()); // Silently ignore other destinations
                }

                for rule in rules {
                    self.apply_rule_value(rule, *behavior);
                }
            }
            PermissionUpdate::RemoveRules { destination, rules, behavior } => {
                if *destination != RuleSource::UserSettings {
                    return Ok(());
                }

                for rule in rules {
                    self.remove_rule_value(rule, *behavior);
                }
            }
        }

        // Save updated config
        self.save()?;
        Ok(())
    }

    /// Apply a single rule value to the config
    fn apply_rule_value(&mut self, rule: &RuleValue, behavior: RuleBehavior) {
        match rule.tool_name.as_str() {
            "Bash" => {
                let pattern = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.bash.allow.push(pattern),
                    RuleBehavior::Deny => self.permissions.bash.deny.push(pattern),
                    RuleBehavior::Ask => self.permissions.bash.ask.push(pattern),
                }
            }
            "Read" => {
                let path = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.read.allow.push(path),
                    RuleBehavior::Deny => self.permissions.read.deny.push(path),
                    RuleBehavior::Ask => self.permissions.read.ask.push(path),
                }
            }
            "Edit" => {
                let path = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.edit.allow.push(path),
                    RuleBehavior::Deny => self.permissions.edit.deny.push(path),
                    RuleBehavior::Ask => self.permissions.edit.ask.push(path),
                }
            }
            _ => {} // Unknown tool, ignore
        }
    }

    /// Remove a single rule value from the config
    fn remove_rule_value(&mut self, rule: &RuleValue, behavior: RuleBehavior) {
        match rule.tool_name.as_str() {
            "Bash" => {
                let pattern = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.bash.allow.retain(|p| p != &pattern),
                    RuleBehavior::Deny => self.permissions.bash.deny.retain(|p| p != &pattern),
                    RuleBehavior::Ask => self.permissions.bash.ask.retain(|p| p != &pattern),
                }
            }
            "Read" => {
                let path = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.read.allow.retain(|p| p != &path),
                    RuleBehavior::Deny => self.permissions.read.deny.retain(|p| p != &path),
                    RuleBehavior::Ask => self.permissions.read.ask.retain(|p| p != &path),
                }
            }
            "Edit" => {
                let path = rule.rule_content.clone().unwrap_or_default();
                match behavior {
                    RuleBehavior::Allow => self.permissions.edit.allow.retain(|p| p != &path),
                    RuleBehavior::Deny => self.permissions.edit.deny.retain(|p| p != &path),
                    RuleBehavior::Ask => self.permissions.edit.ask.retain(|p| p != &path),
                }
            }
            _ => {} // Unknown tool, ignore
        }
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
    Serialize(Box<dyn std::error::Error + Send + Sync>),
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
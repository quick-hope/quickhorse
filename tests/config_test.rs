//! Unit tests for Config management

use quickhorse::config::{Config, Providers, OpenAIConfig, AnthropicConfig, GeminiConfig, OllamaConfig, AgentConfig};

#[test]
fn test_config_default() {
    let config = Config::default();
    assert_eq!(config.default_provider, "openai");
    assert_eq!(config.providers.openai.model, "gpt-4");
    assert_eq!(config.providers.anthropic.model, "claude-3-opus-20240229");
    assert_eq!(config.providers.gemini.model, "gemini-pro");
    assert_eq!(config.providers.ollama.model, "llama2");
}

#[test]
fn test_config_get_model() {
    let config = Config::default();
    assert_eq!(config.get_model("openai"), "gpt-4");
    assert_eq!(config.get_model("anthropic"), "claude-3-opus-20240229");
    assert_eq!(config.get_model("gemini"), "gemini-pro");
    assert_eq!(config.get_model("ollama"), "llama2");
    // Unknown provider returns default
    assert_eq!(config.get_model("unknown"), "gpt-4");
}

#[test]
fn test_config_set_api_key() {
    let mut config = Config::default();
    config.set_api_key("openai", "sk-test123".to_string());
    // API Key is now stored in SecureStorage, verify via get_api_key()
    // Note: If SecureStorage fails, it falls back to config file
    let key = config.get_api_key("openai");
    assert!(key.is_some(), "API key should be stored somewhere");

    config.set_api_key("anthropic", "ant-test456".to_string());
    let key = config.get_api_key("anthropic");
    assert!(key.is_some(), "API key should be stored somewhere");

    config.set_api_key("gemini", "gem-test789".to_string());
    let key = config.get_api_key("gemini");
    assert!(key.is_some(), "API key should be stored somewhere");
}

#[test]
fn test_config_set_model() {
    let mut config = Config::default();
    config.set_model("openai", "gpt-4o".to_string());
    assert_eq!(config.providers.openai.model, "gpt-4o");

    config.set_model("anthropic", "claude-3-5-sonnet".to_string());
    assert_eq!(config.providers.anthropic.model, "claude-3-5-sonnet");

    config.set_model("ollama", "llama3".to_string());
    assert_eq!(config.providers.ollama.model, "llama3");
}

#[test]
fn test_config_set_base_url() {
    let mut config = Config::default();
    config.set_base_url("https://api.example.com/v1".to_string());
    assert_eq!(config.providers.openai.base_url, Some("https://api.example.com/v1".to_string()));
}

#[test]
fn test_config_set_default_provider() {
    let mut config = Config::default();
    config.set_default_provider("anthropic".to_string());
    assert_eq!(config.default_provider, "anthropic");

    config.set_default_provider("ollama".to_string());
    assert_eq!(config.default_provider, "ollama");
}

#[test]
fn test_providers_default() {
    let providers = Providers::default();
    assert!(providers.openai.api_key.is_none());
    assert!(providers.anthropic.api_key.is_none());
    assert!(providers.gemini.api_key.is_none());
    // Ollama has URL but no key
    assert_eq!(providers.ollama.url, "http://localhost:11434");
}

#[test]
fn test_openai_config_default() {
    let config = OpenAIConfig::default();
    assert!(config.api_key.is_none());
    assert_eq!(config.model, "gpt-4");
    assert!(config.base_url.is_none());
}

#[test]
fn test_anthropic_config_default() {
    let config = AnthropicConfig::default();
    assert!(config.api_key.is_none());
    assert_eq!(config.model, "claude-3-opus-20240229");
}

#[test]
fn test_gemini_config_default() {
    let config = GeminiConfig::default();
    assert!(config.api_key.is_none());
    assert_eq!(config.model, "gemini-pro");
}

#[test]
fn test_ollama_config_default() {
    let config = OllamaConfig::default();
    assert_eq!(config.url, "http://localhost:11434");
    assert_eq!(config.model, "llama2");
}

#[test]
fn test_agent_config_default() {
    let config = AgentConfig::default();
    assert!(config.system_prompt.is_none());
    assert!(config.max_tokens.is_none());
    assert!(config.temperature.is_none());
}

#[test]
fn test_config_serialize_deserialize() {
    let config = Config::default();
    let serialized = toml::to_string_pretty(&config).unwrap();

    // Check serialization contains expected fields
    assert!(serialized.contains("default_provider"));
    assert!(serialized.contains("openai"));
    assert!(serialized.contains("anthropic"));
    assert!(serialized.contains("gemini"));
    assert!(serialized.contains("ollama"));

    // Deserialize back
    let deserialized: Config = toml::from_str(&serialized).unwrap();
    assert_eq!(deserialized.default_provider, config.default_provider);
    assert_eq!(deserialized.providers.openai.model, config.providers.openai.model);
}

#[test]
fn test_config_serialize_no_api_key() {
    // Create config with default values (api_key = None)
    let config = Config::default();

    let serialized = toml::to_string_pretty(&config).unwrap();

    // Verify api_key fields are NOT in serialized output when None
    // This is the expected behavior - api_key is skipped when None
    assert!(!serialized.contains("api_key"), "api_key should not be serialized when None");

    // Verify model fields ARE in serialized output
    assert!(serialized.contains("model = \"gpt-4\""), "model should be serialized");
    assert!(serialized.contains("model = \"claude-3-opus-20240229\""), "anthropic model should be serialized");
}

#[test]
fn test_config_set_api_key_no_config_storage() {
    use tempfile::TempDir;
    use std::fs;

    // Create temp directory for config
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.toml");

    let mut config = Config::default();

    // Set API key (should go to SecureStorage only)
    config.set_api_key("openai", "sk-test-key".to_string());

    // Manually save config to temp file
    let serialized = toml::to_string_pretty(&config).unwrap();
    fs::write(&config_path, &serialized).unwrap();

    // Read back and verify no api_key in file
    let file_content = fs::read_to_string(&config_path).unwrap();
    assert!(!file_content.contains("api_key"), "api_key should not be in config file");
    assert!(!file_content.contains("sk-test-key"), "API key should not be in config file");
}
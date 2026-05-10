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
    assert_eq!(config.providers.openai.api_key, Some("sk-test123".to_string()));

    config.set_api_key("anthropic", "ant-test456".to_string());
    assert_eq!(config.providers.anthropic.api_key, Some("ant-test456".to_string()));

    config.set_api_key("gemini", "gem-test789".to_string());
    assert_eq!(config.providers.gemini.api_key, Some("gem-test789".to_string()));
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
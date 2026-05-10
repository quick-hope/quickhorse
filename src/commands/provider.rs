//! Provider Command - /provider [name]

use super::{Command, CommandContext, CommandResult};
use crate::provider::{AnthropicProvider, GeminiProvider, OllamaProvider, OpenAIProvider, Provider};
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// Provider 切换命令
pub struct ProviderCommand;

impl ProviderCommand {
    /// 创建新的 Provider 实例
    fn create_provider(name: &str, ctx: &CommandContext) -> Result<Arc<RwLock<dyn Provider>>, String> {
        let config = &ctx.config;

        let provider: Arc<RwLock<dyn Provider>> = match name.to_lowercase().as_str() {
            "openai" => {
                let api_key = config
                    .get_api_key("openai")
                    .or_else(|| std::env::var("OPENAI_API_KEY").ok())
                    .ok_or_else(|| "OPENAI_API_KEY not set".to_string())?;

                let model = config.get_model("openai");
                let base_url = config.get_base_url();

                match base_url {
                    Some(url) => Arc::new(RwLock::new(OpenAIProvider::new_with_base_url(api_key, model, url))),
                    None => Arc::new(RwLock::new(OpenAIProvider::new(api_key, model))),
                }
            }
            "anthropic" | "claude" => {
                let api_key = config
                    .get_api_key("anthropic")
                    .or_else(|| std::env::var("ANTHROPIC_API_KEY").ok())
                    .ok_or_else(|| "ANTHROPIC_API_KEY not set".to_string())?;

                let model = config.get_model("anthropic");
                Arc::new(RwLock::new(AnthropicProvider::new(api_key, model)))
            }
            "gemini" | "google" => {
                let api_key = config
                    .get_api_key("gemini")
                    .or_else(|| std::env::var("GEMINI_API_KEY").ok())
                    .ok_or_else(|| "GEMINI_API_KEY not set".to_string())?;

                let model = config.get_model("gemini");
                Arc::new(RwLock::new(GeminiProvider::new(api_key, model)))
            }
            "ollama" | "local" => {
                let model = config.get_model("ollama");
                Arc::new(RwLock::new(OllamaProvider::new(model)))
            }
            _ => return Err(format!("Unknown provider: {}. Available: openai, anthropic, gemini, ollama", name)),
        };

        Ok(provider)
    }
}

#[async_trait]
impl Command for ProviderCommand {
    fn name(&self) -> &str {
        "provider"
    }

    fn description(&self) -> String {
        "Switch or show current LLM provider".to_string()
    }

    fn usage(&self) -> String {
        "/provider [name] - Switch to openai, anthropic, gemini, or ollama".to_string()
    }

    async fn execute(&self, args: &[String], ctx: &mut CommandContext) -> CommandResult {
        match args.first() {
            Some(name) => {
                // 切换 Provider
                match Self::create_provider(name, ctx) {
                    Ok(new_provider) => {
                        ctx.provider = new_provider;
                        ctx.current_provider_name = name.clone();
                        CommandResult::provider_switched(name.clone())
                    }
                    Err(e) => CommandResult::error(e),
                }
            }
            None => {
                // 显示当前 Provider
                let current = ctx.current_provider_name.clone();
                CommandResult::output(format!("Current provider: {}", current))
            }
        }
    }
}
//! Help Command - /help

use super::{Command, CommandContext, CommandResult};
use async_trait::async_trait;

/// 帮助命令
pub struct HelpCommand;

#[async_trait]
impl Command for HelpCommand {
    fn name(&self) -> &str {
        "help"
    }

    fn description(&self) -> String {
        "Show available commands".to_string()
    }

    fn usage(&self) -> String {
        "/help - List all available slash commands".to_string()
    }

    async fn execute(&self, _args: &[String], _ctx: &mut CommandContext) -> CommandResult {
        let help_text = [
            "Available Commands:",
            "",
            "/provider [name] - Switch or show LLM provider",
            "  - openai: GPT-4, GPT-4o, GPT-3.5-turbo",
            "  - anthropic: Claude 3.5 Sonnet, Claude 3 Opus",
            "  - gemini: Gemini 1.5 Pro, Gemini 2.0",
            "  - ollama: Llama3, Mistral (local)",
            "",
            "/model [name] - Switch or show current model",
            "",
            "/help - Show this help message",
            "",
            "/clear - Clear message history",
            "",
            "/status - Show current provider, model, and session status",
            "",
            "/session [id] - List or switch sessions",
            "",
            "Tips:",
            "- Press 'i' to enter input mode",
            "- Press 'Esc' to exit input mode",
            "- Press 'q' to quit",
        ];

        CommandResult::output(help_text.join("\n"))
    }
}
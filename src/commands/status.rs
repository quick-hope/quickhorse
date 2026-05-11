//! Status Command - /status

use super::{Command, CommandContext, CommandResult};

/// 状态显示命令
pub struct StatusCommand;

impl Command for StatusCommand {
    fn name(&self) -> &str {
        "status"
    }

    fn description(&self) -> String {
        "Show current session status".to_string()
    }

    fn usage(&self) -> String {
        "/status - Display provider, model, and session info".to_string()
    }

    fn execute(&self, _args: &[String], ctx: &mut CommandContext) -> CommandResult {
        let provider = ctx.provider.read().unwrap();
        let provider_name = ctx.current_provider_name.clone();
        let model = provider.model();
        let message_count = ctx.messages.len();
        let session_id = ctx.current_session_id.as_deref().unwrap_or("none");

        let status = [
            "=== QuickHorse Status ===",
            "",
            &format!("Provider: {}", provider_name),
            &format!("Model: {}", model),
            &format!("Messages: {}", message_count),
            &format!("Session: {}", session_id),
            "",
            "Available providers: openai, anthropic, gemini, ollama",
        ];

        CommandResult::output(status.join("\n"))
    }
}
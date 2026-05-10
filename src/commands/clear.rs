//! Clear Command - /clear

use super::{Command, CommandContext, CommandResult};
use async_trait::async_trait;

/// 清屏/清除历史命令
pub struct ClearCommand;

#[async_trait]
impl Command for ClearCommand {
    fn name(&self) -> &str {
        "clear"
    }

    fn description(&self) -> String {
        "Clear message history".to_string()
    }

    fn usage(&self) -> String {
        "/clear - Clear all messages in current session".to_string()
    }

    async fn execute(&self, _args: &[String], ctx: &mut CommandContext) -> CommandResult {
        ctx.messages.clear();
        CommandResult::clear()
    }
}
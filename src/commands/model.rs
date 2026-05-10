//! Model Command - /model [name]

use super::{Command, CommandContext, CommandResult};
use async_trait::async_trait;

/// Model 切换命令
pub struct ModelCommand;

#[async_trait]
impl Command for ModelCommand {
    fn name(&self) -> &str {
        "model"
    }

    fn description(&self) -> String {
        "Switch or show current model".to_string()
    }

    fn usage(&self) -> String {
        "/model [name] - Switch to a different model".to_string()
    }

    async fn execute(&self, args: &[String], ctx: &mut CommandContext) -> CommandResult {
        match args.first() {
            Some(model_name) => {
                // 切换 Model
                let mut provider = ctx.provider.write().unwrap();
                provider.set_model(model_name.clone());
                CommandResult::output(format!("Switched to model: {}", model_name))
            }
            None => {
                // 显示当前 Model
                let provider = ctx.provider.read().unwrap();
                let model = provider.model();
                CommandResult::output(format!("Current model: {}", model))
            }
        }
    }
}
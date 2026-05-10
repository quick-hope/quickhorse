//! Command Registry - Manages all available slash commands

use super::{Command, CommandContext, CommandResult};
use std::collections::HashMap;
use std::sync::Arc;

/// 命令注册表 - 存储和管理所有命令
pub struct CommandRegistry {
    /// 命令映射表
    commands: HashMap<String, Arc<dyn Command>>,
}

impl CommandRegistry {
    /// 创建新的命令注册表（带默认命令）
    pub fn new() -> Self {
        let mut registry = Self {
            commands: HashMap::new(),
        };

        // 注册默认命令
        registry.register(Arc::new(super::ProviderCommand));
        registry.register(Arc::new(super::ModelCommand));
        registry.register(Arc::new(super::HelpCommand));
        registry.register(Arc::new(super::ClearCommand));
        registry.register(Arc::new(super::StatusCommand));
        registry.register(Arc::new(super::SessionCommand));

        registry
    }

    /// 注册一个命令
    pub fn register(&mut self, command: Arc<dyn Command>) {
        self.commands.insert(command.name().to_string(), command);
    }

    /// 获取命令
    pub fn get(&self, name: &str) -> Option<Arc<dyn Command>> {
        self.commands.get(name).cloned()
    }

    /// 检查输入是否是 slash 命令
    pub fn is_command(input: &str) -> bool {
        input.starts_with('/')
    }

    /// 解析命令输入
    pub fn parse_input(input: &str) -> Option<(String, Vec<String>)> {
        if !Self::is_command(input) {
            return None;
        }

        // 去掉 / 前缀
        let trimmed = input.trim()[1..].trim();

        if trimmed.is_empty() {
            return None;
        }

        // 分割命令名和参数
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        let cmd_name = parts.first()?.to_string();
        let args: Vec<String> = parts[1..].iter().map(|s| s.to_string()).collect();

        Some((cmd_name, args))
    }

    /// 执行命令（同步版本，用于 TUI）
    pub fn execute(&self, input: &str, ctx: &mut CommandContext) -> Option<CommandResult> {
        let (cmd_name, args) = Self::parse_input(input)?;

        let command = self.commands.get(&cmd_name)?;

        // 验证参数
        if let Err(e) = command.validate_args(&args) {
            return Some(CommandResult::error(e));
        }

        // 执行命令
        // 注意：由于 TUI 事件循环是同步的，这里使用 block_on
        let result = tokio::runtime::Handle::current().block_on(async {
            command.execute(&args, ctx).await
        });

        Some(result)
    }

    /// 获取所有命令的帮助文本
    pub fn get_all_help(&self) -> Vec<String> {
        self.commands
            .values()
            .map(|cmd| cmd.help_text())
            .collect()
    }

    /// 获取命令列表
    pub fn list_commands(&self) -> Vec<&str> {
        self.commands.keys().map(|s| s.as_str()).collect()
    }
}

impl Default for CommandRegistry {
    fn default() -> Self {
        Self::new()
    }
}
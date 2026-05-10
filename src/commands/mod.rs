//! Commands module - Slash commands for TUI

mod registry;
mod provider;
mod model;
mod help;
mod clear;
mod status;
mod session;

pub use registry::CommandRegistry;
pub use provider::ProviderCommand;
pub use model::ModelCommand;
pub use help::HelpCommand;
pub use clear::ClearCommand;
pub use status::StatusCommand;
pub use session::SessionCommand;

use crate::config::Config;
use crate::provider::{Message, Provider};
use crate::session::SessionMetadata;
use async_trait::async_trait;
use std::sync::{Arc, RwLock};

/// 命令执行上下文
pub struct CommandContext {
    /// 当前 Provider (可修改)
    pub provider: Arc<RwLock<dyn Provider>>,
    /// 配置
    pub config: Config,
    /// 消息历史
    pub messages: Vec<Message>,
    /// 会话列表
    pub sessions: Vec<SessionMetadata>,
    /// 当前会话 ID
    pub current_session_id: Option<String>,
    /// 当前 Provider 名称
    pub current_provider_name: String,
}

impl CommandContext {
    /// 创建新的命令上下文
    pub fn new(provider: Arc<RwLock<dyn Provider>>, config: Config) -> Self {
        let provider_name = provider.read().unwrap().name().to_string();
        Self {
            provider,
            config,
            messages: Vec::new(),
            sessions: Vec::new(),
            current_session_id: None,
            current_provider_name: provider_name,
        }
    }
}

/// 命令执行结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CommandResult {
    /// 输出消息（显示给用户）
    pub output: String,
    /// 是否需要清除历史
    pub clear_history: bool,
    /// 是否切换了 Provider
    pub provider_changed: bool,
    /// 新的 Provider 名称（如果切换）
    pub new_provider: Option<String>,
}

impl CommandResult {
    /// 创建简单输出结果
    pub fn output(text: String) -> Self {
        Self {
            output: text,
            clear_history: false,
            provider_changed: false,
            new_provider: None,
        }
    }

    /// 创建清除历史结果
    pub fn clear() -> Self {
        Self {
            output: "Message history cleared".to_string(),
            clear_history: true,
            provider_changed: false,
            new_provider: None,
        }
    }

    /// 创建 Provider 切换结果
    pub fn provider_switched(new_name: String) -> Self {
        Self {
            output: format!("Switched to provider: {}", new_name),
            clear_history: false,
            provider_changed: true,
            new_provider: Some(new_name),
        }
    }

    /// 创建错误结果
    pub fn error(message: String) -> Self {
        Self {
            output: format!("Error: {}", message),
            clear_history: false,
            provider_changed: false,
            new_provider: None,
        }
    }
}

/// Command trait - 所有 slash 命令的接口
#[async_trait]
#[allow(dead_code)]
pub trait Command: Send + Sync {
    /// 命令名称 (不含 /)
    fn name(&self) -> &str;

    /// 命令简短描述
    fn description(&self) -> String;

    /// 命令用法说明
    fn usage(&self) -> String;

    /// 执行命令
    async fn execute(&self, args: &[String], ctx: &mut CommandContext) -> CommandResult;

    /// 验证参数（可选）
    fn validate_args(&self, _args: &[String]) -> Result<(), String> {
        Ok(())
    }

    /// 获取帮助文本
    fn help_text(&self) -> String {
        format!("/{} - {}", self.name(), self.description())
    }
}
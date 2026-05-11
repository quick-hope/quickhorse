//! Session Command - /session [id]

use super::{Command, CommandContext, CommandResult};

/// Session 管理命令
pub struct SessionCommand;

impl Command for SessionCommand {
    fn name(&self) -> &str {
        "session"
    }

    fn description(&self) -> String {
        "List or switch sessions".to_string()
    }

    fn usage(&self) -> String {
        "/session [id] - List sessions or switch to a specific session".to_string()
    }

    fn execute(&self, args: &[String], ctx: &mut CommandContext) -> CommandResult {
        match args.first() {
            Some(session_id) => {
                // 切换到指定会话
                if session_id == "new" {
                    // 创建新会话
                    ctx.current_session_id = None;
                    ctx.messages.clear();
                    CommandResult::output("Started new session".to_string())
                } else {
                    // 切换到现有会话 (需要从持久化加载)
                    // 目前简化处理，只更新 ID
                    ctx.current_session_id = Some(session_id.clone());
                    CommandResult::output(format!("Switched to session: {}", session_id))
                }
            }
            None => {
                // 列出所有会话
                if ctx.sessions.is_empty() {
                    CommandResult::output("No saved sessions. Use /session new to start fresh.".to_string())
                } else {
                    let session_list: Vec<String> = ctx
                        .sessions
                        .iter()
                        .map(|s| {
                            let current = if ctx.current_session_id.as_deref() == Some(s.id.as_str()) {
                                " (current)"
                            } else {
                                ""
                            };
                            format!("  {} - {}{}", s.id.as_str(), s.name.as_deref().unwrap_or("unnamed"), current)
                        })
                        .collect();

                    let output = ["Saved Sessions:", ""]
                        .iter()
                        .map(|s| s.to_string())
                        .chain(session_list.into_iter())
                        .collect::<Vec<_>>();

                    CommandResult::output(output.join("\n"))
                }
            }
        }
    }
}
//! Permission system for tool execution control
//!
//! Provides:
//! - Permission modes (Default, AcceptEdits, Bypass, DontAsk)
//! - Allow/deny/ask rule configuration
//! - Bash command safety classification
//! - User confirmation flow integration

#![allow(dead_code)] // Future use: permission rules integration

mod types;
mod bash;
mod config;

pub use types::{
    PermissionMode, PermissionBehavior, PermissionRule, PermissionResult,
    PermissionUpdate, RuleSource, RuleValue, RuleBehavior, DecisionReason,
    BashSafetyLevel, SAFE_COMMANDS, READ_ONLY_COMMANDS, NETWORK_COMMANDS,
    WRITE_COMMANDS, DANGEROUS_COMMANDS, SYSTEM_COMMANDS, BLOCKED_COMMANDS,
    SENSITIVE_FILE_PATTERNS,
};
pub use bash::{BashPermissionChecker, BashPermissionConfig};
pub use config::{PermissionConfig, ReadPermissionConfig, EditPermissionConfig};

/// Permission checker trait for tool-specific implementations
pub trait PermissionChecker {
    /// Check permission for an operation
    fn check(&self, operation: &str) -> PermissionResult;

    /// Get current permission mode
    fn mode(&self) -> PermissionMode;

    /// Set permission mode
    fn set_mode(&mut self, mode: PermissionMode);
}

/// Create a permission checker with default settings
pub fn default_permission_checker() -> BashPermissionChecker {
    BashPermissionChecker::new()
}

/// Create a permission checker with configuration
pub fn permission_checker_from_config(config: &PermissionConfig) -> BashPermissionChecker {
    let mut checker = BashPermissionChecker::with_mode(config.mode);
    checker.load_rules(&config.bash);
    checker
}
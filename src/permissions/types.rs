//! Permission types and structures
//!
//! Defines the permission system for tool execution control.

#![allow(dead_code)] // Future use: permission types

use serde::{Deserialize, Serialize};

/// Permission mode - determines how permission checks are handled
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub enum PermissionMode {
    /// Default mode - dangerous operations need user confirmation
    #[default]
    Default,
    /// Auto-accept all edit operations
    AcceptEdits,
    /// Bypass all permission checks (use with caution)
    BypassPermissions,
    /// Don't ask mode - auto approve but log
    DontAsk,
}

/// Permission behavior - result of a permission check
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PermissionBehavior {
    /// Allow execution
    Allow,
    /// Deny execution
    Deny,
    /// Need user confirmation
    Ask,
}

/// Rule source - where a permission rule came from
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RuleSource {
    /// User global settings (~/.quickhorse/config.toml)
    UserSettings,
    /// Project settings (.quickhorse/project.toml)
    ProjectSettings,
    /// CLI argument
    CliArg,
    /// Session temporary rule
    Session,
}

/// Rule value - specifies which tool and optional content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuleValue {
    /// Tool name (Bash, Read, Edit, Glob, Grep, WebFetch)
    pub tool_name: String,
    /// Rule content (command pattern, path pattern, etc.)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rule_content: Option<String>,
}

/// Permission rule with source and behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionRule {
    /// Where this rule came from
    pub source: RuleSource,
    /// What behavior this rule triggers
    pub behavior: RuleBehavior,
    /// The rule value
    pub value: RuleValue,
}

/// Rule behavior for configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RuleBehavior {
    Allow,
    Deny,
    Ask,
}

impl From<RuleBehavior> for PermissionBehavior {
    fn from(b: RuleBehavior) -> Self {
        match b {
            RuleBehavior::Allow => PermissionBehavior::Allow,
            RuleBehavior::Deny => PermissionBehavior::Deny,
            RuleBehavior::Ask => PermissionBehavior::Ask,
        }
    }
}

/// Permission decision reason - why a decision was made
#[derive(Debug, Clone)]
pub enum DecisionReason {
    /// Decision from a rule
    Rule { rule: PermissionRule },
    /// Decision from mode
    Mode { mode: PermissionMode },
    /// Decision from safety classification
    Safety { level: BashSafetyLevel },
    /// Decision from path constraint
    Path { reason: String },
    /// Decision from subcommand check
    Subcommand { reasons: Vec<PermissionResult> },
    /// Other reason
    Other { reason: String },
}

/// Permission check result
#[derive(Debug, Clone)]
pub struct PermissionResult {
    /// The behavior decision
    pub behavior: PermissionBehavior,
    /// Human-readable message
    pub message: String,
    /// Why this decision was made
    pub reason: Option<DecisionReason>,
    /// Suggestions for rule updates
    pub suggestions: Vec<PermissionUpdate>,
}

impl PermissionResult {
    /// Create an allow result
    pub fn allow(message: impl Into<String>) -> Self {
        Self {
            behavior: PermissionBehavior::Allow,
            message: message.into(),
            reason: None,
            suggestions: Vec::new(),
        }
    }

    /// Create an allow result with reason
    pub fn allow_with_reason(message: impl Into<String>, reason: DecisionReason) -> Self {
        Self {
            behavior: PermissionBehavior::Allow,
            message: message.into(),
            reason: Some(reason),
            suggestions: Vec::new(),
        }
    }

    /// Create a deny result
    pub fn deny(message: impl Into<String>) -> Self {
        Self {
            behavior: PermissionBehavior::Deny,
            message: message.into(),
            reason: None,
            suggestions: Vec::new(),
        }
    }

    /// Create a deny result with reason
    pub fn deny_with_reason(message: impl Into<String>, reason: DecisionReason) -> Self {
        Self {
            behavior: PermissionBehavior::Deny,
            message: message.into(),
            reason: Some(reason),
            suggestions: Vec::new(),
        }
    }

    /// Create an ask result
    pub fn ask(message: impl Into<String>) -> Self {
        Self {
            behavior: PermissionBehavior::Ask,
            message: message.into(),
            reason: None,
            suggestions: Vec::new(),
        }
    }

    /// Create an ask result with suggestions
    pub fn ask_with_suggestions(
        message: impl Into<String>,
        suggestions: Vec<PermissionUpdate>,
    ) -> Self {
        Self {
            behavior: PermissionBehavior::Ask,
            message: message.into(),
            reason: None,
            suggestions,
        }
    }

    /// Check if permission is allowed
    pub fn is_allowed(&self) -> bool {
        self.behavior == PermissionBehavior::Allow
    }

    /// Check if permission is denied
    pub fn is_denied(&self) -> bool {
        self.behavior == PermissionBehavior::Deny
    }

    /// Check if permission needs confirmation
    pub fn needs_confirmation(&self) -> bool {
        self.behavior == PermissionBehavior::Ask
    }
}

/// Permission update operation
#[derive(Debug, Clone)]
pub enum PermissionUpdate {
    /// Add rules to a destination
    AddRules {
        destination: RuleSource,
        rules: Vec<RuleValue>,
        behavior: RuleBehavior,
    },
    /// Remove rules from a destination
    RemoveRules {
        destination: RuleSource,
        rules: Vec<RuleValue>,
        behavior: RuleBehavior,
    },
}

/// Bash command safety level
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BashSafetyLevel {
    /// Safe commands - auto approve (ls, cat, echo, pwd)
    Safe,
    /// Read-only commands - default approve (git status, find, grep)
    ReadOnly,
    /// Network commands - need confirmation (curl, wget, ssh)
    Network,
    /// Write commands - need confirmation (rm, mv, cp, mkdir)
    Write,
    /// Dangerous commands - high caution (sudo, dd, mkfs)
    Dangerous,
    /// System commands - force deny (shutdown, reboot)
    System,
}

/// Predefined safe commands that don't need confirmation
pub const SAFE_COMMANDS: &[&str] = &[
    "ls", "dir", "cat", "echo", "pwd", "date", "whoami",
    "which", "whereis", "env", "printenv", "uname",
    "hostname", "id", "groups", "tty", "true", "false",
];

/// Read-only commands that are generally safe
pub const READ_ONLY_COMMANDS: &[&str] = &[
    "git status", "git log", "git diff", "git show", "git branch",
    "git remote", "git rev-parse", "git ls-files",
    "find", "grep", "egrep", "fgrep", "rg", "ag",
    "head", "tail", "less", "more", "wc", "sort", "uniq",
    "stat", "file", "tree", "du", "df", "ls",
];

/// Network commands that need confirmation
pub const NETWORK_COMMANDS: &[&str] = &[
    "curl", "wget", "fetch", "aria2c",
    "ssh", "scp", "rsync", "sftp",
    "nc", "telnet", "ftp",
];

/// Write commands that need confirmation
pub const WRITE_COMMANDS: &[&str] = &[
    "rm", "rmdir", "mv", "cp", "mkdir", "touch",
    "chmod", "chown", "ln", "symlink",
    "tar", "unzip", "gzip", "gunzip",
    "npm install", "npm uninstall", "npm update",
    "cargo install", "cargo uninstall",
    "pip install", "pip uninstall",
    "go get", "go install",
];

/// Dangerous commands that require extra caution
pub const DANGEROUS_COMMANDS: &[&str] = &[
    "sudo", "su", "doas", "pkexec",
    "dd", "mkfs", "fdisk", "parted",
    "shutdown", "reboot", "halt", "init", "poweroff",
    "systemctl reboot", "systemctl poweroff",
];

/// System commands that are always blocked
pub const SYSTEM_COMMANDS: &[&str] = &[
    "shutdown", "reboot", "halt", "init", "poweroff",
    "systemctl reboot", "systemctl poweroff", "systemctl halt",
];

/// Always blocked command patterns (exact match or prefix)
pub const BLOCKED_COMMANDS: &[&str] = &[
    "rm -rf /", "rm -rf /*",  // Extremely dangerous
    ":(){ :|:& };:",  // Fork bomb
    "chmod -R 777 /",
    "mkfs", "fdisk",
];

/// Maximum subcommands to check in compound commands
pub const MAX_SUBCOMMANDS: usize = 20;

/// Sensitive file patterns that need extra confirmation
pub const SENSITIVE_FILE_PATTERNS: &[&str] = &[
    ".env", ".env.local", ".env.production", ".env.development",
    "id_rsa", "id_ed25519", "id_dsa", "id_ecdsa",
    ".pem", ".key", ".ssh", ".gnupg",
    ".git/config", ".git/credentials",
    "credentials.json", "secrets.json", "secrets.yaml",
    "api_keys.txt", "tokens.txt",
];
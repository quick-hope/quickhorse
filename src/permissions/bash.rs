//! Bash command permission checker
//!
//! Implements security checks for Bash tool execution.

use super::types::{
    PermissionMode, PermissionResult, PermissionRule,
    PermissionUpdate, RuleBehavior, RuleSource, RuleValue, DecisionReason,
    BashSafetyLevel, SAFE_COMMANDS, READ_ONLY_COMMANDS, NETWORK_COMMANDS,
    WRITE_COMMANDS, DANGEROUS_COMMANDS, SYSTEM_COMMANDS, BLOCKED_COMMANDS,
    MAX_SUBCOMMANDS,
};
use serde::{Deserialize, Serialize};

/// Bash permission checker
pub struct BashPermissionChecker {
    /// Current permission mode
    mode: PermissionMode,
    /// Allow rules from configuration
    allow_rules: Vec<PermissionRule>,
    /// Deny rules from configuration
    deny_rules: Vec<PermissionRule>,
    /// Ask rules from configuration
    ask_rules: Vec<PermissionRule>,
    /// Working directory constraint
    working_dir: Option<String>,
}

impl Default for BashPermissionChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl BashPermissionChecker {
    /// Create a new permission checker
    pub fn new() -> Self {
        Self {
            mode: PermissionMode::Default,
            allow_rules: Vec::new(),
            deny_rules: Vec::new(),
            ask_rules: Vec::new(),
            working_dir: None,
        }
    }

    /// Create checker with mode
    pub fn with_mode(mode: PermissionMode) -> Self {
        Self {
            mode,
            ..Self::new()
        }
    }

    /// Set permission mode
    pub fn set_mode(&mut self, mode: PermissionMode) {
        self.mode = mode;
    }

    /// Get current mode
    pub fn mode(&self) -> PermissionMode {
        self.mode
    }

    /// Set working directory constraint
    pub fn set_working_dir(&mut self, dir: String) {
        self.working_dir = Some(dir);
    }

    /// Add an allow rule
    pub fn add_allow_rule(&mut self, tool_name: String, pattern: Option<String>, source: RuleSource) {
        self.allow_rules.push(PermissionRule {
            source,
            behavior: RuleBehavior::Allow,
            value: RuleValue { tool_name, rule_content: pattern },
        });
    }

    /// Add a deny rule
    pub fn add_deny_rule(&mut self, tool_name: String, pattern: Option<String>, source: RuleSource) {
        self.deny_rules.push(PermissionRule {
            source,
            behavior: RuleBehavior::Deny,
            value: RuleValue { tool_name, rule_content: pattern },
        });
    }

    /// Add an ask rule
    pub fn add_ask_rule(&mut self, tool_name: String, pattern: Option<String>, source: RuleSource) {
        self.ask_rules.push(PermissionRule {
            source,
            behavior: RuleBehavior::Ask,
            value: RuleValue { tool_name, rule_content: pattern },
        });
    }

    /// Load rules from configuration
    pub fn load_rules(&mut self, config: &BashPermissionConfig) {
        // Load allow rules
        for pattern in &config.allow {
            self.add_allow_rule("Bash".to_string(), Some(pattern.clone()), RuleSource::UserSettings);
        }

        // Load deny rules
        for pattern in &config.deny {
            self.add_deny_rule("Bash".to_string(), Some(pattern.clone()), RuleSource::UserSettings);
        }

        // Load ask rules
        for pattern in &config.ask {
            self.add_ask_rule("Bash".to_string(), Some(pattern.clone()), RuleSource::UserSettings);
        }
    }

    /// Check permission for a command
    pub fn check(&self, command: &str) -> PermissionResult {
        // 1. Bypass mode - allow everything
        if self.mode == PermissionMode::BypassPermissions {
            return PermissionResult::allow_with_reason(
                "Bypass mode enabled",
                DecisionReason::Mode { mode: self.mode },
            );
        }

        // 2. Parse and normalize command
        let normalized = self.normalize_command(command);

        // 3. Check system commands - force deny
        if self.is_system_command(&normalized) {
            return PermissionResult::deny_with_reason(
                "System commands are blocked for safety",
                DecisionReason::Safety { level: BashSafetyLevel::System },
            );
        }

        // 4. Check blocked command patterns - force deny
        if self.is_blocked_command(&normalized) {
            return PermissionResult::deny_with_reason(
                "This command is blocked for safety reasons",
                DecisionReason::Safety { level: BashSafetyLevel::Dangerous },
            );
        }

        // 5. Check dangerous commands - ask with warning
        if self.is_dangerous_command(&normalized) {
            return PermissionResult::ask_with_suggestions(
                format!("Dangerous command '{}' requires confirmation", normalized),
                vec![PermissionUpdate::AddRules {
                    destination: RuleSource::UserSettings,
                    rules: vec![RuleValue { tool_name: "Bash".to_string(), rule_content: Some(format!("{}:*", self.get_base_command(&normalized))) }],
                    behavior: RuleBehavior::Allow,
                }],
            );
        }

        // 6. Check deny rules - highest priority
        if let Some(rule) = self.match_rule(&normalized, &self.deny_rules) {
            return PermissionResult::deny_with_reason(
                format!("Command '{}' is blocked by rule", normalized),
                DecisionReason::Rule { rule: rule.clone() },
            );
        }

        // 7. Check ask rules
        if let Some(rule) = self.match_rule(&normalized, &self.ask_rules) {
            return PermissionResult::ask_with_suggestions(
                format!("Command '{}' needs confirmation", normalized),
                vec![PermissionUpdate::AddRules {
                    destination: RuleSource::UserSettings,
                    rules: vec![rule.value.clone()],
                    behavior: RuleBehavior::Allow,
                }],
            );
        }

        // 8. Check allow rules
        if let Some(rule) = self.match_rule(&normalized, &self.allow_rules) {
            return PermissionResult::allow_with_reason(
                format!("Command '{}' is allowed by rule", normalized),
                DecisionReason::Rule { rule: rule.clone() },
            );
        }

        // 9. Check compound commands
        if self.is_compound_command(&normalized) {
            return self.check_compound_command(&normalized);
        }

        // 10. Classify by safety level
        let level = self.classify_command(&normalized);
        match level {
            BashSafetyLevel::Safe => PermissionResult::allow_with_reason(
                "Safe command",
                DecisionReason::Safety { level },
            ),
            BashSafetyLevel::ReadOnly => {
                if self.mode == PermissionMode::AcceptEdits || self.mode == PermissionMode::DontAsk {
                    PermissionResult::allow_with_reason(
                        "Read-only command in auto-accept mode",
                        DecisionReason::Safety { level },
                    )
                } else {
                    PermissionResult::allow_with_reason(
                        "Read-only command",
                        DecisionReason::Safety { level },
                    )
                }
            },
            BashSafetyLevel::Network => PermissionResult::ask_with_suggestions(
                format!("Network command '{}' requires confirmation", normalized),
                vec![PermissionUpdate::AddRules {
                    destination: RuleSource::UserSettings,
                    rules: vec![RuleValue { tool_name: "Bash".to_string(), rule_content: Some(format!("{}:*", self.get_base_command(&normalized))) }],
                    behavior: RuleBehavior::Allow,
                }],
            ),
            BashSafetyLevel::Write => PermissionResult::ask_with_suggestions(
                format!("Write command '{}' requires confirmation", normalized),
                vec![PermissionUpdate::AddRules {
                    destination: RuleSource::UserSettings,
                    rules: vec![RuleValue { tool_name: "Bash".to_string(), rule_content: Some(format!("{}:*", self.get_base_command(&normalized))) }],
                    behavior: RuleBehavior::Allow,
                }],
            ),
            BashSafetyLevel::Dangerous => PermissionResult::ask_with_suggestions(
                format!("Dangerous command '{}' requires extra confirmation", normalized),
                vec![],
            ),
            BashSafetyLevel::System => PermissionResult::deny_with_reason(
                "System command blocked",
                DecisionReason::Safety { level },
            ),
        }
    }

    /// Normalize command (strip wrappers, etc.)
    fn normalize_command(&self, command: &str) -> String {
        self.strip_safe_wrappers(command.trim())
    }

    /// Strip safe wrapper commands
    fn strip_safe_wrappers(&self, command: &str) -> String {
        // Safe wrapper commands
        const SAFE_WRAPPERS: &[&str] = &[
            "timeout", "time", "nice", "nohup", "stdbuf", "ionice",
        ];

        // Safe environment variables (won't affect execution path)
        const SAFE_ENV_VARS: &[&str] = &[
            "NODE_ENV", "RUST_BACKTRACE", "RUST_LOG",
            "LANG", "LC_ALL", "TZ", "TERM",
            "GOOS", "GOARCH", "CGO_ENABLED",
            "CI", "DEBUG", "VERBOSE",
        ];

        let mut remaining = command;

        // Strip environment variables
        loop {
            let stripped = false;
            for env_var in SAFE_ENV_VARS {
                let prefix = format!("{}=", env_var);
                if remaining.starts_with(&prefix) {
                    // Find the end of the value
                    let after_prefix = &remaining[prefix.len()..];
                    if let Some(space_idx) = after_prefix.find(' ') {
                        remaining = &after_prefix[space_idx + 1..];
                        break;
                    } else {
                        // No command after env var - return empty
                        return String::new();
                    }
                }
            }
            if !stripped {
                break;
            }
        }

        // Strip wrapper commands
        for wrapper in SAFE_WRAPPERS {
            let prefix = format!("{} ", wrapper);
            if remaining.starts_with(&prefix) {
                // Find the actual command after wrapper arguments
                let after_wrapper = &remaining[prefix.len()..];
                // Skip wrapper arguments (e.g., "timeout 10" -> "10 npm test")
                let parts: Vec<&str> = after_wrapper.split_whitespace().collect();
                if parts.len() > 1 {
                    // Assume first part after wrapper is argument, rest is command
                    // This is simplified - real implementation would parse better
                    return parts[1..].join(" ");
                } else if parts.len() == 1 {
                    return parts[0].to_string();
                }
            }
        }

        remaining.to_string()
    }

    /// Get base command (first word)
    fn get_base_command(&self, command: &str) -> String {
        command.split_whitespace().next().unwrap_or(command).to_string()
    }

    /// Check if command is a system command
    fn is_system_command(&self, command: &str) -> bool {
        let base = self.get_base_command(command);
        SYSTEM_COMMANDS.iter().any(|c| {
            base == c.split_whitespace().next().unwrap_or(c)
        }) || SYSTEM_COMMANDS.iter().any(|c| command.starts_with(c))
    }

    /// Check if command matches blocked patterns
    fn is_blocked_command(&self, command: &str) -> bool {
        BLOCKED_COMMANDS.iter().any(|pattern| {
            // Exact match
            command == *pattern ||
            // Prefix match for patterns ending with wildcard-like endings
            (pattern.contains(" -rf ") && command.starts_with(*pattern))
        })
    }

    /// Check if command is dangerous
    fn is_dangerous_command(&self, command: &str) -> bool {
        let base = self.get_base_command(command);
        DANGEROUS_COMMANDS.iter().any(|c| {
            base == c.split_whitespace().next().unwrap_or(c)
        })
    }

    /// Check if command is compound (&&, ||, |, ;)
    fn is_compound_command(&self, command: &str) -> bool {
        command.contains("&&") || command.contains("||") ||
        command.contains("|") || command.contains(";")
    }

    /// Split compound command into subcommands
    fn split_compound_command(&self, command: &str) -> Vec<String> {
        // Simplified splitting - real implementation would parse shell syntax properly
        let mut subcommands = Vec::new();

        // Split by &&, ||, |, ;
        let mut remaining = command;
        while !remaining.is_empty() {
            let separators = ["&&", "||", "|", ";"];

            let mut earliest_idx = None;
            let mut earliest_sep = "";

            for sep in separators {
                if let Some(idx) = remaining.find(sep) {
                    if earliest_idx.is_none() || idx < earliest_idx.unwrap() {
                        earliest_idx = Some(idx);
                        earliest_sep = sep;
                    }
                }
            }

            if let Some(idx) = earliest_idx {
                let subcmd = remaining[..idx].trim();
                if !subcmd.is_empty() {
                    subcommands.push(subcmd.to_string());
                }
                remaining = &remaining[idx + earliest_sep.len()..];
            } else {
                let subcmd = remaining.trim();
                if !subcmd.is_empty() {
                    subcommands.push(subcmd.to_string());
                }
                break;
            }
        }

        subcommands
    }

    /// Check compound command
    fn check_compound_command(&self, command: &str) -> PermissionResult {
        let subcommands = self.split_compound_command(command);

        // Check subcommand limit
        if subcommands.len() > MAX_SUBCOMMANDS {
            return PermissionResult::ask(format!(
                "Too many subcommands ({}) to verify safely",
                subcommands.len()
            ));
        }

        // Check for cd + git combination (bare repo risk)
        let has_cd = subcommands.iter().any(|s| s.split_whitespace().next() == Some("cd"));
        let has_git = subcommands.iter().any(|s| s.split_whitespace().next() == Some("git"));
        if has_cd && has_git {
            return PermissionResult::ask_with_suggestions(
                "cd + git combination requires confirmation (bare repo risk)",
                vec![],
            );
        }

        // Check each subcommand
        let results: Vec<PermissionResult> = subcommands.iter()
            .map(|s| self.check(s))
            .collect();

        // Any deny -> deny
        if results.iter().any(|r| r.is_denied()) {
            return PermissionResult::deny("Subcommand blocked");
        }

        // All allow -> allow
        if results.iter().all(|r| r.is_allowed()) {
            return PermissionResult::allow("All subcommands allowed");
        }

        // Otherwise ask
        PermissionResult::ask(format!(
            "Compound command has {} subcommands needing confirmation",
            results.iter().filter(|r| r.needs_confirmation()).count()
        ))
    }

    /// Classify command by safety level
    fn classify_command(&self, command: &str) -> BashSafetyLevel {
        let base = self.get_base_command(command);
        let full_command = command;

        // Check safe commands
        if SAFE_COMMANDS.contains(&base.as_str()) {
            return BashSafetyLevel::Safe;
        }

        // Check read-only commands (prefix match)
        for cmd in READ_ONLY_COMMANDS {
            if full_command.starts_with(cmd) || base == cmd.split_whitespace().next().unwrap_or(cmd) {
                return BashSafetyLevel::ReadOnly;
            }
        }

        // Check network commands
        if NETWORK_COMMANDS.contains(&base.as_str()) {
            return BashSafetyLevel::Network;
        }

        // Check write commands
        for cmd in WRITE_COMMANDS {
            if full_command.starts_with(cmd) {
                return BashSafetyLevel::Write;
            }
        }
        if WRITE_COMMANDS.contains(&base.as_str()) {
            return BashSafetyLevel::Write;
        }

        // Check dangerous commands
        if DANGEROUS_COMMANDS.contains(&base.as_str()) {
            return BashSafetyLevel::Dangerous;
        }

        // Check system commands
        if SYSTEM_COMMANDS.contains(&base.as_str()) {
            return BashSafetyLevel::System;
        }

        // Unknown command - ask for confirmation
        BashSafetyLevel::Write
    }

    /// Match command against rules
    fn match_rule<'a>(&self, command: &str, rules: &'a [PermissionRule]) -> Option<&'a PermissionRule> {
        for rule in rules {
            if rule.value.tool_name != "Bash" {
                continue;
            }

            let pattern = rule.value.rule_content.as_ref()?;
            if self.match_pattern(command, pattern) {
                return Some(rule);
            }
        }
        None
    }

    /// Match command against pattern
    fn match_pattern(&self, command: &str, pattern: &str) -> bool {
        // Prefix match: "git:*" matches "git status", "git log"
        if pattern.ends_with(":*") {
            let prefix = pattern.trim_end_matches(":*");
            let base = self.get_base_command(command);
            // Word boundary check
            return base == prefix || command.starts_with(&format!("{} ", prefix));
        }

        // Exact match
        if command == pattern {
            return true;
        }

        // Base command exact match
        let base = self.get_base_command(command);
        if base == pattern {
            return true;
        }

        false
    }
}

/// Bash permission configuration from config file
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BashPermissionConfig {
    /// Commands to always allow
    #[serde(default)]
    pub allow: Vec<String>,
    /// Commands to always deny
    #[serde(default)]
    pub deny: Vec<String>,
    /// Commands to always ask for confirmation
    #[serde(default)]
    pub ask: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_safe_commands_auto_allowed() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("ls").is_allowed());
        assert!(checker.check("pwd").is_allowed());
        assert!(checker.check("echo hello").is_allowed());
        assert!(checker.check("cat file.txt").is_allowed());
    }

    #[test]
    fn test_system_commands_blocked() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("shutdown now").is_denied());
        assert!(checker.check("reboot").is_denied());
        assert!(checker.check("halt").is_denied());
    }

    #[test]
    fn test_dangerous_commands_ask() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("sudo rm file").needs_confirmation());
        assert!(checker.check("dd if=/dev/zero").needs_confirmation());
    }

    #[test]
    fn test_bypass_mode_allows_all() {
        let checker = BashPermissionChecker::with_mode(PermissionMode::BypassPermissions);

        assert!(checker.check("sudo rm -rf /").is_allowed());
        assert!(checker.check("shutdown now").is_allowed());
    }

    #[test]
    fn test_compound_cd_git_ask() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("cd /tmp && git status").needs_confirmation());
    }

    #[test]
    fn test_prefix_rule_matching() {
        let mut checker = BashPermissionChecker::new();
        checker.add_allow_rule("Bash".to_string(), Some("git:*".to_string()), RuleSource::UserSettings);

        assert!(checker.check("git status").is_allowed());
        assert!(checker.check("git log").is_allowed());
        assert!(checker.check("git").is_allowed());
    }

    #[test]
    fn test_deny_rule_priority() {
        let mut checker = BashPermissionChecker::new();
        checker.add_allow_rule("Bash".to_string(), Some("rm:*".to_string()), RuleSource::UserSettings);
        checker.add_deny_rule("Bash".to_string(), Some("rm -rf /".to_string()), RuleSource::UserSettings);

        // rm -rf / should be denied even with rm:* allow rule
        assert!(checker.check("rm -rf /").is_denied());
        // Other rm commands should be allowed
        assert!(checker.check("rm file.txt").is_allowed());
    }

    #[test]
    fn test_wrapper_stripping() {
        let checker = BashPermissionChecker::new();

        assert_eq!(checker.strip_safe_wrappers("timeout 10 npm test"), "npm test");
        assert_eq!(checker.strip_safe_wrappers("NODE_ENV=prod node script.js"), "node script.js");
    }

    #[test]
    fn test_network_commands_ask() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("curl https://example.com").needs_confirmation());
        assert!(checker.check("wget file.zip").needs_confirmation());
    }

    #[test]
    fn test_write_commands_ask() {
        let checker = BashPermissionChecker::new();

        assert!(checker.check("rm file.txt").needs_confirmation());
        assert!(checker.check("mv old.txt new.txt").needs_confirmation());
        assert!(checker.check("mkdir newdir").needs_confirmation());
    }
}
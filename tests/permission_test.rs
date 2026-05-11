//! Permission system integration tests
//!
//! Tests cover:
//! - Permission modes and behaviors
//! - Bash command safety classification
//! - Rule matching and priority
//! - Configuration integration

use quickhorse::permissions::{
    PermissionMode, PermissionBehavior, PermissionResult,
    PermissionConfig, BashPermissionChecker, BashPermissionConfig,
    RuleSource, RuleBehavior,
};

// ============================================================================
// Permission Mode Tests
// ============================================================================

#[test]
fn test_permission_mode_default() {
    let config = PermissionConfig::new();
    assert_eq!(config.mode, PermissionMode::Default);
}

#[test]
fn test_permission_mode_bypass() {
    let config = PermissionConfig::bypass_mode();
    assert_eq!(config.mode, PermissionMode::BypassPermissions);
    assert!(config.is_bypass());
}

#[test]
fn test_permission_mode_accept_edits() {
    let config = PermissionConfig::accept_edits_mode();
    assert_eq!(config.mode, PermissionMode::AcceptEdits);
    assert!(config.is_accept_edits());
}

// ============================================================================
// Bash Permission Tests
// ============================================================================

#[test]
fn test_safe_commands_allowed() {
    let checker = BashPermissionChecker::new();

    // Safe commands should be allowed automatically
    assert!(checker.check("ls").is_allowed());
    assert!(checker.check("pwd").is_allowed());
    assert!(checker.check("echo hello").is_allowed());
    assert!(checker.check("cat file.txt").is_allowed());
    assert!(checker.check("whoami").is_allowed());
    assert!(checker.check("date").is_allowed());
}

#[test]
fn test_readonly_commands_allowed() {
    let checker = BashPermissionChecker::new();

    // Read-only commands should be allowed
    assert!(checker.check("git status").is_allowed());
    assert!(checker.check("git log").is_allowed());
    assert!(checker.check("git diff").is_allowed());
    assert!(checker.check("find . -name '*.rs'").is_allowed());
    assert!(checker.check("grep pattern file").is_allowed());
}

#[test]
fn test_system_commands_denied() {
    let checker = BashPermissionChecker::new();

    // System commands should always be denied
    assert!(checker.check("shutdown").is_denied());
    assert!(checker.check("reboot").is_denied());
    assert!(checker.check("halt").is_denied());
    assert!(checker.check("init 0").is_denied());
}

#[test]
fn test_dangerous_commands_ask() {
    let checker = BashPermissionChecker::new();

    // Dangerous commands need confirmation
    assert!(checker.check("sudo ls").needs_confirmation());
    assert!(checker.check("dd if=/dev/zero").needs_confirmation());
}

#[test]
fn test_network_commands_ask() {
    let checker = BashPermissionChecker::new();

    // Network commands need confirmation
    assert!(checker.check("curl https://example.com").needs_confirmation());
    assert!(checker.check("wget file.zip").needs_confirmation());
    assert!(checker.check("ssh user@host").needs_confirmation());
}

#[test]
fn test_write_commands_ask() {
    let checker = BashPermissionChecker::new();

    // Write commands need confirmation
    assert!(checker.check("rm file.txt").needs_confirmation());
    assert!(checker.check("mv old.txt new.txt").needs_confirmation());
    assert!(checker.check("mkdir newdir").needs_confirmation());
    assert!(checker.check("touch newfile").needs_confirmation());
}

#[test]
fn test_bypass_mode_allows_all() {
    let checker = BashPermissionChecker::with_mode(PermissionMode::BypassPermissions);

    // Even dangerous commands allowed in bypass mode
    assert!(checker.check("sudo rm -rf /").is_allowed());
    assert!(checker.check("shutdown now").is_allowed());
    assert!(checker.check("dd if=/dev/zero of=/dev/sda").is_allowed());
}

// ============================================================================
// Rule Matching Tests
// ============================================================================

#[test]
fn test_prefix_rule_matching() {
    let mut checker = BashPermissionChecker::new();
    checker.add_allow_rule("Bash".to_string(), Some("git:*".to_string()), RuleSource::UserSettings);

    // Prefix match should work
    assert!(checker.check("git status").is_allowed());
    assert!(checker.check("git log").is_allowed());
    assert!(checker.check("git commit").is_allowed());
    assert!(checker.check("git").is_allowed());
}

#[test]
fn test_exact_rule_matching() {
    let mut checker = BashPermissionChecker::new();
    checker.add_allow_rule("Bash".to_string(), Some("cargo build".to_string()), RuleSource::UserSettings);

    // Exact match
    assert!(checker.check("cargo build").is_allowed());
    // Not exact match - should ask
    assert!(checker.check("cargo build --release").needs_confirmation());
}

#[test]
fn test_deny_rule_priority() {
    let mut checker = BashPermissionChecker::new();
    // Allow all git commands
    checker.add_allow_rule("Bash".to_string(), Some("git:*".to_string()), RuleSource::UserSettings);
    // But deny git push (more specific)
    checker.add_deny_rule("Bash".to_string(), Some("git push".to_string()), RuleSource::UserSettings);

    // git push should be denied even with git:* allow rule
    assert!(checker.check("git push").is_denied());
    // Other git commands should be allowed
    assert!(checker.check("git status").is_allowed());
    assert!(checker.check("git commit").is_allowed());
}

#[test]
fn test_deny_before_allow_priority() {
    let mut checker = BashPermissionChecker::new();
    checker.add_allow_rule("Bash".to_string(), Some("rm:*".to_string()), RuleSource::UserSettings);
    checker.add_deny_rule("Bash".to_string(), Some("rm -rf /".to_string()), RuleSource::UserSettings);

    // rm -rf / should be denied
    assert!(checker.check("rm -rf /").is_denied());
    // Other rm commands allowed
    assert!(checker.check("rm file.txt").is_allowed());
}

// ============================================================================
// Compound Command Tests
// ============================================================================

#[test]
fn test_compound_cd_git_blocked() {
    let checker = BashPermissionChecker::new();

    // cd + git combination needs confirmation (bare repo risk)
    assert!(checker.check("cd /tmp && git status").needs_confirmation());
    assert!(checker.check("cd .. && git log").needs_confirmation());
}

#[test]
fn test_compound_safe_commands_allowed() {
    let checker = BashPermissionChecker::new();

    // All safe subcommands -> allow
    assert!(checker.check("ls && pwd").is_allowed());
    assert!(checker.check("echo hello && echo world").is_allowed());
}

#[test]
fn test_compound_with_deny_blocked() {
    let checker = BashPermissionChecker::new();

    // Any deny subcommand -> deny
    assert!(checker.check("ls && shutdown").is_denied());
}

// ============================================================================
// Wrapper Stripping Tests
// ============================================================================

#[test]
fn test_timeout_wrapper_stripped() {
    let checker = BashPermissionChecker::new();

    // timeout wrapper should be stripped
    assert!(checker.check("timeout 10 npm test").needs_confirmation()); // npm test is write command
}

#[test]
fn test_env_var_wrapper_stripped() {
    let checker = BashPermissionChecker::new();

    // NODE_ENV wrapper should be stripped
    let result = checker.check("NODE_ENV=production node script.js");
    // After stripping, "node script.js" - unknown -> ask
    assert!(result.needs_confirmation());
}

// ============================================================================
// Configuration Integration Tests
// ============================================================================

#[test]
fn test_config_to_checker() {
    let mut config = PermissionConfig::new();
    config.mode = PermissionMode::Default;
    config.add_bash_allow("git:*".to_string());
    config.add_bash_deny("sudo:*".to_string());

    let checker = BashPermissionChecker::with_mode(config.mode);

    // Mode should be set
    assert_eq!(checker.mode(), PermissionMode::Default);
}

#[test]
fn test_config_serialization() {
    let mut config = PermissionConfig::new();
    config.add_bash_allow("git:*".to_string());
    config.add_bash_deny("sudo:*".to_string());

    // Serialize to TOML
    let toml = toml::to_string_pretty(&config.bash).unwrap();

    // Should contain rules
    assert!(toml.contains("allow"));
    assert!(toml.contains("git:*"));
}

#[test]
fn test_config_deserialization() {
    let toml_str = r#"
mode = "default"

[bash]
allow = ["git:*", "npm:*"]
deny = ["sudo:*", "rm -rf /"]
"#;

    let config: PermissionConfig = toml::from_str(toml_str).unwrap();

    assert_eq!(config.mode, PermissionMode::Default);
    assert!(config.bash.allow.contains(&"git:*".to_string()));
    assert!(config.bash.deny.contains(&"sudo:*".to_string()));
}

// ============================================================================
// Permission Result Tests
// ============================================================================

#[test]
fn test_permission_result_is_allowed() {
    let result = PermissionResult::allow("Test allow");
    assert!(result.is_allowed());
    assert!(!result.is_denied());
    assert!(!result.needs_confirmation());
}

#[test]
fn test_permission_result_is_denied() {
    let result = PermissionResult::deny("Test deny");
    assert!(result.is_denied());
    assert!(!result.is_allowed());
    assert!(!result.needs_confirmation());
}

#[test]
fn test_permission_result_needs_confirmation() {
    let result = PermissionResult::ask("Test ask");
    assert!(result.needs_confirmation());
    assert!(!result.is_allowed());
    assert!(!result.is_denied());
}
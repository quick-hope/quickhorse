//! Permission configuration integration
//!
//! Loads and saves permission settings from config.toml.

#![allow(dead_code)] // Future use: permission config persistence

use super::types::PermissionMode;
use super::bash::BashPermissionConfig;
use serde::{Deserialize, Serialize};

/// Permission configuration section
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionConfig {
    /// Global permission mode
    #[serde(default)]
    pub mode: PermissionMode,

    /// Additional working directories
    #[serde(default)]
    pub additional_working_dirs: Vec<String>,

    /// Bash tool permissions
    #[serde(default)]
    pub bash: BashPermissionConfig,

    /// Read tool permissions
    #[serde(default)]
    pub read: ReadPermissionConfig,

    /// Edit tool permissions
    #[serde(default)]
    pub edit: EditPermissionConfig,
}

/// Read tool permission configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReadPermissionConfig {
    /// Paths to always allow reading
    #[serde(default)]
    pub allow: Vec<String>,
    /// Paths to always deny reading
    #[serde(default)]
    pub deny: Vec<String>,
    /// Paths to always ask for confirmation
    #[serde(default)]
    pub ask: Vec<String>,
}

/// Edit tool permission configuration
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EditPermissionConfig {
    /// Paths to always allow editing
    #[serde(default)]
    pub allow: Vec<String>,
    /// Paths to always deny editing
    #[serde(default)]
    pub deny: Vec<String>,
    /// Paths to always ask for confirmation
    #[serde(default)]
    pub ask: Vec<String>,
}

impl PermissionConfig {
    /// Create default permission config
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with a specific mode
    pub fn with_mode(mode: PermissionMode) -> Self {
        Self {
            mode,
            ..Self::default()
        }
    }

    /// Create bypass mode config (use with caution)
    pub fn bypass_mode() -> Self {
        Self::with_mode(PermissionMode::BypassPermissions)
    }

    /// Create accept-edits mode config
    pub fn accept_edits_mode() -> Self {
        Self::with_mode(PermissionMode::AcceptEdits)
    }

    /// Check if bypass mode is enabled
    pub fn is_bypass(&self) -> bool {
        self.mode == PermissionMode::BypassPermissions
    }

    /// Check if accept-edits mode is enabled
    pub fn is_accept_edits(&self) -> bool {
        self.mode == PermissionMode::AcceptEdits
    }

    /// Add a bash allow rule
    pub fn add_bash_allow(&mut self, pattern: String) {
        self.bash.allow.push(pattern);
    }

    /// Add a bash deny rule
    pub fn add_bash_deny(&mut self, pattern: String) {
        self.bash.deny.push(pattern);
    }

    /// Add a read allow rule
    pub fn add_read_allow(&mut self, path: String) {
        self.read.allow.push(path);
    }

    /// Add an edit allow rule
    pub fn add_edit_allow(&mut self, path: String) {
        self.edit.allow.push(path);
    }

    /// Remove a bash allow rule
    pub fn remove_bash_allow(&mut self, pattern: &str) {
        self.bash.allow.retain(|p| p != pattern);
    }

    /// Get bash rules as formatted strings for display
    pub fn bash_rules_summary(&self) -> String {
        let mut summary = String::new();

        if !self.bash.allow.is_empty() {
            summary.push_str("Allow:\n");
            for rule in &self.bash.allow {
                summary.push_str(&format!("  - {}\n", rule));
            }
        }

        if !self.bash.deny.is_empty() {
            summary.push_str("Deny:\n");
            for rule in &self.bash.deny {
                summary.push_str(&format!("  - {}\n", rule));
            }
        }

        if !self.bash.ask.is_empty() {
            summary.push_str("Ask:\n");
            for rule in &self.bash.ask {
                summary.push_str(&format!("  - {}\n", rule));
            }
        }

        if summary.is_empty() {
            summary = "No custom rules configured".to_string();
        }

        summary
    }

    /// Generate sample config for documentation
    pub fn sample_config() -> String {
        let sample = PermissionConfig {
            mode: PermissionMode::Default,
            additional_working_dirs: vec!["~/other-project".to_string()],
            bash: BashPermissionConfig {
                allow: vec![
                    "git:*".to_string(),
                    "npm:*".to_string(),
                    "cargo build".to_string(),
                ],
                deny: vec![
                    "rm -rf /".to_string(),
                    "sudo:*".to_string(),
                ],
                ask: vec![
                    "curl:*".to_string(),
                ],
            },
            read: ReadPermissionConfig {
                allow: vec!["~/docs/**".to_string()],
                deny: vec!["~/.ssh/*".to_string()],
                ask: vec![],
            },
            edit: EditPermissionConfig {
                allow: vec!["src/**".to_string(), "tests/**".to_string()],
                deny: vec!["*.env".to_string(), "*.key".to_string()],
                ask: vec![],
            },
        };

        toml::to_string_pretty(&sample).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = PermissionConfig::new();
        assert_eq!(config.mode, PermissionMode::Default);
        assert!(config.bash.allow.is_empty());
        assert!(config.bash.deny.is_empty());
    }

    #[test]
    fn test_bypass_mode() {
        let config = PermissionConfig::bypass_mode();
        assert!(config.is_bypass());
    }

    #[test]
    fn test_add_rules() {
        let mut config = PermissionConfig::new();
        config.add_bash_allow("git:*".to_string());
        config.add_bash_deny("sudo:*".to_string());

        assert!(config.bash.allow.contains(&"git:*".to_string()));
        assert!(config.bash.deny.contains(&"sudo:*".to_string()));
    }

    #[test]
    fn test_sample_config_serialization() {
        let sample = PermissionConfig::sample_config();
        assert!(sample.contains("mode = \"default\""));
        assert!(sample.contains("allow ="));
    }

    #[test]
    fn test_config_deserialization() {
        let toml_str = r#"
mode = "acceptEdits"
additional_working_dirs = ["~/other-project"]

[bash]
allow = ["git:*", "npm:*"]
deny = ["sudo:*"]
"#;

        let config: PermissionConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.mode, PermissionMode::AcceptEdits);
        assert!(config.bash.allow.contains(&"git:*".to_string()));
    }
}
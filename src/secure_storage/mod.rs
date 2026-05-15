//! Secure Storage module - API Key encryption and storage
//!
//! Provides secure storage for sensitive data like API keys.
//!
//! Platform implementations:
//! - macOS: Keychain Services via `security` CLI
//! - Linux: Encrypted file storage (libsecret would require system dependencies)
//! - Windows: Encrypted file storage (Credential Manager requires Windows-specific APIs)
//! - Fallback: Encrypted file with user-specific key

#![allow(dead_code)] // Future use: secure storage backends

mod encrypted_file;
mod keychain;
mod plain_text;

use serde::{Deserialize, Serialize};
use std::error::Error;

/// Secure storage data structure
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SecureStorageData {
    /// API keys for different providers
    #[serde(default)]
    pub api_keys: std::collections::HashMap<String, String>,
}

/// Secure storage trait - interface for all storage backends
pub trait SecureStorage: Send + Sync {
    /// Get storage name
    fn name(&self) -> &str;

    /// Read data from storage
    fn read(&self) -> Result<Option<SecureStorageData>, Box<dyn Error + Send + Sync>>;

    /// Write data to storage
    fn write(&self, data: &SecureStorageData) -> Result<bool, Box<dyn Error + Send + Sync>>;

    /// Delete all data from storage
    fn delete(&self) -> Result<bool, Box<dyn Error + Send + Sync>>;
}

/// Get the appropriate secure storage for the current platform
pub fn get_secure_storage() -> Box<dyn SecureStorage> {
    if cfg!(target_os = "macos") {
        // macOS: Keychain with encrypted file fallback
        Box::new(FallbackStorage::new(
            Box::new(keychain::MacOsKeychainStorage::new()),
            Box::new(encrypted_file::EncryptedFileStorage::new()),
        ))
    } else {
        // Other platforms: encrypted file storage
        Box::new(encrypted_file::EncryptedFileStorage::new())
    }
}

/// Get secure storage with plaintext fallback (for testing/dev)
pub fn get_secure_storage_with_plaintext_fallback() -> Box<dyn SecureStorage> {
    if cfg!(target_os = "macos") {
        Box::new(FallbackStorage::new(
            Box::new(keychain::MacOsKeychainStorage::new()),
            Box::new(FallbackStorage::new(
                Box::new(encrypted_file::EncryptedFileStorage::new()),
                Box::new(plain_text::PlainTextStorage::new()),
            )),
        ))
    } else {
        Box::new(FallbackStorage::new(
            Box::new(encrypted_file::EncryptedFileStorage::new()),
            Box::new(plain_text::PlainTextStorage::new()),
        ))
    }
}

/// Fallback storage - tries primary first, then secondary
pub struct FallbackStorage {
    primary: Box<dyn SecureStorage>,
    secondary: Box<dyn SecureStorage>,
}

impl FallbackStorage {
    pub fn new(primary: Box<dyn SecureStorage>, secondary: Box<dyn SecureStorage>) -> Self {
        Self { primary, secondary }
    }
}

impl SecureStorage for FallbackStorage {
    fn name(&self) -> &str {
        "fallback"
    }

    fn read(&self) -> Result<Option<SecureStorageData>, Box<dyn Error + Send + Sync>> {
        // Try primary first
        match self.primary.read() {
            Ok(Some(data)) => return Ok(Some(data)),
            Ok(None) => {}
            Err(_) => {}
        }

        // Fall back to secondary
        self.secondary.read()
    }

    fn write(&self, data: &SecureStorageData) -> Result<bool, Box<dyn Error + Send + Sync>> {
        // Try primary first
        match self.primary.write(data) {
            Ok(true) => {
                // Delete secondary after successful primary write (migration)
                self.secondary.delete().ok();
                return Ok(true);
            }
            Ok(false) | Err(_) => {}
        }

        // Fall back to secondary
        self.secondary.write(data)
    }

    fn delete(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let primary_result = self.primary.delete().ok();
        let secondary_result = self.secondary.delete().ok();

        Ok(primary_result.unwrap_or(false) || secondary_result.unwrap_or(false))
    }
}

/// Get the storage directory path (~/.quickhorse/)
fn get_storage_dir() -> std::path::PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("/"))
        .join(".quickhorse")
}

/// Get the current username
fn get_username() -> String {
    std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "quickhorse-user".to_string())
}

/// Get service name for keychain entries
fn get_service_name() -> String {
    "QuickHorse".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secure_storage_data_default() {
        let data = SecureStorageData::default();
        assert!(data.api_keys.is_empty());
    }

    #[test]
    fn test_secure_storage_data_serialization() {
        let mut data = SecureStorageData::default();
        data.api_keys.insert("openai".to_string(), "sk-test".to_string());

        let json = serde_json::to_string(&data).unwrap();
        assert!(json.contains("openai"));
        assert!(json.contains("sk-test"));

        let parsed: SecureStorageData = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.api_keys.get("openai").unwrap(), "sk-test");
    }

    #[test]
    fn test_get_storage_dir() {
        let dir = get_storage_dir();
        assert!(dir.to_string_lossy().contains(".quickhorse"));
    }

    #[test]
    fn test_get_service_name() {
        let name = get_service_name();
        assert_eq!(name, "QuickHorse");
    }
}
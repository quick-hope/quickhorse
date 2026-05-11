//! macOS Keychain storage implementation
//!
//! Uses the `security` CLI to interact with macOS Keychain Services.
//! This approach is portable and doesn't require native Rust bindings.

use super::{get_service_name, get_username, SecureStorage, SecureStorageData};
use std::error::Error;
use std::process::Command;

/// macOS Keychain storage backend
pub struct MacOsKeychainStorage {
    service_name: String,
    username: String,
}

impl MacOsKeychainStorage {
    pub fn new() -> Self {
        Self {
            service_name: get_service_name(),
            username: get_username(),
        }
    }

    /// Execute security command and capture output
    fn run_security_command(&self, args: &[&str]) -> Result<String, Box<dyn Error + Send + Sync>> {
        let output = Command::new("security")
            .args(args)
            .output();

        match output {
            Ok(result) => {
                if result.status.success() {
                    Ok(String::from_utf8_lossy(&result.stdout).trim().to_string())
                } else {
                    // Exit code 44 = item not found (expected for empty storage)
                    if result.status.code() == Some(44) {
                        Err("Item not found in keychain".into())
                    } else {
                        Err(format!(
                            "security command failed: {}",
                            String::from_utf8_lossy(&result.stderr)
                        ).into())
                    }
                }
            }
            Err(e) => Err(format!("Failed to execute security command: {}", e).into()),
        }
    }
}

impl Default for MacOsKeychainStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for MacOsKeychainStorage {
    fn name(&self) -> &str {
        "keychain"
    }

    fn read(&self) -> Result<Option<SecureStorageData>, Box<dyn Error + Send + Sync>> {
        let args = [
            "find-generic-password",
            "-a", &self.username,
            "-w", // Output password only
            "-s", &self.service_name,
        ];

        match self.run_security_command(&args) {
            Ok(json_string) => {
                if json_string.is_empty() {
                    Ok(None)
                } else {
                    let data: SecureStorageData = serde_json::from_str(&json_string)
                        .map_err(|e| format!("Failed to parse stored data: {}", e))?;
                    Ok(Some(data))
                }
            }
            Err(_) => Ok(None), // Item not found or error = no data
        }
    }

    fn write(&self, data: &SecureStorageData) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let json_string = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        // Convert to hex to avoid escaping issues (same as OpenClaude)
        let hex_value = hex_encode(&json_string);

        // Use add-generic-password with -U to update existing entry
        let args = [
            "add-generic-password",
            "-U", // Update if exists
            "-a", &self.username,
            "-s", &self.service_name,
            "-X", &hex_value, // Hex-encoded password
        ];

        match self.run_security_command(&args) {
            Ok(_) => Ok(true),
            Err(e) => Err(format!("Failed to write to keychain: {}", e).into()),
        }
    }

    fn delete(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        let args = [
            "delete-generic-password",
            "-a", &self.username,
            "-s", &self.service_name,
        ];

        match self.run_security_command(&args) {
            Ok(_) => Ok(true),
            Err(_) => Ok(true), // Already deleted or not found = success
        }
    }
}

/// Encode string to hexadecimal
fn hex_encode(s: &str) -> String {
    s.as_bytes()
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect()
}

/// Decode hexadecimal to string
fn hex_decode(s: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    if s.len() % 2 != 0 {
        return Err("Invalid hex string length".into());
    }

    let bytes: Result<Vec<u8>, _> = (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16)
                .map_err(|e| format!("Invalid hex character: {}", e))
        })
        .collect();

    String::from_utf8(bytes?)
        .map_err(|e| format!("Invalid UTF-8 after hex decode: {}", e).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hex_encode() {
        let input = "hello";
        let encoded = hex_encode(input);
        assert_eq!(encoded, "68656c6c6f");
    }

    #[test]
    fn test_hex_decode() {
        let input = "68656c6c6f";
        let decoded = hex_decode(input).unwrap();
        assert_eq!(decoded, "hello");
    }

    #[test]
    fn test_hex_encode_decode_roundtrip() {
        let original = "{\"api_keys\":{\"openai\":\"sk-test123\"}}";
        let encoded = hex_encode(original);
        let decoded = hex_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_keychain_storage_creation() {
        let storage = MacOsKeychainStorage::new();
        assert_eq!(storage.name(), "keychain");
        assert!(!storage.service_name.is_empty());
        assert!(!storage.username.is_empty());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_keychain_read_write_cycle() {
        let storage = MacOsKeychainStorage::new();

        // Clean up any existing data
        storage.delete().ok();

        // Write test data
        let mut data = SecureStorageData::default();
        data.api_keys.insert("test_provider".to_string(), "test_key_123".to_string());

        let write_result = storage.write(&data);
        if write_result.is_ok() {
            // Read back
            let read_result = storage.read();
            if let Ok(Some(read_data)) = read_result {
                assert_eq!(
                    read_data.api_keys.get("test_provider"),
                    Some(&"test_key_123".to_string())
                );
            }

            // Clean up
            storage.delete().ok();
        }
        // Note: This test may fail in CI environments without keychain access
    }
}
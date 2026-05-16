//! Encrypted file storage implementation
//!
//! Provides encrypted storage for sensitive data when platform-specific
//! secure storage (like macOS Keychain) is not available.
//!
//! Security approach:
//! 1. Derive encryption key from machine-specific identifiers
//! 2. XOR encrypt the JSON data (simple but effective for API key storage)
//! 3. Base64 encode the encrypted data
//! 4. Store in file with restricted permissions (0o600)

#![allow(dead_code)] // Future use: encrypted storage backend

use super::{get_storage_dir, SecureStorage, SecureStorageData};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

/// Encrypted file storage backend
pub struct EncryptedFileStorage {
    storage_path: PathBuf,
}

impl EncryptedFileStorage {
    pub fn new() -> Self {
        let storage_dir = get_storage_dir();
        let storage_path = storage_dir.join(".credentials.enc");
        Self { storage_path }
    }

    /// Get the storage file path
    fn get_path(&self) -> &PathBuf {
        &self.storage_path
    }

    /// Derive encryption key from machine-specific identifiers
    fn derive_key(&self) -> Vec<u8> {
        // Combine multiple sources for key derivation
        let mut key_source = String::new();

        // Username
        key_source.push_str(&super::get_username());

        // Home directory path
        key_source.push_str(&get_storage_dir().to_string_lossy());

        // Machine hostname (if available)
        if let Ok(hostname) = std::env::var("HOSTNAME") {
            key_source.push_str(&hostname);
        }

        // Process a deterministic key (32 bytes for AES-256)
        // This is a simple approach - for production use a proper KDF like HKDF
        let source_bytes = key_source.as_bytes();
        let mut key = vec![0u8; 32];

        for (i, byte) in source_bytes.iter().enumerate() {
            key[i % 32] = key[i % 32] ^ *byte;
        }

        // Add some fixed padding to ensure minimum entropy
        for (i, byte) in key.iter_mut().enumerate() {
            *byte = *byte ^ (0x5A + i as u8); // Simple fixed pattern
        }

        key
    }

    /// XOR encrypt data with derived key
    fn encrypt(&self, data: &str) -> String {
        let key = self.derive_key();
        let bytes = data.as_bytes();

        let encrypted: Vec<u8> = bytes
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ key[i % key.len()])
            .collect();

        // Base64 encode for storage
        base64_encode(&encrypted)
    }

    /// XOR decrypt data with derived key
    fn decrypt(&self, encoded: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
        let key = self.derive_key();
        let encrypted = base64_decode(encoded)?;

        let decrypted: Vec<u8> = encrypted
            .iter()
            .enumerate()
            .map(|(i, byte)| byte ^ key[i % key.len()])
            .collect();

        String::from_utf8(decrypted)
            .map_err(|e| format!("Failed to decode decrypted data: {}", e).into())
    }

    /// Ensure storage directory exists
    fn ensure_dir(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        let dir = self.storage_path.parent()
            .ok_or("Invalid storage path")?;

        if !dir.exists() {
            fs::create_dir_all(dir)
                .map_err(|e| format!("Failed to create storage directory: {}", e))?;
        }

        Ok(())
    }
}

impl Default for EncryptedFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for EncryptedFileStorage {
    fn name(&self) -> &str {
        "encrypted-file"
    }

    fn read(&self) -> Result<Option<SecureStorageData>, Box<dyn Error + Send + Sync>> {
        if !self.storage_path.exists() {
            return Ok(None);
        }

        // Read encrypted content
        let mut file = fs::File::open(&self.storage_path)
            .map_err(|e| format!("Failed to open storage file: {}", e))?;

        let mut encoded = String::new();
        file.read_to_string(&mut encoded)
            .map_err(|e| format!("Failed to read storage file: {}", e))?;

        if encoded.is_empty() {
            return Ok(None);
        }

        // Decrypt and parse
        let json_string = self.decrypt(&encoded)?;
        let data: SecureStorageData = serde_json::from_str(&json_string)
            .map_err(|e| format!("Failed to parse stored data: {}", e))?;

        Ok(Some(data))
    }

    fn write(&self, data: &SecureStorageData) -> Result<bool, Box<dyn Error + Send + Sync>> {
        self.ensure_dir()?;

        // Serialize and encrypt
        let json_string = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        let encrypted = self.encrypt(&json_string);

        // Write with restricted permissions
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // Owner read/write only
            .open(&self.storage_path)
            .map_err(|e| format!("Failed to create storage file: {}", e))?;

        file.write_all(encrypted.as_bytes())
            .map_err(|e| format!("Failed to write storage file: {}", e))?;

        Ok(true)
    }

    fn delete(&self) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if self.storage_path.exists() {
            fs::remove_file(&self.storage_path)
                .map_err(|e| format!("Failed to delete storage file: {}", e))?;
        }
        Ok(true)
    }
}

/// Base64 encode bytes
fn base64_encode(bytes: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let n = chunk.len();
        let mut val = 0u32;

        for (i, byte) in chunk.iter().enumerate() {
            val |= (*byte as u32) << (16 - 8 * i);
        }

        for i in 0..4 {
            if i <= n {
                result.push(ALPHABET[(val >> (18 - 6 * i)) as usize & 63] as char);
            } else {
                result.push('=');
            }
        }
    }

    result
}

/// Base64 decode string
fn base64_decode(s: &str) -> Result<Vec<u8>, Box<dyn Error + Send + Sync>> {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    let s = s.trim_end_matches('=');

    let mut result = Vec::new();

    for chunk in s.as_bytes().chunks(4) {
        let mut val = 0u32;

        for (i, byte) in chunk.iter().enumerate() {
            let idx = ALPHABET.iter()
                .position(|b| *b == *byte)
                .ok_or("Invalid base64 character")?;

            val |= (idx as u32) << (18 - 6 * i);
        }

        // Calculate number of output bytes based on padding
        let output_count = if chunk.len() == 4 {
            if chunk[3] == '=' as u8 {
                if chunk[2] == '=' as u8 {
                    1
                } else {
                    2
                }
            } else {
                3
            }
        } else if chunk.len() == 3 {
            2
        } else if chunk.len() == 2 {
            1
        } else {
            0
        };

        for i in 0..output_count {
            result.push((val >> (16 - 8 * i)) as u8);
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode() {
        let input = b"hello";
        let encoded = base64_encode(input);
        assert_eq!(encoded, "aGVsbG8=");
    }

    #[test]
    fn test_base64_decode() {
        let input = "aGVsbG8=";
        let decoded = base64_decode(input).unwrap();
        assert_eq!(decoded, b"hello");
    }

    #[test]
    fn test_base64_roundtrip() {
        let original = b"Test API Key: sk-1234567890abcdef";
        let encoded = base64_encode(original);
        let decoded = base64_decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }

    #[test]
    fn test_encrypted_storage_creation() {
        let storage = EncryptedFileStorage::new();
        assert_eq!(storage.name(), "encrypted-file");
        assert!(storage.storage_path.to_string_lossy().contains(".credentials.enc"));
    }

    #[test]
    fn test_encrypt_decrypt_roundtrip() {
        let storage = EncryptedFileStorage::new();
        let original = "{\"api_keys\":{\"openai\":\"sk-test123\"}}";

        let encrypted = storage.encrypt(original);
        assert_ne!(encrypted, original); // Should be different

        let decrypted = storage.decrypt(&encrypted).unwrap();
        assert_eq!(decrypted, original);
    }

    #[test]
    fn test_encrypted_file_storage_cycle() {
        use tempfile::TempDir;

        // Create temp directory for test
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join(".credentials.enc");

        // Create custom storage with temp path
        let mut storage = EncryptedFileStorage::new();
        storage.storage_path = storage_path.clone();

        // Write test data
        let mut data = SecureStorageData::default();
        data.api_keys.insert("test_provider".to_string(), "test_key_xyz".to_string());

        let write_result = storage.write(&data);
        assert!(write_result.is_ok());

        // Read back
        let read_result = storage.read();
        assert!(read_result.is_ok());
        let read_data = read_result.unwrap().unwrap();
        assert_eq!(
            read_data.api_keys.get("test_provider"),
            Some(&"test_key_xyz".to_string())
        );

        // Delete
        let delete_result = storage.delete();
        assert!(delete_result.is_ok());
        assert!(!storage_path.exists());
    }

    #[test]
    fn test_derive_key_consistency() {
        let storage = EncryptedFileStorage::new();

        // Key derivation should be deterministic for same inputs
        let key1 = storage.derive_key();
        let key2 = storage.derive_key();

        assert_eq!(key1, key2);
        assert_eq!(key1.len(), 32);
    }
}
//! Plaintext storage implementation
//!
//! Provides plaintext file storage as a fallback when secure storage
//! is not available. Not recommended for production use.
//!
//! File permissions are restricted (0o600) to limit access.

#![allow(dead_code)] // Future use: plaintext fallback

use super::{get_storage_dir, SecureStorage, SecureStorageData};
use std::error::Error;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::path::PathBuf;

/// Plaintext file storage backend (fallback)
pub struct PlainTextStorage {
    storage_path: PathBuf,
}

impl PlainTextStorage {
    pub fn new() -> Self {
        let storage_dir = get_storage_dir();
        let storage_path = storage_dir.join(".credentials.json");
        Self { storage_path }
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

impl Default for PlainTextStorage {
    fn default() -> Self {
        Self::new()
    }
}

impl SecureStorage for PlainTextStorage {
    fn name(&self) -> &str {
        "plaintext"
    }

    fn read(&self) -> Result<Option<SecureStorageData>, Box<dyn Error + Send + Sync>> {
        if !self.storage_path.exists() {
            return Ok(None);
        }

        let mut file = fs::File::open(&self.storage_path)
            .map_err(|e| format!("Failed to open storage file: {}", e))?;

        let mut content = String::new();
        file.read_to_string(&mut content)
            .map_err(|e| format!("Failed to read storage file: {}", e))?;

        if content.is_empty() {
            return Ok(None);
        }

        let data: SecureStorageData = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse stored data: {}", e))?;

        Ok(Some(data))
    }

    fn write(&self, data: &SecureStorageData) -> Result<bool, Box<dyn Error + Send + Sync>> {
        self.ensure_dir()?;

        let json_string = serde_json::to_string(data)
            .map_err(|e| format!("Failed to serialize data: {}", e))?;

        // Write with restricted permissions
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600) // Owner read/write only
            .open(&self.storage_path)
            .map_err(|e| format!("Failed to create storage file: {}", e))?;

        file.write_all(json_string.as_bytes())
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

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_plaintext_storage_creation() {
        let storage = PlainTextStorage::new();
        assert_eq!(storage.name(), "plaintext");
        assert!(storage.storage_path.to_string_lossy().contains(".credentials.json"));
    }

    #[test]
    fn test_plaintext_file_storage_cycle() {
        let temp_dir = TempDir::new().unwrap();
        let storage_path = temp_dir.path().join(".credentials.json");

        let mut storage = PlainTextStorage::new();
        storage.storage_path = storage_path.clone();

        // Write test data
        let mut data = SecureStorageData::default();
        data.api_keys.insert("test_provider".to_string(), "test_key_abc".to_string());

        assert!(storage.write(&data).is_ok());

        // Read back
        let read_result = storage.read();
        assert!(read_result.is_ok());
        let read_data = read_result.unwrap().unwrap();
        assert_eq!(
            read_data.api_keys.get("test_provider"),
            Some(&"test_key_abc".to_string())
        );

        // Verify file content is JSON
        let content = fs::read_to_string(&storage_path).unwrap();
        assert!(content.contains("test_provider"));
        assert!(content.contains("test_key_abc"));

        // Delete
        assert!(storage.delete().is_ok());
        assert!(!storage_path.exists());
    }
}
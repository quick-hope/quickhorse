//! Session Persistence - handles saving and loading sessions

use crate::session::Session;
use std::path::PathBuf;
use tokio::fs;

/// Session storage configuration
#[derive(Debug, Clone)]
pub struct SessionStorage {
    /// Storage directory
    pub directory: PathBuf,
    /// Max sessions to keep
    pub max_sessions: Option<usize>,
}

impl SessionStorage {
    /// Create default storage
    pub fn default() -> Self {
        Self {
            directory: PathBuf::from(".quickhorse/sessions"),
            max_sessions: Some(100),
        }
    }

    /// Create with custom directory
    pub fn with_directory(directory: String) -> Self {
        Self {
            directory: PathBuf::from(directory),
            max_sessions: Some(100),
        }
    }
}

impl Default for SessionStorage {
    fn default() -> Self {
        Self::default()
    }
}

/// Session persistence handler
pub struct SessionPersistence {
    storage: SessionStorage,
}

impl SessionPersistence {
    /// Create new persistence handler
    pub fn new(storage_dir: String) -> Self {
        Self {
            storage: SessionStorage::with_directory(storage_dir),
        }
    }

    /// Create with default storage
    pub fn default_storage() -> Self {
        Self {
            storage: SessionStorage::default(),
        }
    }

    /// Save a session to disk (async version)
    pub async fn save_async(&self, session: &Session) -> Result<(), String> {
        let path = self.get_session_path(session.metadata.id.as_str());

        // Ensure directory exists
        fs::create_dir_all(&self.storage.directory)
            .await
            .map_err(|e| format!("Failed to create sessions directory: {}", e))?;

        // Serialize session
        let content = serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        // Write to file
        fs::write(&path, content)
            .await
            .map_err(|e| format!("Failed to write session file: {}", e))?;

        Ok(())
    }

    /// Save a session to disk (sync version)
    pub fn save(&self, session: &Session) -> Result<(), String> {
        let path = self.get_session_path(session.metadata.id.as_str());

        // Ensure directory exists
        std::fs::create_dir_all(&self.storage.directory)
            .map_err(|e| format!("Failed to create sessions directory: {}", e))?;

        // Serialize session
        let content = serde_json::to_string_pretty(session)
            .map_err(|e| format!("Failed to serialize session: {}", e))?;

        // Write to file
        std::fs::write(&path, content)
            .map_err(|e| format!("Failed to write session file: {}", e))?;

        Ok(())
    }

    /// Load a session from disk (async version)
    pub async fn load_async(&self, id: &str) -> Result<Session, String> {
        let path = self.get_session_path(id);

        let content = fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read session file: {}", e))?;

        let session: Session = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse session: {}", e))?;

        Ok(session)
    }

    /// Load a session from disk (sync version)
    pub fn load(&self, id: &str) -> Result<Session, String> {
        let path = self.get_session_path(id);

        let content = std::fs::read_to_string(&path)
            .map_err(|e| format!("Failed to read session file: {}", e))?;

        let session: Session = serde_json::from_str(&content)
            .map_err(|e| format!("Failed to parse session: {}", e))?;

        Ok(session)
    }

    /// Load all sessions from disk (async version)
    pub async fn load_all_async(&self) -> Result<Vec<Session>, String> {
        let dir = &self.storage.directory;

        if !fs::try_exists(dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        let mut entries = fs::read_dir(dir)
            .await
            .map_err(|e| format!("Failed to read sessions directory: {}", e))?;

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| format!("Failed to read entry: {}", e))?
        {
            let path = entry.path();

            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        sessions.push(session);
                    }
                }
            }
        }

        // Sort by updated_at descending
        sessions.sort_by(|a, b| b.metadata.updated_at.cmp(&a.metadata.updated_at));

        Ok(sessions)
    }

    /// Delete a session from disk (async version)
    pub async fn delete_async(&self, id: &str) -> Result<(), String> {
        let path = self.get_session_path(id);

        if fs::try_exists(&path).await.unwrap_or(false) {
            fs::remove_file(&path)
                .await
                .map_err(|e| format!("Failed to delete session file: {}", e))?;
        }

        Ok(())
    }

    /// Get session file path
    fn get_session_path(&self, id: &str) -> PathBuf {
        self.storage.directory.join(format!("{}.json", id))
    }

    /// Check if session exists
    pub async fn exists(&self, id: &str) -> bool {
        fs::try_exists(self.get_session_path(id))
            .await
            .unwrap_or(false)
    }
}

impl Default for SessionPersistence {
    fn default() -> Self {
        Self::default_storage()
    }
}
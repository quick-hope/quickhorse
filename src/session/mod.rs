//! Session module - Session management and persistence

mod persistence;

pub use persistence::{SessionPersistence, SessionStorage};

use crate::provider::Message;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::fs;
use uuid::Uuid;

/// Session identifier
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionId(String);

impl SessionId {
    /// Generate a new unique session ID
    pub fn generate() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    /// Create from string
    pub fn from_string(s: String) -> Self {
        Self(s)
    }

    /// Get as string
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::generate()
    }
}

/// Session metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMetadata {
    /// Session ID
    pub id: SessionId,
    /// Session name (optional)
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: u64,
    /// Last update timestamp
    pub updated_at: u64,
    /// Provider used
    pub provider: String,
    /// Model used
    pub model: String,
}

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Message history
    pub messages: Vec<Message>,
    /// Working directory
    pub working_dir: Option<String>,
}

impl Session {
    /// Create a new session
    pub fn new(provider: String, model: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            metadata: SessionMetadata {
                id: SessionId::generate(),
                name: None,
                created_at: now,
                updated_at: now,
                provider,
                model,
            },
            messages: Vec::new(),
            working_dir: None,
        }
    }

    /// Create with custom ID
    pub fn with_id(id: SessionId, provider: String, model: String) -> Self {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        Self {
            metadata: SessionMetadata {
                id,
                name: None,
                created_at: now,
                updated_at: now,
                provider,
                model,
            },
            messages: Vec::new(),
            working_dir: None,
        }
    }

    /// Set session name
    pub fn set_name(&mut self, name: String) {
        self.metadata.name = Some(name);
        self.touch();
    }

    /// Set working directory
    pub fn set_working_dir(&mut self, path: String) {
        self.working_dir = Some(path);
        self.touch();
    }

    /// Add a message to the session
    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.touch();
    }

    /// Get all messages
    pub fn messages(&self) -> &[Message] {
        &self.messages
    }

    /// Clear messages
    pub fn clear_messages(&mut self) {
        self.messages.clear();
        self.touch();
    }

    /// Update timestamp
    fn touch(&mut self) {
        self.metadata.updated_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
    }
}

/// Session manager - handles persistence and restoration
pub struct SessionManager {
    /// Sessions directory
    sessions_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(sessions_dir: PathBuf) -> Self {
        Self { sessions_dir }
    }

    /// Create with default directory
    pub fn default_dir() -> Self {
        Self::new(PathBuf::from(".quickhorse/sessions"))
    }

    /// Save a session to file
    pub async fn save(&self, session: &Session) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        // Ensure directory exists
        fs::create_dir_all(&self.sessions_dir).await?;

        let file_path = self.session_path(session.metadata.id.as_str());
        let content = serde_json::to_string_pretty(session)?;

        fs::write(&file_path, content).await?;

        Ok(())
    }

    /// Load a session from file
    pub async fn load(&self, id: &str) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        let file_path = self.session_path(id);
        let content = fs::read_to_string(&file_path).await?;

        let session: Session = serde_json::from_str(&content)?;

        Ok(session)
    }

    /// List all saved sessions
    pub async fn list(&self) -> Result<Vec<SessionMetadata>, Box<dyn std::error::Error + Send + Sync>> {
        // Ensure directory exists
        if !fs::try_exists(&self.sessions_dir).await.unwrap_or(false) {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();
        let mut entries = fs::read_dir(&self.sessions_dir).await?;

        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = fs::read_to_string(&path).await {
                    if let Ok(session) = serde_json::from_str::<Session>(&content) {
                        sessions.push(session.metadata);
                    }
                }
            }
        }

        // Sort by updated_at descending (most recent first)
        sessions.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));

        Ok(sessions)
    }

    /// Delete a session
    pub async fn delete(&self, id: &str) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let file_path = self.session_path(id);
        fs::remove_file(&file_path).await?;
        Ok(())
    }

    /// Get session file path
    fn session_path(&self, id: &str) -> PathBuf {
        self.sessions_dir.join(format!("{}.json", id))
    }
}

/// Session restoration helper
pub struct SessionRestorer {
    manager: SessionManager,
}

impl SessionRestorer {
    /// Create a new restorer
    pub fn new(manager: SessionManager) -> Self {
        Self { manager }
    }

    /// Restore session or create new if not found
    pub async fn restore_or_create(
        &self,
        id: Option<&str>,
        provider: String,
        model: String,
    ) -> Result<Session, Box<dyn std::error::Error + Send + Sync>> {
        match id {
            Some(id) => {
                // Try to load existing session
                match self.manager.load(id).await {
                    Ok(session) => Ok(session),
                    Err(_) => {
                        // Create new with provided ID
                        Ok(Session::with_id(
                            SessionId::from_string(id.to_string()),
                            provider,
                            model,
                        ))
                    }
                }
            }
            None => {
                // Create new session
                Ok(Session::new(provider, model))
            }
        }
    }
}
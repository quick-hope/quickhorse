//! Session Manager - manages active sessions

use crate::session::{SessionId, SessionMessage, SessionPersistence, ToolCallRecord, ToolResultRecord};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Session state
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SessionState {
    /// Session is active
    Active,
    /// Session is paused
    Paused,
    /// Session is completed
    Completed,
    /// Session has error
    Error,
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Session {
    /// Session ID
    pub id: SessionId,
    /// Session name/title
    pub name: String,
    /// Working directory
    pub cwd: String,
    /// Provider type (openai, anthropic, etc.)
    pub provider: String,
    /// Model name
    pub model: String,
    /// System prompt
    #[serde(default)]
    pub system_prompt: Option<String>,
    /// Message history
    pub messages: Vec<SessionMessage>,
    /// Session state
    pub state: SessionState,
    /// Creation time
    pub created_at: DateTime<Utc>,
    /// Last update time
    pub updated_at: DateTime<Utc>,
    /// Session metadata
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}

impl Session {
    /// Create a new session
    pub fn new(name: String, cwd: String, provider: String, model: String) -> Self {
        Self {
            id: SessionId::new(),
            name,
            cwd,
            provider,
            model,
            system_prompt: None,
            messages: Vec::new(),
            state: SessionState::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
            metadata: HashMap::new(),
        }
    }

    /// Create with system prompt
    pub fn with_system_prompt(mut self, prompt: String) -> Self {
        self.system_prompt = Some(prompt);
        self
    }

    /// Add a message
    pub fn add_message(&mut self, message: SessionMessage) {
        self.messages.push(message);
        self.updated_at = Utc::now();
    }

    /// Add user message
    pub fn add_user_message(&mut self, content: String) {
        self.add_message(SessionMessage::user(content));
    }

    /// Add assistant message
    pub fn add_assistant_message(&mut self, content: String) {
        self.add_message(SessionMessage::assistant(content));
    }

    /// Add system message
    pub fn add_system_message(&mut self, content: String) {
        self.add_message(SessionMessage::system(content));
    }

    /// Add tool call to last message
    pub fn add_tool_call_to_last(&mut self, tool_call: ToolCallRecord) {
        if let Some(last_msg) = self.messages.last_mut() {
            last_msg.add_tool_call(tool_call);
            self.updated_at = Utc::now();
        }
    }

    /// Add tool result to last message
    pub fn add_tool_result_to_last(&mut self, tool_result: ToolResultRecord) {
        if let Some(last_msg) = self.messages.last_mut() {
            last_msg.add_tool_result(tool_result);
            self.updated_at = Utc::now();
        }
    }

    /// Set state
    pub fn set_state(&mut self, state: SessionState) {
        self.state = state;
        self.updated_at = Utc::now();
    }

    /// Get messages as provider format
    pub fn get_provider_messages(&self) -> Vec<crate::provider::Message> {
        let mut provider_messages = Vec::new();

        // Add system prompt first
        if let Some(system) = &self.system_prompt {
            provider_messages.push(crate::provider::Message::system(system.clone()));
        }

        // Convert session messages
        for msg in &self.messages {
            let provider_msg = match msg.role.as_str() {
                "user" => crate::provider::Message::user(msg.content.clone()),
                "assistant" => {
                    // If message has tool calls, create with tool blocks
                    if msg.tool_calls.is_empty() {
                        crate::provider::Message::assistant(msg.content.clone())
                    } else {
                        let blocks: Vec<crate::provider::ContentBlock> = msg
                            .tool_calls
                            .iter()
                            .map(|tc| {
                                crate::provider::ContentBlock::tool_use(
                                    tc.id.clone(),
                                    tc.name.clone(),
                                    tc.arguments.clone(),
                                )
                            })
                            .collect();

                        // Add text content first if present
                        let mut all_blocks = Vec::new();
                        if !msg.content.is_empty() {
                            all_blocks.push(crate::provider::ContentBlock::text(msg.content.clone()));
                        }
                        all_blocks.extend(blocks);

                        crate::provider::Message::assistant_with_tools(all_blocks)
                    }
                }
                "system" => crate::provider::Message::system(msg.content.clone()),
                _ => continue,
            };

            provider_messages.push(provider_msg);

            // Add tool results as user message
            if !msg.tool_results.is_empty() {
                let result_blocks: Vec<crate::provider::ContentBlock> = msg
                    .tool_results
                    .iter()
                    .map(|tr| {
                        crate::provider::ContentBlock::tool_result(
                            tr.tool_call_id.clone(),
                            tr.content.clone(),
                            tr.is_error,
                        )
                    })
                    .collect();

                provider_messages.push(crate::provider::Message::user_with_tool_results(result_blocks));
            }
        }

        provider_messages
    }

    /// Get message count
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Get last N messages
    pub fn get_last_messages(&self, n: usize) -> &[SessionMessage] {
        let start = self.messages.len().saturating_sub(n);
        &self.messages[start..]
    }
}

/// Session manager - manages multiple sessions
pub struct SessionManager {
    /// Active sessions
    sessions: HashMap<String, Session>,
    /// Current session ID
    current_session_id: Option<String>,
    /// Persistence layer
    persistence: Arc<SessionPersistence>,
}

impl SessionManager {
    /// Create a new session manager
    pub fn new(persistence: Arc<SessionPersistence>) -> Self {
        Self {
            sessions: HashMap::new(),
            current_session_id: None,
            persistence,
        }
    }

    /// Create with default persistence
    pub fn with_default_persistence(storage_dir: String) -> Self {
        Self::new(Arc::new(SessionPersistence::new(storage_dir)))
    }

    /// Create a new session
    pub fn create_session(
        &mut self,
        name: String,
        cwd: String,
        provider: String,
        model: String,
    ) -> Session {
        let session = Session::new(name, cwd, provider, model);
        let id = session.id.as_str().to_string();
        self.sessions.insert(id.clone(), session.clone());
        self.current_session_id = Some(id);
        session
    }

    /// Get current session
    pub fn get_current_session(&self) -> Option<&Session> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.sessions.get(id))
    }

    /// Get current session for modification
    pub fn get_current_session_mut(&mut self) -> Option<&mut Session> {
        self.current_session_id
            .as_ref()
            .and_then(|id| self.sessions.get_mut(id))
    }

    /// Set current session
    pub fn set_current_session(&mut self, id: &str) -> Result<(), String> {
        if self.sessions.contains_key(id) {
            self.current_session_id = Some(id.to_string());
            Ok(())
        } else {
            Err(format!("Session not found: {}", id))
        }
    }

    /// Get session by ID
    pub fn get_session(&self, id: &str) -> Option<&Session> {
        self.sessions.get(id)
    }

    /// Get session by ID for modification
    pub fn get_session_mut(&mut self, id: &str) -> Option<&mut Session> {
        self.sessions.get_mut(id)
    }

    /// List all sessions
    pub fn list_sessions(&self) -> Vec<&Session> {
        self.sessions.values().collect()
    }

    /// List active sessions
    pub fn list_active_sessions(&self) -> Vec<&Session> {
        self.sessions
            .values()
            .filter(|s| s.state == SessionState::Active)
            .collect()
    }

    /// Delete a session
    pub fn delete_session(&mut self, id: &str) -> Result<(), String> {
        if self.sessions.remove(id).is_some() {
            if self.current_session_id.as_ref() == Some(&id.to_string()) {
                self.current_session_id = None;
            }
            // Delete from persistence
            self.persistence.delete(id)?;
            Ok(())
        } else {
            Err(format!("Session not found: {}", id))
        }
    }

    /// Save current session
    pub fn save_current_session(&self) -> Result<(), String> {
        if let Some(session) = self.get_current_session() {
            self.persistence.save(session)?;
        }
        Ok(())
    }

    /// Save all sessions
    pub fn save_all_sessions(&self) -> Result<(), String> {
        for session in self.sessions.values() {
            self.persistence.save(session)?;
        }
        Ok(())
    }

    /// Load a session
    pub fn load_session(&mut self, id: &str) -> Result<Session, String> {
        let session = self.persistence.load(id)?;
        self.sessions.insert(id.to_string(), session.clone());
        Ok(session)
    }

    /// Load all persisted sessions
    pub fn load_all_sessions(&mut self) -> Result<Vec<Session>, String> {
        let sessions = self.persistence.load_all()?;
        for session in &sessions {
            self.sessions.insert(session.id.as_str().to_string(), session.clone());
        }
        Ok(sessions)
    }

    /// Get session count
    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}
//! Integration tests for Session management
//!
//! Tests cover:
//! - Session creation and persistence
//! - Session loading and restoration
//! - Message serialization
//! - Metadata management

use quickhorse::session::{Session, SessionManager, SessionMetadata, SessionId};
use quickhorse::provider::Message;
use quickhorse::test_utils::TestSessionFixture;
use std::path::PathBuf;

// ============================================================================
// Session Creation Tests
// ============================================================================

#[test]
fn test_session_creation() {
    let session = Session::new("mock".to_string(), "mock-model".to_string());

    // Session has metadata with ID
    assert!(!session.metadata.id.as_str().is_empty());
    assert!(session.messages().is_empty());
}

#[test]
fn test_session_with_id() {
    let id = SessionId::from_string("test-session-123".to_string());
    let session = Session::with_id(id.clone(), "mock".to_string(), "mock-model".to_string());

    assert_eq!(session.metadata.id.as_str(), id.as_str());
}

#[test]
fn test_session_default_id() {
    let id = SessionId::default();
    assert!(!id.as_str().is_empty());
}

// ============================================================================
// Message Management Tests
// ============================================================================

#[test]
fn test_session_add_message() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());

    session.add_message(Message::user("Hello".to_string()));
    session.add_message(Message::assistant("Hi there".to_string()));

    assert_eq!(session.messages().len(), 2);
}

#[test]
fn test_session_clear_messages() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());

    session.add_message(Message::user("Test".to_string()));
    session.add_message(Message::assistant("Response".to_string()));

    assert_eq!(session.messages().len(), 2);

    session.clear_messages();

    assert!(session.messages().is_empty());
}

#[test]
fn test_session_message_sequence() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());

    // Add messages in order
    session.add_message(Message::system("Be helpful".to_string()));
    session.add_message(Message::user("First question".to_string()));
    session.add_message(Message::assistant("First answer".to_string()));
    session.add_message(Message::user("Second question".to_string()));

    let messages = session.messages();
    assert_eq!(messages.len(), 4);
    assert_eq!(messages[0].role, "system");
    assert_eq!(messages[1].role, "user");
    assert_eq!(messages[2].role, "assistant");
    assert_eq!(messages[3].role, "user");
}

// ============================================================================
// Metadata Tests
// ============================================================================

#[test]
fn test_session_metadata_fields() {
    let session = Session::new("anthropic".to_string(), "claude-3".to_string());

    assert!(!session.metadata.id.as_str().is_empty());
    assert_eq!(session.metadata.provider, "anthropic");
    assert_eq!(session.metadata.model, "claude-3");
    assert!(session.metadata.created_at > 0);
    assert!(session.metadata.updated_at > 0);
}

#[test]
fn test_session_set_name() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());

    session.set_name("Test Session".to_string());

    assert_eq!(session.metadata.name, Some("Test Session".to_string()));
}

#[test]
fn test_session_working_dir() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());

    session.set_working_dir("/tmp/test".to_string());

    assert_eq!(session.working_dir, Some("/tmp/test".to_string()));
}

// ============================================================================
// Session Manager Tests
// ============================================================================

#[test]
fn test_session_manager_creation() {
    let fixture = TestSessionFixture::new();
    let manager = SessionManager::new(fixture.temp_dir.path().to_path_buf());

    // Manager should be created successfully
    assert!(manager.sessions_dir().exists() || !fixture.temp_dir.path().exists());
}

#[test]
fn test_session_manager_default_dir() {
    let manager = SessionManager::default_dir();

    // Should use default directory
    let sessions_dir = manager.sessions_dir();
    assert!(sessions_dir.to_string_lossy().contains("quickhorse"));
}

// ============================================================================
// Serialization Tests
// ============================================================================

#[test]
fn test_session_serialization() {
    let mut session = Session::new("mock".to_string(), "mock-model".to_string());
    session.add_message(Message::user("Test".to_string()));

    // Serialize to JSON
    let json = serde_json::to_string(&session);
    assert!(json.is_ok());

    // Deserialize back
    let deserialized: Session = serde_json::from_str(&json.unwrap()).unwrap();
    assert_eq!(deserialized.messages().len(), 1);
}

#[test]
fn test_message_serialization() {
    let message = Message::user("Hello world".to_string());

    let json = serde_json::to_string(&message);
    assert!(json.is_ok());

    let deserialized: Message = serde_json::from_str(&json.unwrap()).unwrap();
    assert_eq!(deserialized.text_content(), "Hello world");
}

#[test]
fn test_session_id_serialization() {
    let id = SessionId::generate();

    let json = serde_json::to_string(&id);
    assert!(json.is_ok());

    let deserialized: SessionId = serde_json::from_str(&json.unwrap()).unwrap();
    assert_eq!(id.as_str(), deserialized.as_str());
}

// ============================================================================
// Persistence Tests
// ============================================================================

#[test]
fn test_session_file_path() {
    let fixture = TestSessionFixture::new();

    let session_path = fixture.session_path();

    // Path should include session ID
    assert!(session_path.to_string_lossy().contains(&fixture.session_id));
}
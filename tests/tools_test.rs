//! Integration tests for QuickHorse tools

use quickhorse::tools::{BashTool, FileReadTool, FileEditTool, GlobTool, Tool, ToolContext};

fn get_context() -> ToolContext {
    ToolContext::default()
}

#[tokio::test]
async fn test_bash_tool_simple_command() {
    let tool = BashTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "command": "echo hello",
        "timeout": 5
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(!tool_result.is_error);
    assert!(tool_result.content.contains("hello"));
}

#[tokio::test]
async fn test_bash_tool_blocked_command() {
    let tool = BashTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "command": "rm -rf /",
        "timeout": 5
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(tool_result.is_error);
    assert!(tool_result.content.contains("blocked"));
}

#[tokio::test]
async fn test_bash_tool_invalid_json() {
    let tool = BashTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "not_command": "echo hello"
    });

    let result = tool.call(input, &context).await;

    // When JSON parsing fails, the tool returns an Err
    assert!(result.is_err());
}

#[tokio::test]
async fn test_file_read_tool() {
    let tool = FileReadTool::new();
    let context = get_context();

    // Read Cargo.toml
    let input = serde_json::json!({
        "file_path": "Cargo.toml",
        "offset": 1,
        "limit": 10
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(!tool_result.is_error);
    assert!(tool_result.content.contains("quickhorse"));
}

#[tokio::test]
async fn test_file_read_tool_nonexistent() {
    let tool = FileReadTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "file_path": "nonexistent_file.txt"
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(tool_result.is_error);
    assert!(tool_result.content.contains("does not exist"));
}

#[tokio::test]
async fn test_glob_tool() {
    let tool = GlobTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "pattern": "**/*.rs"
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(!tool_result.is_error);
    assert!(tool_result.content.contains(".rs"));
}

#[tokio::test]
async fn test_glob_tool_empty_pattern() {
    let tool = GlobTool::new();
    let context = get_context();

    let input = serde_json::json!({
        "pattern": ""
    });

    let result = tool.call(input, &context).await;

    assert!(result.is_ok());
    let tool_result = result.unwrap();
    assert!(tool_result.is_error);
}

#[test]
fn test_tool_names() {
    let bash = BashTool::new();
    let read = FileReadTool::new();
    let edit = FileEditTool::new();
    let glob = GlobTool::new();

    assert_eq!(bash.name(), "Bash");
    assert_eq!(read.name(), "Read");
    assert_eq!(edit.name(), "Edit");
    assert_eq!(glob.name(), "Glob");
}

#[test]
fn test_tool_is_read_only() {
    let bash = BashTool::new();
    let read = FileReadTool::new();
    let edit = FileEditTool::new();
    let glob = GlobTool::new();

    // Bash with read command
    let bash_read = serde_json::json!({"command": "ls"});
    assert!(bash.is_read_only(&bash_read));

    // Bash with write command
    let bash_write = serde_json::json!({"command": "rm file.txt"});
    assert!(!bash.is_read_only(&bash_write));

    // Read is always read-only
    let read_input = serde_json::json!({"file_path": "test.txt"});
    assert!(read.is_read_only(&read_input));

    // Edit is never read-only
    let edit_input = serde_json::json!({"file_path": "test.txt", "old_string": "a", "new_string": "b"});
    assert!(!edit.is_read_only(&edit_input));

    // Glob is always read-only
    let glob_input = serde_json::json!({"pattern": "*.txt"});
    assert!(glob.is_read_only(&glob_input));
}
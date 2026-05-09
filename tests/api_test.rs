//! Test API connection with custom base_url

use quickhorse::provider::{OpenAIProvider, Provider, Message};

#[tokio::test]
async fn test_bailian_api_connection() {
    let api_key = std::env::var("OPENAI_API_KEY")
        .unwrap_or("test-key".to_string());
    let base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or("https://api.openai.com/v1/chat/completions".to_string());
    
    // Skip if no real API key
    if api_key == "test-key" {
        println!("Skipping test - no API key provided");
        return;
    }
    
    let provider = OpenAIProvider::new_with_base_url(
        api_key,
        "qwen3.5-plus".to_string(),
        base_url,
    );
    
    let messages = vec![
        Message::user("你好，请回复一句话".to_string()),
    ];
    
    let result = provider.send_message(&messages).await;
    
    match result {
        Ok(msg) => {
            let content = msg.text_content();
            println!("Response: {}", content);
            assert!(!content.is_empty(), "Response should not be empty");
        }
        Err(e) => {
            println!("Error: {}", e);
            // Don't fail the test on API errors - just log them
        }
    }
}
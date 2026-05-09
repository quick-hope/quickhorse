use quickhorse::provider::{OpenAIProvider, Provider, Message};
use std::sync::Arc;

fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").unwrap();
    let base_url = std::env::var("OPENAI_BASE_URL")
        .unwrap_or("https://api.openai.com/v1/chat/completions".to_string());
    
    let provider = OpenAIProvider::new_with_url(
        api_key,
        "qwen3.5-plus".to_string(),
        base_url,
    );
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let messages = vec![
            Message::system("You are a helpful assistant.".to_string()),
            Message::user("Say hello in Chinese.".to_string()),
        ];
        
        let result = provider.send_message(&messages).await;
        match result {
            Ok(msg) => {
                println!("Response: {}", msg.text_content());
            }
            Err(e) => {
                println!("Error: {}", e);
            }
        }
    });
}

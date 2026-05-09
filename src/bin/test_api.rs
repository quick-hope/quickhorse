use quickhorse::provider::{OpenAIProvider, Provider, Message};

fn main() {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY required");
    let base_url = std::env::var("OPENAI_BASE_URL")
        .expect("OPENAI_BASE_URL required");
    
    println!("API Key: {}", &api_key[..10]);
    println!("Base URL: {}", base_url);
    
    let provider = OpenAIProvider::new_with_base_url(
        api_key,
        "qwen3.5-plus".to_string(),
        base_url,
    );
    
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let messages = vec![
            Message::user("你好".to_string()),
        ];
        
        println!("Sending message...");
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
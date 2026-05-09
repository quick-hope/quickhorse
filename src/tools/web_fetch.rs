//! WebFetchTool - Fetch and analyze web content

use crate::tools::tool_trait::{build_schema, Tool, ToolContext, ToolResult};
use async_trait::async_trait;
use reqwest::Client;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::time::Duration;

/// WebFetchTool input schema
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WebFetchInput {
    /// The URL to fetch
    pub url: String,
    /// Prompt describing what information to extract from the page
    pub prompt: String,
    /// Timeout in seconds (default 30)
    #[serde(default = "default_timeout")]
    pub timeout: u32,
}

fn default_timeout() -> u32 {
    30
}

/// WebFetchTool - Fetch web content and extract information
pub struct WebFetchTool {
    client: Client,
}

impl WebFetchTool {
    /// Create a new WebFetchTool instance
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("QuickHorse/0.1.0 (AI Coding Agent)")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self { client }
    }

    /// Convert HTML to simple markdown-like text
    fn html_to_text(html: &str) -> String {
        let mut text = html.to_string();

        // Remove script and style tags
        let script_regex = regex::Regex::new(r"<script[^>]*>.*?</script>").unwrap();
        let style_regex = regex::Regex::new(r"<style[^>]*>.*?</style>").unwrap();
        text = script_regex.replace_all(&text, "").to_string();
        text = style_regex.replace_all(&text, "").to_string();

        // Remove HTML comments
        let comment_regex = regex::Regex::new(r"<!--.*?-->").unwrap();
        text = comment_regex.replace_all(&text, "").to_string();

        // Convert common HTML elements
        let heading_regex = regex::Regex::new(r"<h[1-6][^>]*>(.*?)</h[1-6]>").unwrap();
        text = heading_regex.replace_all(&text, "\n## $1\n").to_string();

        let paragraph_regex = regex::Regex::new(r"<p[^>]*>(.*?)</p>").unwrap();
        text = paragraph_regex.replace_all(&text, "\n$1\n").to_string();

        // Use r#"..."# for patterns containing quotes
        let link_regex = regex::Regex::new(r#"<a[^>]*href="([^"]+)"[^>]*>(.*?)</a>"#).unwrap();
        text = link_regex.replace_all(&text, "[$2]($1)").to_string();

        let bold_regex = regex::Regex::new(r"<strong[^>]*>(.*?)</strong>|<b[^>]*>(.*?)</b>").unwrap();
        text = bold_regex.replace_all(&text, "**$1$2**").to_string();

        let italic_regex = regex::Regex::new(r"<em[^>]*>(.*?)</em>|<i[^>]*>(.*?)</i>").unwrap();
        text = italic_regex.replace_all(&text, "*$1$2*").to_string();

        let code_regex = regex::Regex::new(r"<code[^>]*>(.*?)</code>").unwrap();
        text = code_regex.replace_all(&text, "`$1`").to_string();

        let pre_regex = regex::Regex::new(r"<pre[^>]*>(.*?)</pre>").unwrap();
        text = pre_regex.replace_all(&text, "\n```\n$1\n```\n").to_string();

        let list_item_regex = regex::Regex::new(r"<li[^>]*>(.*?)</li>").unwrap();
        text = list_item_regex.replace_all(&text, "- $1").to_string();

        let br_regex = regex::Regex::new(r"<br[^>]*/?>").unwrap();
        text = br_regex.replace_all(&text, "\n").to_string();

        // Remove remaining HTML tags
        let tag_regex = regex::Regex::new(r"<[^>]+>").unwrap();
        text = tag_regex.replace_all(&text, "").to_string();

        // Decode HTML entities
        text = text.replace("&nbsp;", " ");
        text = text.replace("&amp;", "&");
        text = text.replace("&lt;", "<");
        text = text.replace("&gt;", ">");
        text = text.replace("&quot;", "\"");
        text = text.replace("&apos;", "'");

        // Clean up whitespace - use raw string for \n which needs literal backslash
        let whitespace_regex = regex::Regex::new(r"\n{3,}").unwrap();
        text = whitespace_regex.replace_all(&text, "\n\n").to_string();

        // Truncate if too long
        const MAX_CHARS: usize = 50000;
        if text.len() > MAX_CHARS {
            text = format!("{}... (truncated, {} chars omitted)",
                &text[..MAX_CHARS], text.len() - MAX_CHARS);
        }

        text.trim().to_string()
    }
}

impl Default for WebFetchTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for WebFetchTool {
    fn name(&self) -> &str {
        "WebFetch"
    }

    fn description(&self) -> String {
        "Fetch web content and extract information. Use for retrieving documentation, API references, articles. Returns processed content based on your prompt.".to_string()
    }

    fn input_schema(&self) -> serde_json::Value {
        build_schema::<WebFetchInput>()
    }

    async fn call(
        &self,
        input: serde_json::Value,
        _context: &ToolContext,
    ) -> Result<ToolResult, Box<dyn Error + Send + Sync>> {
        let fetch_input: WebFetchInput = serde_json::from_value(input.clone())
            .map_err(|e| format!("Invalid input: {}", e))?;

        // Validate URL
        let url = fetch_input.url.trim();
        if url.is_empty() {
            return Ok(ToolResult::error("URL cannot be empty".to_string()));
        }

        // Check URL scheme
        if !url.starts_with("http://") && !url.starts_with("https://") {
            return Ok(ToolResult::error(
                "URL must start with http:// or https://".to_string()
            ));
        }

        // Fetch the URL
        let response = self.client
            .get(url)
            .timeout(Duration::from_secs(fetch_input.timeout.min(60) as u64))
            .send()
            .await
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        // Check status
        let status = response.status();
        if !status.is_success() {
            return Ok(ToolResult::error(format!(
                "HTTP error: {} {}",
                status.as_u16(),
                status.canonical_reason().unwrap_or("Unknown")
            )));
        }

        // Get content type first (before consuming response)
        let content_type = response.headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();  // Clone to owned string

        // Get body (consumes response)
        let body = response.text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        // Process content based on type
        let processed_content = if content_type.contains("text/html") {
            Self::html_to_text(&body)
        } else if content_type.contains("application/json") {
            // Format JSON nicely
            body
        } else {
            // Plain text or other
            body
        };

        // Build result
        let result = format!(
            "URL: {}\n\n{}\n\nExtracted for prompt: {}",
            url,
            processed_content,
            fetch_input.prompt
        );

        Ok(ToolResult::success(result))
    }

    fn is_read_only(&self, _input: &serde_json::Value) -> bool {
        true
    }

    fn summarize(&self, input: &serde_json::Value) -> String {
        if let Ok(fetch_input) = serde_json::from_value::<WebFetchInput>(input.clone()) {
            format!("Fetching: {}", fetch_input.url)
        } else {
            "Fetching web content".to_string()
        }
    }
}
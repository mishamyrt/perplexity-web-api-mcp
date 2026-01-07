//! Example demonstrating authenticated usage with cookies for pro/reasoning modes.
//!
//! Run with: `cargo run --example with_cookies`

use perplexity_web_api::{Client, SearchRequest};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> perplexity_web_api::Result<()> {
    println!("=== Authenticated Usage Example ===\n");

    // To use pro/reasoning modes, you need Perplexity account cookies.
    // See README for instructions on obtaining cookies.
    let mut cookies = HashMap::new();
    cookies.insert("next-auth.csrf-token".to_string(), "your-token".to_string());
    cookies.insert(
        "next-auth.session-token".to_string(),
        "your-session".to_string(),
    );

    if cookies.is_empty() {
        println!("No cookies provided. Showing example code only.\n");
        println!(
            r#"Example code for authenticated usage:

// 1. Get your cookies from Perplexity.ai (see README)
let mut cookies = HashMap::new();
cookies.insert("next-auth.csrf-token".to_string(), "your-token".to_string());
cookies.insert("next-auth.session-token".to_string(), "your-session".to_string());

// 2. Create authenticated client
let client = Client::builder()
    .cookies(cookies)
    .build()
    .await?;

// 3. Use pro mode with specific model
let response = client.search(
    SearchRequest::new("Explain the implications of quantum supremacy")
        .mode("pro")
        .model("gpt-5.2")
).await?;

// 4. Use reasoning mode for complex analysis
let response = client.search(
    SearchRequest::new("Compare the economic policies of keynesianism vs monetarism")
        .mode("reasoning")
        .model("claude-4.5-sonnet-thinking")
).await?;

// 5. Use deep research for comprehensive topics
let response = client.search(
    SearchRequest::new("Latest developments in fusion energy research")
        .mode("deep research")
).await?;
"#
        );
        return Ok(());
    }

    let client = Client::builder().cookies(cookies).build().await?;

    println!("Making pro mode query with GPT-5.2...\n");

    let response = client
        .search(
            SearchRequest::new("Explain the technical challenges of achieving AGI")
                .mode("pro")
                .model("gpt-5.2"),
        )
        .await?;

    println!("--- Pro Mode Response ---");
    if let Some(answer) = response.answer {
        println!("{}", answer);
    }
    println!("-------------------------\n");

    // Follow-up query using context
    println!("Making follow-up query...\n");

    let response = client
        .search(
            SearchRequest::new("What are the leading approaches to solving these challenges?")
                .mode("pro")
                .follow_up(response.follow_up),
        )
        .await?;

    println!("--- Follow-up Response ---");
    if let Some(answer) = response.answer {
        println!("{}", answer);
    }
    println!("--------------------------");

    Ok(())
}

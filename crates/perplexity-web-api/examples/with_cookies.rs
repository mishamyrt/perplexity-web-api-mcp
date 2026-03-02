//! Example demonstrating authenticated usage with cookies for pro/reasoning modes.
//!
//! Run with: `cargo run --example with_cookies`

use perplexity_web_api::{Client, SearchMode, SearchModel, SearchRequest};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> perplexity_web_api::Result<()> {
    println!("=== Authenticated Usage Example ===\n");

    // To use pro/reasoning modes, you need Perplexity account cookies.
    // See README for instructions on obtaining cookies.
    let session_token = std::env::var("PERPLEXITY_SESSION_TOKEN").ok();
    let csrf_token = std::env::var("PERPLEXITY_CSRF_TOKEN").ok();

    let (Some(session_token), Some(csrf_token)) = (session_token, csrf_token) else {
        println!("No cookies provided. Showing example code only.\n");
        println!(
            r#"Example code for authenticated usage:

// 1. Export your cookies from Perplexity.ai (see README)
// export PERPLEXITY_SESSION_TOKEN="your-session-token"
// export PERPLEXITY_CSRF_TOKEN="your-csrf-token"

// 2. Create authenticated client
let mut cookies = HashMap::new();
cookies.insert("next-auth.session-token".to_string(), std::env::var("PERPLEXITY_SESSION_TOKEN")?);
cookies.insert("next-auth.csrf-token".to_string(), std::env::var("PERPLEXITY_CSRF_TOKEN")?);

let client = Client::builder()
    .cookies(cookies)
    .build()
    .await?;

// 3. Use pro mode with specific model
let response = client.search(
    SearchRequest::new("Explain the implications of quantum supremacy")
        .mode(SearchMode::Pro)
        .model(SearchModel::Gpt52)
).await?;

// 4. Use reasoning mode for complex analysis
let response = client.search(
    SearchRequest::new("Compare the economic policies of keynesianism vs monetarism")
        .mode(SearchMode::Reasoning)
        .model(ReasonModel::Claude46SonnetThinking)
).await?;

// 5. Use deep research for comprehensive topics
let response = client.search(
    SearchRequest::new("Latest developments in fusion energy research")
        .mode(SearchMode::DeepResearch)
).await?;
"#
        );
        return Ok(());
    };

    let mut cookies = HashMap::new();
    cookies.insert("next-auth.session-token".to_string(), session_token);
    cookies.insert("next-auth.csrf-token".to_string(), csrf_token);

    let client = Client::builder().cookies(cookies).build().await?;

    println!("Making pro mode query with GPT-5.2...\n");

    let response = client
        .search(
            SearchRequest::new("Explain the technical challenges of achieving AGI")
                .mode(SearchMode::Pro)
                .model(SearchModel::Gpt52),
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
                .mode(SearchMode::Pro)
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

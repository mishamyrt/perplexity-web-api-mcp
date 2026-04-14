//! Example demonstrating authenticated usage with cookies for pro/reasoning modes.
//!
//! Run with: `cargo run --example with_cookies`

use perplexity_web_api::{AuthCookies, Client, SearchMode, SearchModel, SearchRequest};

#[tokio::main(flavor = "current_thread")]
async fn main() -> perplexity_web_api::Result<()> {
    println!("=== Authenticated Usage Example ===\n");

    // To use pro/reasoning modes, you need the values from the
    // `__Secure-next-auth.session-token` and `next-auth.csrf-token` browser cookies.
    // See README for instructions on obtaining them.
    let session_token = std::env::var("PERPLEXITY_SESSION_TOKEN").ok();
    let csrf_token = std::env::var("PERPLEXITY_CSRF_TOKEN").ok();

    let (Some(session_token), Some(csrf_token)) = (session_token, csrf_token) else {
        println!("No tokens provided, exiting.");
        return Ok(());
    };

    let cookies = AuthCookies::new(session_token, csrf_token);

    let client = Client::builder().cookies(cookies).build().await?;

    println!("Making pro mode query with GPT-5.4...\n");

    let response = client
        .search(
            SearchRequest::new("Explain the technical challenges of achieving AGI")
                .mode(SearchMode::Pro)
                .model(SearchModel::Gpt54),
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

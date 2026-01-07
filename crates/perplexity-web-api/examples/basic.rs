//! Basic usage example demonstrating a simple non-streaming query.
//!
//! Run with: `cargo run --example basic`

use perplexity_web_api::{Client, SearchRequest};

#[tokio::main]
async fn main() -> perplexity_web_api::Result<()> {
    println!("Creating Perplexity client...");

    let client = Client::builder().build().await?;

    println!("Making query...");

    let response = client
        .search(SearchRequest::new("What is the Rust programming language?"))
        .await?;

    println!("\n--- Response ---");
    if let Some(answer) = response.answer {
        println!("{}", answer);
    } else {
        println!("No answer received");
    }
    println!("----------------\n");

    // Example with different sources
    println!("Making query with scholar sources...");

    let response = client
        .search(
            SearchRequest::new("Latest advances in machine learning")
                .sources(vec!["scholar".to_string()]),
        )
        .await?;

    println!("\n--- Scholar Response ---");
    if let Some(answer) = response.answer {
        // Print first 500 chars
        let preview: String = answer.chars().take(500).collect();
        println!("{}...", preview);
    }
    println!("------------------------\n");

    Ok(())
}

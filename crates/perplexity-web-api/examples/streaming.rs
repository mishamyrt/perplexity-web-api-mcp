//! Streaming response example demonstrating real-time event handling.
//!
//! Run with: `cargo run --example streaming`

use futures_util::StreamExt;
use perplexity_web_api::{Client, SearchRequest};

#[tokio::main]
async fn main() -> perplexity_web_api::Result<()> {
    println!("Creating Perplexity client...");

    let client = Client::builder().build().await?;

    println!("Starting streaming query...\n");

    let mut stream = client
        .search_stream(SearchRequest::new("Explain quantum computing in simple terms"))
        .await?;

    let mut chunk_count = 0;
    let mut last_answer: Option<String> = None;

    while let Some(result) = stream.next().await {
        match result {
            Ok(event) => {
                chunk_count += 1;

                if chunk_count % 10 == 0 {
                    println!("[Received {} chunks...]", chunk_count);
                }

                if let Some(answer) = event.answer {
                    last_answer = Some(answer);
                }
            }
            Err(e) => {
                eprintln!("Error: {}", e);
                break;
            }
        }
    }

    println!("\nTotal chunks received: {}", chunk_count);

    if let Some(answer) = last_answer {
        println!("\n--- Final Answer ---");
        println!("{}", answer);
        println!("--------------------");
    }

    Ok(())
}

//! File upload example demonstrating document analysis.
//!
//! Note: File uploads require authentication cookies.
//!
//! Run with: `cargo run --example file_upload`

use perplexity_web_api::{Client, SearchRequest, UploadFile};
use std::collections::HashMap;

#[tokio::main]
async fn main() -> perplexity_web_api::Result<()> {
    println!("=== File Upload Example ===\n");
    println!("Note: File uploads require Perplexity account cookies.");
    println!("See README for instructions on obtaining cookies.\n");

    // To use this example, provide your Perplexity cookies:
    let cookies = HashMap::new();
    // cookies.insert("next-auth.csrf-token".to_string(), "your-token".to_string());
    // cookies.insert("next-auth.session-token".to_string(), "your-session".to_string());

    if cookies.is_empty() {
        println!("No cookies provided. Showing example code only.\n");
        println!("Example code:");
        println!("---");
        println!(
            r#"
// With actual cookies:
let mut cookies = HashMap::new();
cookies.insert("next-auth.csrf-token".to_string(), "your-token".to_string());
cookies.insert("next-auth.session-token".to_string(), "your-session".to_string());

let client = Client::builder()
    .cookies(cookies)
    .build()
    .await?;

// Upload a text file
let response = client.search(
    SearchRequest::new("Summarize this document")
        .file(UploadFile::from_text("document.txt", "Your document content here..."))
).await?;

// Upload bytes (e.g., PDF)
let pdf_bytes = std::fs::read("report.pdf")?;
let response = client.search(
    SearchRequest::new("What are the key findings?")
        .file(UploadFile::from_bytes("report.pdf", pdf_bytes))
).await?;
"#
        );
        println!("---");
        return Ok(());
    }

    // Actual implementation with cookies
    let client = Client::builder().cookies(cookies).build().await?;

    println!("Client created with authentication");

    // Create sample content
    let sample_content = r#"
    Rust is a systems programming language focused on safety, speed, and concurrency.
    It achieves memory safety without garbage collection through its ownership system.
    Key features include:
    - Zero-cost abstractions
    - Move semantics
    - Guaranteed memory safety
    - Threads without data races
    - Trait-based generics
    - Pattern matching
    "#;

    let response = client
        .search(
            SearchRequest::new("What are the main points in this document?")
                .file(UploadFile::from_text("rust_overview.txt", sample_content)),
        )
        .await?;

    println!("\n--- Response ---");
    if let Some(answer) = response.answer {
        println!("{}", answer);
    }
    println!("----------------");

    Ok(())
}

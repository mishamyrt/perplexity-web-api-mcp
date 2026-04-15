//! Upload a file to Perplexity's S3 storage and print the resulting object URL.
//!
//! Run with: cargo run --example with_file

use perplexity_web_api::{AuthCookies, Client, SearchMode, SearchRequest, UploadFile};

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // These values come from the `__Secure-next-auth.session-token`
    // and `next-auth.csrf-token` browser cookies.
    let session_token = std::env::var("PERPLEXITY_SESSION_TOKEN").ok();
    let csrf_token = std::env::var("PERPLEXITY_CSRF_TOKEN").ok();

    let (Some(session_token), Some(csrf_token)) = (session_token, csrf_token) else {
        println!("No tokens provided, exiting.");
        return Ok(());
    };

    let content = r#"
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
    let file = UploadFile::from_text("rust_overview.txt", content);

    let client =
        Client::builder().cookies(AuthCookies::new(session_token, csrf_token)).build().await?;

    let response = client
        .search(
            SearchRequest::new("What are the main points in this document?")
                .mode(SearchMode::Pro) // Auto is not supported for file uploads
                .file(file),
        )
        .await?;

    println!("{}", response.answer.unwrap());

    Ok(())
}

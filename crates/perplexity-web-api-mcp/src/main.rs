//! MCP server exposing Perplexity AI tools for search, research, and reasoning.

mod server;

use perplexity_web_api::Client;
use rmcp::{ServiceExt, transport::stdio};
use std::{collections::HashMap, env};
use tracing_subscriber::{EnvFilter, fmt};

use crate::server::PerplexityServer;

/// Reads a required environment variable or exits with an error.
fn require_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        eprintln!("Error: Required environment variable {} is not set.", name);
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  PERPLEXITY_SESSION_TOKEN=<token> PERPLEXITY_CSRF_TOKEN=<token> perplexity-web-api-mcp");
        eprintln!();
        eprintln!("Required environment variables:");
        eprintln!(
            "  PERPLEXITY_SESSION_TOKEN  - Perplexity session token (next-auth.session-token cookie)"
        );
        eprintln!("  PERPLEXITY_CSRF_TOKEN     - Perplexity CSRF token (next-auth.csrf-token cookie)");
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (logs to stderr to not interfere with stdio transport)
    fmt()
        .with_env_filter(
            EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()),
        )
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Read required environment variables
    let session_token = require_env("PERPLEXITY_SESSION_TOKEN");
    let csrf_token = require_env("PERPLEXITY_CSRF_TOKEN");

    tracing::info!("Starting Perplexity MCP server");

    // Map env vars to Perplexity cookie names
    let mut cookies = HashMap::new();
    cookies.insert("next-auth.session-token".to_string(), session_token);
    cookies.insert("next-auth.csrf-token".to_string(), csrf_token);

    // Build the Perplexity client with authentication
    let client = Client::builder().cookies(cookies).build().await.map_err(|e| {
        eprintln!("Failed to create Perplexity client: {}", e);
        e
    })?;

    tracing::info!("Perplexity client initialized");

    // Create and start the MCP server
    let server = PerplexityServer::new(client);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Server error: {:?}", e);
    })?;

    tracing::info!("MCP server running on stdio");

    service.waiting().await?;

    Ok(())
}

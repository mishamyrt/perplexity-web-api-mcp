//! MCP server exposing Perplexity AI tools for search, research, and reasoning.

mod server;

use perplexity_web_api::{
    AuthCookies, CSRF_TOKEN_COOKIE_NAME, Client, ReasonModel, SESSION_TOKEN_COOKIE_NAME,
    SearchModel,
};
use rmcp::{ServiceExt, transport::stdio};
use std::{env, env::VarError};
use tracing_subscriber::{EnvFilter, fmt};

use crate::server::PerplexityServer;

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{SignalKind, signal};

    match signal(SignalKind::terminate()) {
        Ok(mut sigterm) => {
            tokio::select! {
                _ = tokio::signal::ctrl_c() => {}
                _ = sigterm.recv() => {}
            }
        }
        Err(err) => {
            tracing::warn!("Failed to register SIGTERM handler: {}", err);
            if let Err(ctrl_c_err) = tokio::signal::ctrl_c().await {
                tracing::warn!("Failed to listen for SIGINT: {}", ctrl_c_err);
            }
        }
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    if let Err(err) = tokio::signal::ctrl_c().await {
        tracing::warn!("Failed to listen for shutdown signal: {}", err);
    }
}

/// Reads a required environment variable or returns a descriptive error.
fn require_env(name: &str) -> Result<String, std::io::Error> {
    env::var(name).map_err(|_| {
        std::io::Error::other(format!(
            "Required environment variable {name} is not set.\n\n\
             Usage:\n\
               PERPLEXITY_SESSION_TOKEN=<token> PERPLEXITY_CSRF_TOKEN=<token> perplexity-web-api-mcp\n\n\
             Required environment variables:\n\
               PERPLEXITY_SESSION_TOKEN  - Perplexity session token ({SESSION_TOKEN_COOKIE_NAME} cookie)\n\
               PERPLEXITY_CSRF_TOKEN     - Perplexity CSRF token ({CSRF_TOKEN_COOKIE_NAME} cookie)"
        ))
    })
}

/// Reads an optional default model from environment.
fn optional_model_env<T>(name: &str) -> Result<Option<T>, std::io::Error>
where
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            trimmed.parse::<T>().map(Some).map_err(|e| {
                std::io::Error::other(format!("Invalid environment variable {name}: {e}"))
            })
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(std::io::Error::other(format!(
            "Environment variable {name} must be valid UTF-8"
        ))),
    }
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
    let session_token = require_env("PERPLEXITY_SESSION_TOKEN")?;
    let csrf_token = require_env("PERPLEXITY_CSRF_TOKEN")?;
    let default_search_model = optional_model_env::<SearchModel>("PERPLEXITY_SEARCH_MODEL")?;
    let default_reason_model = optional_model_env::<ReasonModel>("PERPLEXITY_REASON_MODEL")?;

    tracing::info!("Starting Perplexity MCP server");

    let cookies = AuthCookies::new(session_token, csrf_token);

    // Build the Perplexity client with authentication
    let client = Client::builder().cookies(cookies).build().await.map_err(|e| {
        eprintln!("Failed to create Perplexity client: {}", e);
        e
    })?;

    tracing::info!("Perplexity client initialized");

    // Create and start the MCP server
    let server = PerplexityServer::new(client, default_search_model, default_reason_model);

    let service = server.serve(stdio()).await.inspect_err(|e| {
        tracing::error!("Server error: {:?}", e);
    })?;

    tracing::info!("MCP server running on stdio");

    tokio::select! {
        result = service.waiting() => {
            result?;
        }
        _ = shutdown_signal() => {
            tracing::info!("Shutdown signal received, stopping MCP server");
        }
    }

    Ok(())
}

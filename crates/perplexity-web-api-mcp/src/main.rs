//! MCP server exposing Perplexity AI tools for search, research, and reasoning.

mod server;

use perplexity_web_api::{AuthCookies, Client, ReasonModel, SearchModel};
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

/// Reads an optional string environment variable, returning `None` if not present.
fn optional_env(name: &str) -> Result<Option<String>, std::io::Error> {
    match env::var(name) {
        Ok(value) => {
            let trimmed = value.trim().to_owned();
            if trimmed.is_empty() { Ok(None) } else { Ok(Some(trimmed)) }
        }
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(std::io::Error::other(format!(
            "Environment variable {name} must be valid UTF-8"
        ))),
    }
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

    let session_token = optional_env("PERPLEXITY_SESSION_TOKEN")?;
    let csrf_token = optional_env("PERPLEXITY_CSRF_TOKEN")?;
    let tokenless = session_token.is_none() || csrf_token.is_none();

    let (default_ask_model, default_reason_model) = if tokenless {
        // In tokenless mode, model overrides are not supported.
        if env::var("PERPLEXITY_ASK_MODEL").is_ok() {
            return Err(std::io::Error::other(
                "PERPLEXITY_ASK_MODEL cannot be used without authentication tokens.\n\n\
                 To use model configuration, provide both:\n\
                   PERPLEXITY_SESSION_TOKEN  - Perplexity session token\n\
                   PERPLEXITY_CSRF_TOKEN     - Perplexity CSRF token",
            )
            .into());
        }
        if env::var("PERPLEXITY_REASON_MODEL").is_ok() {
            return Err(std::io::Error::other(
                "PERPLEXITY_REASON_MODEL cannot be used without authentication tokens.\n\n\
                 To use model configuration, provide both:\n\
                   PERPLEXITY_SESSION_TOKEN  - Perplexity session token\n\
                   PERPLEXITY_CSRF_TOKEN     - Perplexity CSRF token",
            )
            .into());
        }
        (Some(SearchModel::Turbo), None)
    } else {
        let ask = optional_model_env::<SearchModel>("PERPLEXITY_ASK_MODEL")?
            .unwrap_or(SearchModel::ProAuto);
        let reason = optional_model_env::<ReasonModel>("PERPLEXITY_REASON_MODEL")?;
        (Some(ask), reason)
    };

    if tokenless {
        tracing::info!(
            "Starting Perplexity MCP server in tokenless mode (only perplexity_search and \
             perplexity_ask with turbo model are available)"
        );
    } else {
        tracing::info!("Starting Perplexity MCP server");
    }

    let mut builder = Client::builder();
    if let (Some(session), Some(csrf)) = (session_token, csrf_token) {
        builder = builder.cookies(AuthCookies::new(session, csrf));
    }

    let client = builder.build().await.map_err(|e| {
        tracing::error!("Failed to create Perplexity client: {}", e);
        e
    })?;

    tracing::info!("Perplexity client initialized");

    let server =
        PerplexityServer::new(client, default_ask_model, default_reason_model, tokenless);

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

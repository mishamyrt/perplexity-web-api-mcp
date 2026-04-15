//! Async Rust client library for the Perplexity AI Web API.
//!
//! This crate provides an ergonomic async interface to interact with Perplexity AI's
//! web search API, supporting both streaming and non-streaming responses.
//!
//! # Quick Start
//!
//! ```no_run
//! use perplexity_web_api::{Client, SearchRequest};
//!
//! # async fn example() -> perplexity_web_api::Result<()> {
//! // Create a client
//! let client = Client::builder().build().await?;
//!
//! // Make a simple search query
//! let response = client.search(
//!     SearchRequest::new("What is Rust programming language?")
//! ).await?;
//!
//! if let Some(answer) = response.answer {
//!     println!("{}", answer);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Streaming Responses
//!
//! For real-time responses as they're generated:
//!
//! ```no_run
//! use perplexity_web_api::{Client, SearchRequest};
//! use futures_util::StreamExt;
//!
//! # async fn example() -> perplexity_web_api::Result<()> {
//! let client = Client::builder().build().await?;
//!
//! let mut stream = client.search_stream(
//!     SearchRequest::new("Explain quantum computing")
//! ).await?;
//!
//! while let Some(event) = stream.next().await {
//!     let event = event?;
//!     if let Some(answer) = &event.answer {
//!         println!("{}", answer);
//!     }
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Authentication
//!
//! For enhanced features (pro mode, file uploads), provide your Perplexity cookies:
//!
//! ```no_run
//! use perplexity_web_api::{AuthCookies, Client};
//!
//! # async fn example() -> perplexity_web_api::Result<()> {
//! let cookies = AuthCookies::new("your-session", "your-token");
//!
//! let client = Client::builder()
//!     .cookies(cookies)
//!     .build()
//!     .await?;
//! # Ok(())
//! # }
//! ```
//!
//! # Search Modes
//!
//! - [`SearchMode::Auto`] - Default mode, uses the turbo model
//! - [`SearchMode::Pro`] - Enhanced mode with access to premium models
//! - [`SearchMode::Reasoning`] - Chain-of-thought reasoning models
//! - [`SearchMode::DeepResearch`] - Extended research capabilities
//!
//! # Sources
//!
//! - [`Source::Web`] - General web search (default)
//! - [`Source::Scholar`] - Academic papers and research
//! - [`Source::Social`] - Social media content

mod auth;
mod client;
mod config;
mod error;
mod http;
mod models;
mod parse;
mod sse;
mod types;
mod upload;

pub use auth::{
    AuthCookies, CSRF_TOKEN_COOKIE_NAME, REDACTED_SECRET, SESSION_TOKEN_COOKIE_NAME,
};
pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use models::{ModelPreference, ReasonModel, SearchModel};
pub use types::{
    FollowUpContext, SearchEvent, SearchMode, SearchRequest, SearchResponse, SearchWebResult,
    Source, UploadFile, request_requires_authentication,
};

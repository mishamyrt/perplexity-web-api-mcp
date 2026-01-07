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
//! use perplexity_web_api::Client;
//! use std::collections::HashMap;
//!
//! # async fn example() -> perplexity_web_api::Result<()> {
//! let mut cookies = HashMap::new();
//! cookies.insert("next-auth.csrf-token".to_string(), "your-token".to_string());
//! cookies.insert("next-auth.session-token".to_string(), "your-session".to_string());
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
//! - `"auto"` - Default mode, uses the turbo model
//! - `"pro"` - Enhanced mode with access to premium models
//! - `"reasoning"` - Chain-of-thought reasoning models
//! - `"deep research"` - Extended research capabilities
//!
//! # Sources
//!
//! - `"web"` - General web search (default)
//! - `"scholar"` - Academic papers and research
//! - `"social"` - Social media content

mod client;
mod config;
mod error;
mod parse;
mod sse;
mod types;
mod upload;

pub use client::{Client, ClientBuilder};
pub use error::{Error, Result};
pub use types::{FollowUpContext, SearchEvent, SearchRequest, SearchResponse, UploadFile};

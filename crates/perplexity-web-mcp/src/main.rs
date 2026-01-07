//! MCP server exposing Perplexity AI tools for search, research, and reasoning.
//!
//! Requires environment variables:
//! - `SESSION_TOKEN`: Perplexity session token (maps to `next-auth.session-token` cookie)
//! - `CSRF_TOKEN`: Perplexity CSRF token (maps to `next-auth.csrf-token` cookie)

use perplexity_web_api::{Client, SearchRequest};
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
    transport::stdio,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, sync::Arc};
use tracing_subscriber::{EnvFilter, fmt};

/// Request parameters shared by all Perplexity tools.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityRequest {
    /// The search query or question to ask.
    pub query: String,

    /// Information sources to search. Valid values: "web", "scholar", "social".
    /// Defaults to ["web"] if not specified.
    #[serde(default)]
    pub sources: Option<Vec<String>>,

    /// Language code (ISO 639), e.g., "en-US". Defaults to "en-US".
    #[serde(default)]
    pub language: Option<String>,
}

/// Response from Perplexity tools.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityResponse {
    /// The generated answer text.
    pub answer: Option<String>,

    /// Citation chunks/sources from the response.
    pub chunks: Vec<serde_json::Value>,

    /// Context for making follow-up queries.
    pub follow_up: FollowUpInfo,
}

/// Follow-up context information.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FollowUpInfo {
    /// Backend UUID for follow-up queries.
    pub backend_uuid: Option<String>,

    /// Attachment URLs from the response.
    pub attachments: Vec<String>,
}

/// MCP server wrapping Perplexity AI client.
#[derive(Clone)]
pub struct PerplexityServer {
    client: Arc<Client>,
    tool_router: ToolRouter<Self>,
}

impl PerplexityServer {
    /// Creates a new server instance with the given Perplexity client.
    pub fn new(client: Client) -> Self {
        Self {
            client: Arc::new(client),
            tool_router: Self::tool_router(),
        }
    }

    /// Helper to execute a search with the given mode.
    async fn do_search(&self, params: PerplexityRequest, mode: &str) -> Result<PerplexityResponse, McpError> {
        let mut request = SearchRequest::new(&params.query).mode(mode);

        if let Some(sources) = params.sources {
            if !sources.is_empty() {
                request = request.sources(sources);
            }
        }

        if let Some(language) = params.language {
            request = request.language(language);
        }

        let response = self
            .client
            .search(request)
            .await
            .map_err(|e| McpError::internal_error(format!("Perplexity API error: {}", e), None))?;

        Ok(PerplexityResponse {
            answer: response.answer,
            chunks: response.chunks,
            follow_up: FollowUpInfo {
                backend_uuid: response.follow_up.backend_uuid,
                attachments: response.follow_up.attachments,
            },
        })
    }
}

#[tool_router]
impl PerplexityServer {
    /// Quick web search using Perplexity's turbo model.
    ///
    /// Best for: Quick questions, everyday searches, and conversational queries
    /// that benefit from web context.
    #[tool(
        name = "perplexity_search",
        description = "Quick web search using Perplexity AI. Best for: Quick questions, everyday searches, and conversational queries that benefit from web context."
    )]
    pub async fn perplexity_search(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self.do_search(params, "auto").await?;
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Deep, comprehensive research using Perplexity's sonar-deep-research model.
    ///
    /// Best for: Complex topics requiring detailed investigation, comprehensive reports,
    /// and in-depth analysis. Provides thorough analysis with citations.
    #[tool(
        name = "perplexity_research",
        description = "Deep, comprehensive research using Perplexity AI's sonar-deep-research model. Provides thorough analysis with citations. Best for: Complex topics requiring detailed investigation, comprehensive reports, and in-depth analysis."
    )]
    pub async fn perplexity_research(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self.do_search(params, "deep research").await?;
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }

    /// Advanced reasoning and problem-solving using Perplexity's sonar-reasoning-pro model.
    ///
    /// Best for: Logical problems, complex analysis, decision-making,
    /// and tasks requiring step-by-step reasoning.
    #[tool(
        name = "perplexity_reason",
        description = "Advanced reasoning and problem-solving using Perplexity AI's sonar-reasoning-pro model. Best for: Logical problems, complex analysis, decision-making, and tasks requiring step-by-step reasoning."
    )]
    pub async fn perplexity_reason(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self.do_search(params, "reasoning").await?;
        let json = serde_json::to_string_pretty(&response)
            .map_err(|e| McpError::internal_error(format!("JSON serialization error: {}", e), None))?;
        Ok(CallToolResult::success(vec![Content::text(json)]))
    }
}

#[tool_handler]
impl ServerHandler for PerplexityServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            instructions: Some(
                "Perplexity AI MCP server providing web search, deep research, and reasoning tools. \
                 Use perplexity_search for quick queries, perplexity_research for comprehensive analysis, \
                 and perplexity_reason for logical problem-solving."
                    .into(),
            ),
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            ..Default::default()
        }
    }
}

/// Reads a required environment variable or exits with an error.
fn require_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| {
        eprintln!("Error: Required environment variable {} is not set.", name);
        eprintln!();
        eprintln!("Usage:");
        eprintln!("  SESSION_TOKEN=<token> CSRF_TOKEN=<token> perlexity-web-mcp");
        eprintln!();
        eprintln!("Required environment variables:");
        eprintln!("  SESSION_TOKEN  - Perplexity session token (next-auth.session-token cookie)");
        eprintln!("  CSRF_TOKEN     - Perplexity CSRF token (next-auth.csrf-token cookie)");
        std::process::exit(1);
    })
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing (logs to stderr to not interfere with stdio transport)
    fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive(tracing::Level::INFO.into()))
        .with_writer(std::io::stderr)
        .with_ansi(false)
        .init();

    // Read required environment variables
    let session_token = require_env("SESSION_TOKEN");
    let csrf_token = require_env("CSRF_TOKEN");

    tracing::info!("Starting Perplexity MCP server");

    // Map env vars to Perplexity cookie names
    let mut cookies = HashMap::new();
    cookies.insert("next-auth.session-token".to_string(), session_token);
    cookies.insert("next-auth.csrf-token".to_string(), csrf_token);

    // Build the Perplexity client with authentication
    let client = Client::builder()
        .cookies(cookies)
        .build()
        .await
        .map_err(|e| {
            eprintln!("Failed to create Perplexity client: {}", e);
            e
        })?;

    tracing::info!("Perplexity client initialized");

    // Create and start the MCP server
    let server = PerplexityServer::new(client);

    let service = server
        .serve(stdio())
        .await
        .inspect_err(|e| {
            tracing::error!("Server error: {:?}", e);
        })?;

    tracing::info!("MCP server running on stdio");

    service.waiting().await?;

    Ok(())
}

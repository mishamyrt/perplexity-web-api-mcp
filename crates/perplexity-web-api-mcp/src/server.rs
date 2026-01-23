use perplexity_web_api::{Client, SearchMode, SearchRequest, Source};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

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

/// Parses a source string into a Source enum.
fn parse_source(s: &str) -> Option<Source> {
    match s {
        "web" => Some(Source::Web),
        "scholar" => Some(Source::Scholar),
        "social" => Some(Source::Social),
        _ => None,
    }
}

/// Web result information from search.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct WebResultInfo {
    /// Name/title of the web result.
    pub name: String,

    /// URL of the web result.
    pub url: String,

    /// Snippet/excerpt from the web result.
    pub snippet: String,
}

/// Response from Perplexity tools.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityResponse {
    /// The generated answer text.
    pub answer: Option<String>,

    /// Web search results/sources from the response.
    pub web_results: Vec<WebResultInfo>,

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

/// Converts a `PerplexityResponse` into a `CallToolResult`.
fn response_to_tool_result(response: PerplexityResponse) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(&response).map_err(|e| {
        McpError::internal_error(format!("JSON serialization error: {}", e), None)
    })?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

impl PerplexityServer {
    /// Creates a new server instance with the given Perplexity client.
    pub fn new(client: Client) -> Self {
        Self { client: Arc::new(client), tool_router: Self::tool_router() }
    }

    /// Helper to execute a search with the given mode.
    async fn do_search(
        &self,
        params: PerplexityRequest,
        mode: SearchMode,
    ) -> Result<PerplexityResponse, McpError> {
        let mut request = SearchRequest::new(&params.query).mode(mode).incognito(true);

        if let Some(sources) = params.sources
            && !sources.is_empty()
        {
            let parsed_sources: Vec<Source> =
                sources.iter().filter_map(|s| parse_source(s.as_str())).collect();
            if !parsed_sources.is_empty() {
                request = request.sources(parsed_sources);
            }
        }

        if let Some(language) = params.language {
            request = request.language(language);
        }

        let response = self.client.search(request).await.map_err(|e| {
            McpError::internal_error(format!("Perplexity API error: {}", e), None)
        })?;

        Ok(PerplexityResponse {
            answer: response.answer,
            web_results: response
                .web_results
                .into_iter()
                .map(|r| WebResultInfo { name: r.name, url: r.url, snippet: r.snippet })
                .collect(),
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
        response_to_tool_result(self.do_search(params, SearchMode::Auto).await?)
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
        response_to_tool_result(self.do_search(params, SearchMode::DeepResearch).await?)
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
        response_to_tool_result(self.do_search(params, SearchMode::Reasoning).await?)
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

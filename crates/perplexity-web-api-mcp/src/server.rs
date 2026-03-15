use perplexity_web_api::{
    Client, ModelPreference, ReasonModel, SearchMode, SearchModel, SearchRequest,
    SearchWebResult, Source,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

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

    /// Web search results/sources from the response.
    pub web_results: Vec<SearchWebResult>,

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

/// Search-only response containing just links, titles, and snippets.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct SearchOnlyResponse {
    /// Web search results with titles, URLs, and snippets.
    pub web_results: Vec<SearchWebResult>,
}

/// MCP server wrapping Perplexity AI client.
#[derive(Clone)]
pub struct PerplexityServer {
    client: Client,
    ask_model: Option<SearchModel>,
    reason_model: Option<ReasonModel>,
    tokenless: bool,
    tool_router: ToolRouter<Self>,
}

fn to_json_tool_result(value: &impl Serialize) -> Result<CallToolResult, McpError> {
    let json = serde_json::to_string_pretty(value).map_err(|e| {
        McpError::internal_error(format!("JSON serialization error: {}", e), None)
    })?;
    Ok(CallToolResult::success(vec![Content::text(json)]))
}

impl PerplexityServer {
    /// Creates a new server instance with the given Perplexity client.
    ///
    /// When `tokenless` is `true`, only `perplexity_search` and `perplexity_ask`
    /// (both with the `turbo` model) are registered. The `perplexity_research` and
    /// `perplexity_reason` tools require authenticated session cookies and are
    /// removed from the router.
    pub fn new(
        client: Client,
        ask_model: Option<SearchModel>,
        reason_model: Option<ReasonModel>,
        tokenless: bool,
    ) -> Self {
        let mut tool_router = Self::tool_router();
        if tokenless {
            tool_router.remove_route("perplexity_research");
            tool_router.remove_route("perplexity_reason");
        }
        Self { client, ask_model, reason_model, tokenless, tool_router }
    }

    /// Helper to execute a search with the given mode.
    async fn do_search(
        &self,
        params: PerplexityRequest,
        mode: SearchMode,
        model_preference: Option<ModelPreference>,
    ) -> Result<PerplexityResponse, McpError> {
        let effective_mode = if mode == SearchMode::Auto && model_preference.is_some() {
            SearchMode::Pro
        } else {
            mode
        };

        let mut request =
            SearchRequest::new(&params.query).mode(effective_mode).incognito(true);

        if let Some(model_preference) = model_preference {
            request = request.model(model_preference);
        }

        if let Some(sources) = params.sources
            && !sources.is_empty()
        {
            let parsed_sources: Vec<Source> =
                sources.iter().filter_map(|s| s.parse::<Source>().ok()).collect();
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
        let perplexity_web_api::SearchResponse { answer, web_results, follow_up, .. } =
            response;

        Ok(PerplexityResponse {
            answer,
            web_results,
            follow_up: FollowUpInfo {
                backend_uuid: follow_up.backend_uuid,
                attachments: follow_up.attachments,
            },
        })
    }
}

#[tool_router]
impl PerplexityServer {
    /// Quick web search returning only links, titles, and snippets.
    ///
    /// Always uses the turbo model. No generated answer is included.
    #[tool(
        name = "perplexity_search",
        description = "Search the web and return a ranked list of results with titles, URLs and snippets. \
                Best for: finding specific URLs, checking recent news, verifying facts, discovering sources. \
                Fastest and cheapest option. \
                Returns formatted results (title, URL, snippet) — no AI synthesis. \
                For AI-generated answers with citations, use perplexity_ask instead.",
        annotations(
            title = "Search the Web",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_search(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self.do_search(params, SearchMode::Auto, None).await?;
        to_json_tool_result(&SearchOnlyResponse { web_results: response.web_results })
    }

    /// Ask Perplexity AI a question and get an answer with sources.
    ///
    /// Uses the configured ask model.
    #[tool(
        name = "perplexity_ask",
        description = "Answer a question using web-grounded AI. \
                Best for: quick factual questions, summaries, explanations, and general Q&A. \
                Returns a text response with formatted results (title, URL, snippet). \
                For in-depth multi-source research, use perplexity_research instead. \
                For step-by-step reasoning and analysis, use perplexity_reason instead.",
        annotations(
            title = "Ask Perplexity",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_ask(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self
            .do_search(params, SearchMode::Auto, self.ask_model.map(ModelPreference::from))
            .await?;
        to_json_tool_result(&response)
    }

    /// Deep, comprehensive research using Perplexity's sonar-deep-research model.
    ///
    /// Best for: Complex topics requiring detailed investigation, comprehensive reports,
    /// and in-depth analysis. Provides thorough analysis with citations.
    #[tool(
        name = "perplexity_research",
        description = "Conduct deep, multi-source research on a topic. \
                Best for: literature reviews, comprehensive overviews, investigative queries needing \
                many sources. Returns a detailed response with numbered citations. \
                Significantly slower than other tools (60+ seconds). \
                For quick factual questions, use perplexity_ask instead. \
                For logical analysis and reasoning, use perplexity_reason instead.",
        annotations(
            title = "Deep Research",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_research(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        if self.tokenless {
            return Err(McpError::invalid_request(
                "perplexity_research requires authentication tokens \
                 (PERPLEXITY_SESSION_TOKEN and PERPLEXITY_CSRF_TOKEN)",
                None,
            ));
        }
        to_json_tool_result(&self.do_search(params, SearchMode::DeepResearch, None).await?)
    }

    /// Advanced reasoning and problem-solving using Perplexity's sonar-reasoning-pro model.
    ///
    /// Best for: Logical problems, complex analysis, decision-making,
    /// and tasks requiring step-by-step reasoning.
    #[tool(
        name = "perplexity_reason",
        description = "Analyze a question using step-by-step reasoning with web grounding. \
                Best for: math, logic, comparisons, complex arguments, and tasks requiring chain-of-thought. \
                Returns a reasoned response with numbered citations. \
                For quick factual questions, use perplexity_ask instead. \
                For comprehensive multi-source research, use perplexity_research instead.",
        annotations(
            title = "Advanced Reasoning",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_reason(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        if self.tokenless {
            return Err(McpError::invalid_request(
                "perplexity_reason requires authentication tokens \
                 (PERPLEXITY_SESSION_TOKEN and PERPLEXITY_CSRF_TOKEN)",
                None,
            ));
        }
        to_json_tool_result(
            &self
                .do_search(
                    params,
                    SearchMode::Reasoning,
                    self.reason_model.map(ModelPreference::from),
                )
                .await?,
        )
    }
}

#[tool_handler]
impl ServerHandler for PerplexityServer {
    fn get_info(&self) -> ServerInfo {
        let mut instructions = String::from(
            "Perplexity AI server for web-grounded search. \
            All tools are read-only and access live web data. \
            Use perplexity_search for finding URLs, facts, and recent news. \
            Use perplexity_ask for quick AI-answered questions with citations.",
        );
        if !self.tokenless {
            instructions.push_str(
                " Use perplexity_research for in-depth multi-source investigation (slow, 60s+). \
                Use perplexity_reason for complex analysis requiring step-by-step logic.",
            );
        }

        let server_info =
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(instructions)
            .with_server_info(server_info)
    }
}

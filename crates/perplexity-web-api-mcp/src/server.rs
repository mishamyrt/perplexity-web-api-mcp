use base64::Engine as _;
use perplexity_web_api::{
    Client, ComputerModel, ModelPreference, ReasonModel, SearchMode, SearchModel, SearchRequest,
    SearchWebResult, Source, UploadFile,
};
use rmcp::{
    ErrorData as McpError, ServerHandler,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content, Implementation, ServerCapabilities, ServerInfo},
    schemars, tool, tool_handler, tool_router,
};
use serde::{Deserialize, Serialize};

/// A file to attach to the query for document analysis.
/// Requires authentication tokens. Provide either `text` or `data`, not both.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct FileAttachment {
    /// Filename with extension, e.g. "report.pdf" or "notes.txt".
    pub filename: String,

    /// Plain-text file content. Use for text files (.txt, .md, .csv, .json, source code).
    /// Mutually exclusive with `data`.
    #[serde(default)]
    pub text: Option<String>,

    /// Base64-encoded binary file content. Use for binary files (.pdf, .docx, images).
    /// Mutually exclusive with `text`.
    #[serde(default)]
    pub data: Option<String>,
}

/// Request parameters for `perplexity_search` (no file attachments).
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexitySearchRequest {
    /// The search query or question to ask.
    pub query: String,

    /// Information sources to search.
    /// Standard (no auth): "web", "scholar", "social".
    /// Connectors (require auth + connected account): "google_drive", "gcal",
    /// "outlook", "notion_mcp", "github_mcp_direct", "linear_alt",
    /// "slack_direct", "jira_mcp_merge", "confluence_mcp_merge",
    /// "microsoft_teams_mcp_merge", "onedrive", "sharepoint", "dropbox", "box".
    /// Defaults to ["web"] if not specified.
    #[serde(default)]
    pub sources: Option<Vec<String>>,

    /// Language code (ISO 639), e.g., "en-US". Defaults to "en-US".
    #[serde(default)]
    pub language: Option<String>,
}

/// Request parameters for AI-powered tools that support file attachments.
#[derive(Debug, Clone, Serialize, Deserialize, schemars::JsonSchema)]
pub struct PerplexityRequest {
    /// The search query or question to ask.
    pub query: String,

    /// Information sources to search.
    /// Standard (no auth): "web", "scholar", "social".
    /// Connectors (require auth + connected account): "google_drive", "gcal",
    /// "outlook", "notion_mcp", "github_mcp_direct", "linear_alt",
    /// "slack_direct", "jira_mcp_merge", "confluence_mcp_merge",
    /// "microsoft_teams_mcp_merge", "onedrive", "sharepoint", "dropbox", "box".
    /// Defaults to ["web"] if not specified.
    #[serde(default)]
    pub sources: Option<Vec<String>>,

    /// Language code (ISO 639), e.g., "en-US". Defaults to "en-US".
    #[serde(default)]
    pub language: Option<String>,

    /// Optional file attachments for document analysis.
    /// Requires authentication tokens (PERPLEXITY_SESSION_TOKEN + PERPLEXITY_CSRF_TOKEN).
    /// Each entry needs `filename` and either `text` (plain text) or `data` (base64 binary).
    #[serde(default)]
    pub files: Option<Vec<FileAttachment>>,
}

impl From<PerplexitySearchRequest> for PerplexityRequest {
    fn from(r: PerplexitySearchRequest) -> Self {
        Self { query: r.query, sources: r.sources, language: r.language, files: None }
    }
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
    computer_model: Option<ComputerModel>,
    tokenless: bool,
    incognito: bool,
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
        computer_model: Option<ComputerModel>,
        tokenless: bool,
        incognito: bool,
    ) -> Self {
        Self { client, ask_model, reason_model, computer_model, tokenless, incognito }
    }

    /// Converts a `FileAttachment` from tool parameters into an `UploadFile`.
    fn convert_attachment(attachment: FileAttachment) -> Result<UploadFile, McpError> {
        match (attachment.text, attachment.data) {
            (Some(text), None) => Ok(UploadFile::from_text(attachment.filename, text)),
            (None, Some(b64)) => {
                let bytes =
                    base64::engine::general_purpose::STANDARD.decode(&b64).map_err(|e| {
                        McpError::invalid_params(
                            format!(
                                "Failed to decode base64 data for '{}': {}",
                                attachment.filename, e
                            ),
                            None,
                        )
                    })?;
                Ok(UploadFile::from_bytes(attachment.filename, bytes))
            }
            (Some(_), Some(_)) => Err(McpError::invalid_params(
                format!(
                    "File '{}' has both `text` and `data` set; provide only one.",
                    attachment.filename
                ),
                None,
            )),
            (None, None) => Err(McpError::invalid_params(
                format!(
                    "File '{}' has neither `text` nor `data` set; provide one.",
                    attachment.filename
                ),
                None,
            )),
        }
    }

    /// Helper to execute a search with the given mode.
    ///
    /// When `files_allowed` is `false`, the method rejects any request that
    /// contains file attachments with a clear error before doing anything else.
    async fn do_search(
        &self,
        params: PerplexityRequest,
        mode: SearchMode,
        model_preference: Option<ModelPreference>,
        files_allowed: bool,
    ) -> Result<PerplexityResponse, McpError> {
        let files: Vec<UploadFile> = if let Some(attachments) = params.files {
            if !attachments.is_empty() {
                if !files_allowed {
                    return Err(McpError::invalid_params(
                        "This tool does not support file attachments. \
                         Use perplexity_ask, perplexity_research, or perplexity_reason instead.",
                        None,
                    ));
                }
                if self.tokenless {
                    return Err(McpError::invalid_params(
                        "File attachments require authentication tokens. \
                         Set PERPLEXITY_SESSION_TOKEN and PERPLEXITY_CSRF_TOKEN.",
                        None,
                    ));
                }
                attachments
                    .into_iter()
                    .map(Self::convert_attachment)
                    .collect::<Result<Vec<_>, _>>()?
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let has_files = !files.is_empty();

        let effective_mode =
            if mode == SearchMode::Auto && (model_preference.is_some() || has_files) {
                SearchMode::Pro
            } else {
                mode
            };

        let mut request =
            SearchRequest::new(&params.query).mode(effective_mode).incognito(self.incognito);

        if let Some(model_preference) = model_preference {
            request = request.model(model_preference);
        }

        for file in files {
            request = request.file(file);
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
        Parameters(params): Parameters<PerplexitySearchRequest>,
    ) -> Result<CallToolResult, McpError> {
        let response = self.do_search(params.into(), SearchMode::Auto, None, false).await?;
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
                For step-by-step reasoning and analysis, use perplexity_reason instead. \
                Supports optional file attachments via the `files` parameter (requires authentication token).",
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
            .do_search(
                params,
                SearchMode::Auto,
                self.ask_model.map(ModelPreference::from),
                true,
            )
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
                For logical analysis and reasoning, use perplexity_reason instead. \
                Supports optional file attachments via the `files` parameter (requires authentication token).",
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
        to_json_tool_result(
            &self.do_search(params, SearchMode::DeepResearch, None, true).await?,
        )
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
                For comprehensive multi-source research, use perplexity_research instead. \
                Supports optional file attachments via the `files` parameter (requires authentication token).",
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
        to_json_tool_result(
            &self
                .do_search(
                    params,
                    SearchMode::Reasoning,
                    self.reason_model.map(ModelPreference::from),
                    true,
                )
                .await?,
        )
    }

    /// Perplexity Computer, an agentic AI that can browse the web, run code, and use connected services.
    ///
    /// Best for: Tasks that require the LLM to take actions, browse the web,
    /// run code, or interact with connected services.
    #[tool(
        name = "perplexity_computer",
        description = "Execute a task using Perplexity Computer, an agentic AI that can browse the web, \
                run code, and use connected services. \
                Best for: multi-step tasks, data analysis with connectors (Google Drive, Calendar, Notion, GitHub), \
                web automation, and tasks requiring tool use. \
                Requires authentication tokens. Significantly slower than other tools (30+ seconds). \
                Sources can include connected services like google_drive, gcal, notion_mcp, github_mcp_direct, etc. \
                Supports optional file attachments via the `files` parameter.",
        annotations(
            title = "Perplexity Computer",
            read_only_hint = false,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_computer(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        to_json_tool_result(
            &self
                .do_search(
                    params,
                    SearchMode::Computer,
                    self.computer_model.map(ModelPreference::from),
                    true,
                )
                .await?,
        )
    }

    /// Study mode — tutor-style explanations with step-by-step teaching.
    ///
    /// Best for: learning new concepts, guided explanations, and educational
    /// breakdowns of complex topics.
    #[tool(
        name = "perplexity_study",
        description = "Answer a question in tutor-style study mode with step-by-step, \
                pedagogical explanations. \
                Best for: learning new concepts, guided walkthroughs, test prep, \
                and educational breakdowns. \
                Requires authentication tokens. \
                Supports optional file attachments via the `files` parameter.",
        annotations(
            title = "Study Mode",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_study(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        to_json_tool_result(&self.do_search(params, SearchMode::Study, None, true).await?)
    }

    /// Document review mode — detailed analysis of uploaded documents.
    ///
    /// Best for: extracting findings, summarizing long documents, and reviewing
    /// contracts, papers, or reports with uploaded files.
    #[tool(
        name = "perplexity_document_review",
        description = "Perform detailed analysis and review of uploaded documents. \
                Best for: contract review, paper analysis, long-document summarization, \
                and extracting structured findings from PDFs or text files. \
                Requires authentication tokens. \
                Attach the document(s) via the `files` parameter (required for best results).",
        annotations(
            title = "Document Review",
            read_only_hint = true,
            open_world_hint = true,
            destructive_hint = false,
            idempotent_hint = false
        )
    )]
    pub async fn perplexity_document_review(
        &self,
        Parameters(params): Parameters<PerplexityRequest>,
    ) -> Result<CallToolResult, McpError> {
        to_json_tool_result(
            &self.do_search(params, SearchMode::DocumentReview, None, true).await?,
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
                Use perplexity_reason for complex analysis requiring step-by-step logic. \
                Use perplexity_computer for agentic tasks with tool use and connected services. \
                Use perplexity_study for tutor-style step-by-step explanations. \
                Use perplexity_document_review for detailed analysis of uploaded documents. \
                All tools accept a `sources` parameter for connector-scoped queries: \
                standard sources (web, scholar, social) work without auth; \
                connectors (google_drive, gcal, outlook, notion_mcp, github_mcp_direct, etc.) \
                require authentication and a connected account. \
                All tools support an optional `files` parameter for document analysis: \
                pass an array of objects each with `filename` and either `text` (plain-text content) \
                or `data` (base64-encoded binary content, e.g. for PDFs).",
            );
        }

        let server_info =
            Implementation::new(env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
        ServerInfo::new(ServerCapabilities::builder().enable_tools().build())
            .with_instructions(instructions)
            .with_server_info(server_info)
    }
}

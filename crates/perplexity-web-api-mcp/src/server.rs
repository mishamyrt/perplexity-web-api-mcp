use base64::Engine as _;
use perplexity_web_api::{
    Client, ModelPreference, ReasonModel, SearchMode, SearchModel, SearchRequest,
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
/// Requires an authenticated Perplexity session. Provide either `text` or `data`, not both.
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

    /// Information sources to search. Valid values: "web", "scholar", "social".
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

    /// Information sources to search. Valid values: "web", "scholar", "social".
    /// Defaults to ["web"] if not specified.
    #[serde(default)]
    pub sources: Option<Vec<String>>,

    /// Language code (ISO 639), e.g., "en-US". Defaults to "en-US".
    #[serde(default)]
    pub language: Option<String>,

    /// Optional file attachments for document analysis.
    /// Requires an authenticated Perplexity session from environment variables or saved local setup.
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
        tokenless: bool,
        incognito: bool,
    ) -> Self {
        Self { client, ask_model, reason_model, tokenless, incognito }
    }

    /// Converts a `FileAttachment` from tool parameters into an `UploadFile`.
    fn convert_attachment(attachment: FileAttachment) -> Result<UploadFile, McpError> {
        if attachment.filename.trim().is_empty() {
            return Err(McpError::invalid_params(
                "Each file attachment must include a non-empty filename.",
                None,
            ));
        }

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

    fn parse_sources(sources: Vec<String>) -> Result<Vec<Source>, McpError> {
        sources
            .into_iter()
            .map(|source| {
                source.parse::<Source>().map_err(|err| {
                    McpError::invalid_params(format!("Invalid source '{source}': {err}"), None)
                })
            })
            .collect()
    }

    /// Helper to execute a search with the given mode.
    ///
    /// When `files_allowed` is `false`, the method rejects any request that
    /// contains file attachments with a clear error before doing anything else.
    fn build_search_request(
        params: PerplexityRequest,
        mode: SearchMode,
        model_preference: Option<ModelPreference>,
        files_allowed: bool,
        tokenless: bool,
        incognito: bool,
    ) -> Result<SearchRequest, McpError> {
        if params.query.trim().is_empty() {
            return Err(McpError::invalid_params("Query must be a non-empty string.", None));
        }

        let files: Vec<UploadFile> = if let Some(attachments) = params.files {
            if !attachments.is_empty() {
                if !files_allowed {
                    return Err(McpError::invalid_params(
                        "This tool does not support file attachments. \
                         Use perplexity_ask, perplexity_research, or perplexity_reason instead.",
                        None,
                    ));
                }
                if tokenless {
                    return Err(McpError::invalid_params(
                        "File attachments require an authenticated Perplexity session. \
                         Provide PERPLEXITY_SESSION_TOKEN and PERPLEXITY_CSRF_TOKEN, or run the MCP binary once in an interactive terminal to complete local setup.",
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

        let needs_authenticated_mode = has_files
            || perplexity_web_api::request_requires_authentication(mode, model_preference);
        let effective_mode = if mode == SearchMode::Auto && needs_authenticated_mode {
            SearchMode::Pro
        } else {
            mode
        };

        let mut request =
            SearchRequest::new(&params.query).mode(effective_mode).incognito(incognito);

        if let Some(model_preference) = model_preference {
            request = request.model(model_preference);
        }

        for file in files {
            request = request.file(file);
        }

        if let Some(sources) = params.sources {
            if sources.is_empty() {
                return Err(McpError::invalid_params(
                    "If provided, `sources` must contain at least one value.",
                    None,
                ));
            }
            request = request.sources(Self::parse_sources(sources)?);
        }

        if let Some(language) = params.language {
            if language.trim().is_empty() {
                return Err(McpError::invalid_params(
                    "If provided, `language` must be a non-empty string.",
                    None,
                ));
            }
            request = request.language(language);
        }

        Ok(request)
    }

    async fn do_search(
        &self,
        params: PerplexityRequest,
        mode: SearchMode,
        model_preference: Option<ModelPreference>,
        files_allowed: bool,
    ) -> Result<PerplexityResponse, McpError> {
        let request = Self::build_search_request(
            params,
            mode,
            model_preference,
            files_allowed,
            self.tokenless,
            self.incognito,
        )?;

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

#[cfg(test)]
#[allow(clippy::items_after_test_module)]
mod tests {
    use super::{FileAttachment, PerplexityRequest, PerplexityServer};
    use perplexity_web_api::{SearchMode, SearchModel};
    use rmcp::ErrorData as McpError;

    #[test]
    fn rejects_empty_query() {
        let error = build_request(PerplexityRequest {
            query: "   ".into(),
            sources: None,
            language: None,
            files: None,
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "Query must be a non-empty string.");
    }

    #[test]
    fn rejects_empty_language() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: None,
            language: Some("   ".into()),
            files: None,
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "`language` must be a non-empty string");
    }

    #[test]
    fn rejects_empty_sources() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: Some(Vec::new()),
            language: None,
            files: None,
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "`sources` must contain at least one value");
    }

    #[test]
    fn rejects_invalid_source() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: Some(vec!["books".into()]),
            language: None,
            files: None,
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "Invalid source 'books'");
    }

    #[test]
    fn rejects_attachment_with_both_text_and_data() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: None,
            language: None,
            files: Some(vec![FileAttachment {
                filename: "notes.txt".into(),
                text: Some("hello".into()),
                data: Some("aGVsbG8=".into()),
            }]),
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "has both `text` and `data` set");
    }

    #[test]
    fn rejects_attachment_with_neither_text_nor_data() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: None,
            language: None,
            files: Some(vec![FileAttachment {
                filename: "notes.txt".into(),
                text: None,
                data: None,
            }]),
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "has neither `text` nor `data` set");
    }

    #[test]
    fn rejects_attachment_with_empty_filename() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: None,
            language: None,
            files: Some(vec![FileAttachment {
                filename: "   ".into(),
                text: Some("hello".into()),
                data: None,
            }]),
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "must include a non-empty filename");
    }

    #[test]
    fn rejects_invalid_base64_attachment() {
        let error = build_request(PerplexityRequest {
            query: "hello".into(),
            sources: None,
            language: None,
            files: Some(vec![FileAttachment {
                filename: "blob.bin".into(),
                text: None,
                data: Some("%%%".into()),
            }]),
        })
        .unwrap_err();

        assert_invalid_params_contains(error, "Failed to decode base64 data");
    }

    #[test]
    fn tokenless_request_allows_explicit_turbo_model() {
        PerplexityServer::build_search_request(
            PerplexityRequest {
                query: "hello".into(),
                sources: None,
                language: None,
                files: None,
            },
            SearchMode::Auto,
            Some(SearchModel::Turbo.into()),
            true,
            true,
            true,
        )
        .unwrap();
    }

    fn build_request(
        params: PerplexityRequest,
    ) -> Result<perplexity_web_api::SearchRequest, McpError> {
        PerplexityServer::build_search_request(
            params,
            SearchMode::Auto,
            None,
            true,
            false,
            true,
        )
    }

    fn assert_invalid_params_contains(error: McpError, needle: &str) {
        assert!(error.to_string().contains(needle), "{error}");
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
                Supports optional file attachments via the `files` parameter (requires authenticated session).",
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
                Supports optional file attachments via the `files` parameter (requires authenticated session).",
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
                Supports optional file attachments via the `files` parameter (requires authenticated session).",
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

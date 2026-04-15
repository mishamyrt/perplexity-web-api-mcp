use crate::models::{DEEP_RESEARCH_MODEL_PREFERENCE, ModelPreference};
use crate::{ReasonModel, SearchModel};
use bytes::Bytes;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;
use std::str::FromStr;

/// Search mode for Perplexity queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchMode {
    /// Default mode using the turbo model.
    #[default]
    Auto,
    /// Enhanced mode with access to premium models.
    Pro,
    /// Chain-of-thought reasoning models.
    Reasoning,
    /// Extended research capabilities.
    DeepResearch,
}

impl SearchMode {
    /// Returns the default model preference for this mode.
    pub const fn default_preference(&self) -> &'static str {
        match self {
            Self::Auto => SearchModel::Turbo.api_preference().as_str(),
            Self::Pro => SearchModel::ProAuto.api_preference().as_str(),
            Self::Reasoning => ReasonModel::Gemini31Pro.api_preference().as_str(),
            Self::DeepResearch => DEEP_RESEARCH_MODEL_PREFERENCE,
        }
    }
}

/// Returns whether the requested mode or explicit model selection requires
/// authenticated Perplexity session cookies.
pub fn request_requires_authentication(
    mode: SearchMode,
    model_preference: Option<ModelPreference>,
) -> bool {
    matches!(mode, SearchMode::Pro | SearchMode::Reasoning | SearchMode::DeepResearch)
        || model_preference.is_some_and(|preference| preference != SearchModel::Turbo.into())
}

/// Information source for search queries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Source {
    /// General web search (default).
    #[default]
    Web,
    /// Academic papers and research.
    Scholar,
    /// Social media content.
    Social,
}

impl Source {
    /// Returns the string representation used by the API.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Web => "web",
            Self::Scholar => "scholar",
            Self::Social => "social",
        }
    }
}

impl fmt::Display for Source {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Source {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "web" => Ok(Self::Web),
            "scholar" => Ok(Self::Scholar),
            "social" => Ok(Self::Social),
            _ => Err(format!("unknown source '{s}', expected one of: web, scholar, social")),
        }
    }
}

impl TryFrom<&str> for Source {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value.parse()
    }
}

/// A file to be uploaded with a search query.
#[derive(Debug, Clone)]
pub enum UploadFile {
    /// File contents as bytes with a filename.
    Binary { filename: String, data: Bytes },
    /// File contents as text with a filename.
    Text { filename: String, content: String },
}

impl UploadFile {
    /// Creates an `UploadFile` from bytes.
    pub fn from_bytes(filename: impl Into<String>, data: impl Into<Bytes>) -> Self {
        Self::Binary { filename: filename.into(), data: data.into() }
    }

    /// Creates an `UploadFile` from text content.
    pub fn from_text(filename: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Text { filename: filename.into(), content: content.into() }
    }

    pub(crate) fn filename(&self) -> &str {
        match self {
            Self::Binary { filename, .. } | Self::Text { filename, .. } => filename,
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Binary { data, .. } => data.len(),
            Self::Text { content, .. } => content.len(),
        }
    }
}

/// Parameters for a search request.
#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    /// The search query string.
    pub query: String,
    /// Search mode: Auto, Pro, Reasoning, or DeepResearch.
    pub mode: SearchMode,
    /// Optional explicit model preference.
    pub model_preference: Option<ModelPreference>,
    /// Information sources: Web, Scholar, Social.
    pub sources: Vec<Source>,
    /// Files to upload with the query.
    pub files: Vec<UploadFile>,
    /// Language code (ISO 639), e.g., "en-US".
    pub language: String,
    /// Context from a previous query for follow-up.
    pub follow_up: Option<FollowUpContext>,
    /// Whether to enable incognito mode.
    pub incognito: bool,
}

impl SearchRequest {
    /// Creates a new search request with the given query.
    pub fn new(query: impl Into<String>) -> Self {
        Self {
            query: query.into(),
            mode: SearchMode::Auto,
            model_preference: None,
            sources: vec![Source::Web],
            files: Vec::new(),
            language: "en-US".to_string(),
            follow_up: None,
            incognito: false,
        }
    }

    /// Sets the search mode.
    pub fn mode(mut self, mode: SearchMode) -> Self {
        self.mode = mode;
        self
    }

    /// Sets the model to use.
    pub fn model(mut self, model: impl Into<ModelPreference>) -> Self {
        self.model_preference = Some(model.into());
        self
    }

    /// Sets the information sources.
    pub fn sources(mut self, sources: Vec<Source>) -> Self {
        self.sources = sources;
        self
    }

    /// Adds a file to upload.
    pub fn file(mut self, file: UploadFile) -> Self {
        self.files.push(file);
        self
    }

    /// Sets the language.
    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    /// Sets the follow-up context from a previous query.
    pub fn follow_up(mut self, context: FollowUpContext) -> Self {
        self.follow_up = Some(context);
        self
    }

    /// Enables or disables incognito mode.
    pub fn incognito(mut self, incognito: bool) -> Self {
        self.incognito = incognito;
        self
    }
}

/// Context for follow-up queries, extracted from a previous response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FollowUpContext {
    /// Backend UUID from the previous response.
    pub backend_uuid: Option<String>,
    /// Attachment URLs from the previous response.
    pub attachments: Vec<String>,
}

/// A single event from the SSE stream.
#[derive(Debug, Clone, Serialize)]
pub struct SearchEvent {
    /// The extracted answer text, if available.
    #[serde(default)]
    pub answer: Option<String>,
    /// Web search results from the response, if available.
    #[serde(default)]
    pub web_results: Vec<SearchWebResult>,
    /// Backend UUID for follow-up queries.
    #[serde(default)]
    pub backend_uuid: Option<String>,
    /// Attachment URLs associated with this response.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// The full raw JSON value from the SSE event.
    pub raw: serde_json::Value,
}

impl SearchEvent {
    /// Creates a follow-up context from this event for chained queries.
    pub fn as_follow_up(&self) -> FollowUpContext {
        FollowUpContext {
            backend_uuid: self.backend_uuid.clone(),
            attachments: self.attachments.clone(),
        }
    }
}

#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchWebResult {
    pub name: String,
    pub url: String,
    pub snippet: String,
}

/// The final response from a non-streaming search.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    /// The final answer text.
    pub answer: Option<String>,
    /// Web search results from the response.
    pub web_results: Vec<SearchWebResult>,
    /// Context for making follow-up queries.
    pub follow_up: FollowUpContext,
    /// The last raw event from the stream.
    pub raw: serde_json::Value,
}

#[derive(Serialize)]
pub(crate) struct AskPayload<'a> {
    pub query_str: &'a str,
    pub params: AskParams<'a>,
}

#[derive(Serialize)]
pub(crate) struct AskParams<'a> {
    pub attachments: Vec<String>,
    pub frontend_context_uuid: String,
    pub frontend_uuid: String,
    pub is_incognito: bool,
    pub language: &'a str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backend_uuid: Option<String>,
    pub mode: &'static str,
    pub model_preference: &'static str,
    pub source: &'static str,
    pub sources: Vec<&'static str>,
    pub version: &'static str,
}

#[derive(Serialize)]
pub(crate) struct BatchUploadFileInfo {
    pub filename: String,
    pub content_type: String,
    pub source: String,
    pub file_size: usize,
    pub force_image: bool,
    pub skip_parsing: bool,
    pub persistent_upload: bool,
}

/// Per-file results from the batch upload URL endpoint.
#[derive(Deserialize)]
pub(crate) struct BatchUploadFileResults {
    pub fields: HashMap<String, String>,
    pub s3_bucket_url: String,
    pub s3_object_url: String,
    pub file_uuid: String,
}

/// Per-file response from the batch upload URL endpoint.
#[derive(Deserialize)]
pub(crate) struct BatchUploadFileResponse {
    /// Results for each uploaded file, keyed by the file's UUID.
    pub results: HashMap<String, BatchUploadFileResults>,
}

pub(crate) struct BatchUploadFileMeta {
    pub s3_object_url: String,
    pub uuid: String,
}

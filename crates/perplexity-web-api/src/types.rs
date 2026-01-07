use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A file to be uploaded with a search query.
#[derive(Debug, Clone)]
pub enum UploadFile {
    /// File contents as bytes with a filename.
    Bytes { filename: String, data: Vec<u8> },
    /// File contents as text with a filename.
    Text { filename: String, content: String },
}

impl UploadFile {
    /// Creates an `UploadFile` from bytes.
    pub fn from_bytes(filename: impl Into<String>, data: impl Into<Vec<u8>>) -> Self {
        Self::Bytes {
            filename: filename.into(),
            data: data.into(),
        }
    }

    /// Creates an `UploadFile` from text content.
    pub fn from_text(filename: impl Into<String>, content: impl Into<String>) -> Self {
        Self::Text {
            filename: filename.into(),
            content: content.into(),
        }
    }

    pub(crate) fn filename(&self) -> &str {
        match self {
            Self::Bytes { filename, .. } | Self::Text { filename, .. } => filename,
        }
    }

    pub(crate) fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Bytes { data, .. } => data,
            Self::Text { content, .. } => content.as_bytes(),
        }
    }

    pub(crate) fn len(&self) -> usize {
        match self {
            Self::Bytes { data, .. } => data.len(),
            Self::Text { content, .. } => content.len(),
        }
    }
}

/// Parameters for a search request.
#[derive(Debug, Clone, Default)]
pub struct SearchRequest {
    /// The search query string.
    pub query: String,
    /// Search mode: "auto", "pro", "reasoning", or "deep research".
    pub mode: String,
    /// Optional model to use for the query.
    pub model: Option<String>,
    /// Information sources: "web", "scholar", "social".
    pub sources: Vec<String>,
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
            mode: "auto".to_string(),
            model: None,
            sources: vec!["web".to_string()],
            files: Vec::new(),
            language: "en-US".to_string(),
            follow_up: None,
            incognito: false,
        }
    }

    /// Sets the search mode.
    pub fn mode(mut self, mode: impl Into<String>) -> Self {
        self.mode = mode.into();
        self
    }

    /// Sets the model to use.
    pub fn model(mut self, model: impl Into<String>) -> Self {
        self.model = Some(model.into());
        self
    }

    /// Sets the information sources.
    pub fn sources(mut self, sources: Vec<String>) -> Self {
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FollowUpContext {
    /// Backend UUID from the previous response.
    pub backend_uuid: Option<String>,
    /// Attachment URLs from the previous response.
    pub attachments: Vec<String>,
}

impl FollowUpContext {
    /// Creates a new empty follow-up context.
    pub fn new() -> Self {
        Self {
            backend_uuid: None,
            attachments: Vec::new(),
        }
    }
}

impl Default for FollowUpContext {
    fn default() -> Self {
        Self::new()
    }
}

/// A single event from the SSE stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchEvent {
    /// The extracted answer text, if available.
    #[serde(default)]
    pub answer: Option<String>,
    /// Chunks/citations from the response, if available.
    #[serde(default)]
    pub chunks: Vec<serde_json::Value>,
    /// Backend UUID for follow-up queries.
    #[serde(default)]
    pub backend_uuid: Option<String>,
    /// Attachment URLs associated with this response.
    #[serde(default)]
    pub attachments: Vec<String>,
    /// The raw JSON value from the SSE event.
    #[serde(flatten)]
    pub raw: HashMap<String, serde_json::Value>,
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

/// The final response from a non-streaming search.
#[derive(Debug, Clone)]
pub struct SearchResponse {
    /// The final answer text.
    pub answer: Option<String>,
    /// Chunks/citations from the response.
    pub chunks: Vec<serde_json::Value>,
    /// Context for making follow-up queries.
    pub follow_up: FollowUpContext,
    /// The last raw event from the stream.
    pub raw: serde_json::Value,
}

#[derive(Serialize)]
pub(crate) struct AskPayload {
    pub query_str: String,
    pub params: AskParams,
}

#[derive(Serialize)]
pub(crate) struct AskParams {
    pub attachments: Vec<String>,
    pub frontend_context_uuid: String,
    pub frontend_uuid: String,
    pub is_incognito: bool,
    pub language: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backend_uuid: Option<String>,
    pub mode: String,
    pub model_preference: String,
    pub source: String,
    pub sources: Vec<String>,
    pub version: String,
}

#[derive(Serialize)]
pub(crate) struct UploadUrlRequest {
    pub content_type: String,
    pub file_size: usize,
    pub filename: String,
    pub force_image: bool,
    pub source: String,
}

#[derive(Deserialize)]
pub(crate) struct UploadUrlResponse {
    pub fields: HashMap<String, String>,
    pub s3_bucket_url: String,
    pub s3_object_url: String,
}

#[derive(Deserialize)]
pub(crate) struct S3UploadResponse {
    pub secure_url: Option<String>,
}

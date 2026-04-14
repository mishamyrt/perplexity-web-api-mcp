use std::time::Duration;
use thiserror::Error;

/// All possible errors that can occur when using the Perplexity client.
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP client initialization failed.
    #[error("HTTP client initialization failed: {0}")]
    HttpClientInit(#[source] rquest::Error),

    /// Session warm-up request failed.
    #[error("Session warmup failed: {0}")]
    SessionWarmup(#[source] rquest::Error),

    /// Authenticated startup did not yield a logged-in session.
    #[error("Perplexity authentication cookies were rejected")]
    AuthenticationFailed,

    /// Session warm-up returned an unexpected payload.
    #[error("Perplexity session warmup returned an unexpected response")]
    InvalidAuthenticationResponse,

    /// Search request failed.
    #[error("Search request failed: {0}")]
    SearchRequest(#[source] rquest::Error),

    /// File upload request failed.
    #[error("Upload request failed: {0}")]
    UploadRequest(#[source] rquest::Error),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Request timed out.
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    /// File uploads require authentication cookies.
    #[error("File uploads require authentication cookies")]
    FileUploadRequiresAuth,

    /// Premium search modes require authentication cookies.
    #[error(
        "Premium search modes and explicit model selection require authentication cookies"
    )]
    AuthenticatedModeRequiresAuth,

    /// Failed to get upload URL.
    #[error("Failed to get upload URL: {0}")]
    UploadUrlFailed(#[source] rquest::Error),

    /// S3 upload failed.
    #[error("S3 upload failed: {0}")]
    S3UploadFailed(#[source] rquest::Error),

    /// Batch upload response did not contain the expected file entry.
    #[error("Missing file entry in batch upload response")]
    MissingUploadResponse,

    /// Attachment processing SSE subscription or streaming failed.
    #[error("Attachment processing failed: {0}")]
    AttachmentProcessing(#[source] rquest::Error),

    /// Invalid MIME type.
    #[error("Invalid MIME type: {0}")]
    InvalidMimeType(String),

    /// Invalid UTF-8 in SSE stream.
    #[error("Invalid UTF-8 in SSE stream")]
    InvalidUtf8,

    /// Server returned an error response.
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    /// Stream ended unexpectedly.
    #[error("Stream ended unexpectedly")]
    UnexpectedEndOfStream,

    #[error("Invalid API base url")]
    InvalidBaseUrl,
}

/// Convenience Result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

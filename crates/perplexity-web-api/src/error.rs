use std::time::Duration;
use thiserror::Error;

/// All possible errors that can occur when using the Perplexity client.
#[derive(Error, Debug)]
pub enum Error {
    /// HTTP request failed.
    #[error("HTTP error: {0}")]
    Http(#[from] rquest::Error),

    /// JSON serialization or deserialization failed.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Request timed out.
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

    /// File uploads require authentication cookies.
    #[error("File uploads require authentication cookies")]
    FileUploadRequiresAuth,

    /// Invalid model for the specified mode.
    #[error("Invalid model '{model}' for mode '{mode}'")]
    InvalidModelForMode { model: String, mode: String },

    /// Failed to get upload URL.
    #[error("Failed to get upload URL: {0}")]
    UploadUrlFailed(String),

    /// S3 upload failed.
    #[error("S3 upload failed: {0}")]
    S3UploadFailed(String),

    /// Missing secure_url in S3 response.
    #[error("Missing secure_url in S3 response")]
    MissingSecureUrl,

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
}

/// Convenience Result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

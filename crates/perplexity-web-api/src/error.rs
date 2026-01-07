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

    /// Invalid request parameters (mode, model, sources, etc.).
    #[error("Validation error: {0}")]
    Validation(String),

    /// File upload failed.
    #[error("File upload error: {0}")]
    Upload(String),

    /// SSE parsing error.
    #[error("SSE parsing error: {0}")]
    Sse(String),

    /// Response parsing error (e.g., missing expected fields).
    #[error("Response parsing error: {0}")]
    Parse(String),

    /// Server returned an error response.
    #[error("Server error: {status} - {message}")]
    Server { status: u16, message: String },

    /// Stream ended unexpectedly.
    #[error("Stream ended unexpectedly")]
    UnexpectedEndOfStream,
}

/// Convenience Result type for this crate.
pub type Result<T> = std::result::Result<T, Error>;

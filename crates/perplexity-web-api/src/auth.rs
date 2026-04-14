use std::fmt;

/// Cookie name for the Perplexity session token.
pub const SESSION_TOKEN_COOKIE_NAME: &str = "__Secure-next-auth.session-token";
/// Cookie name for the Perplexity CSRF token.
pub const CSRF_TOKEN_COOKIE_NAME: &str = "next-auth.csrf-token";
/// Redacted placeholder used when formatting secret values for diagnostics.
pub const REDACTED_SECRET: &str = "[REDACTED]";

const LEGACY_SESSION_TOKEN_COOKIE_NAME: &str = "next-auth.session-token";

/// Authentication cookies required for authenticated Perplexity features.
#[derive(Clone, PartialEq, Eq)]
pub struct AuthCookies {
    session_token: String,
    csrf_token: String,
}

impl AuthCookies {
    /// Creates a new set of authentication cookies.
    pub fn new(session_token: impl Into<String>, csrf_token: impl Into<String>) -> Self {
        Self { session_token: session_token.into(), csrf_token: csrf_token.into() }
    }

    /// Returns the session token value.
    pub fn session_token(&self) -> &str {
        &self.session_token
    }

    /// Returns the CSRF token value.
    pub fn csrf_token(&self) -> &str {
        &self.csrf_token
    }

    pub(crate) fn as_pairs(&self) -> [(&str, &str); 3] {
        [
            (SESSION_TOKEN_COOKIE_NAME, self.session_token()),
            (LEGACY_SESSION_TOKEN_COOKIE_NAME, self.session_token()),
            (CSRF_TOKEN_COOKIE_NAME, self.csrf_token()),
        ]
    }
}

impl fmt::Debug for AuthCookies {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthCookies")
            .field("session_token", &REDACTED_SECRET)
            .field("csrf_token", &REDACTED_SECRET)
            .finish()
    }
}

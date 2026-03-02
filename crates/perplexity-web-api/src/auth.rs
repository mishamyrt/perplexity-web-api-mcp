use std::collections::HashMap;

/// Cookie name for the Perplexity session token.
pub const SESSION_TOKEN_COOKIE_NAME: &str = "next-auth.session-token";
/// Cookie name for the Perplexity CSRF token.
pub const CSRF_TOKEN_COOKIE_NAME: &str = "next-auth.csrf-token";

/// Authentication cookies required for authenticated Perplexity features.
#[derive(Debug, Clone, PartialEq, Eq)]
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

    pub(crate) fn as_pairs(&self) -> [(&str, &str); 2] {
        [
            (SESSION_TOKEN_COOKIE_NAME, self.session_token()),
            (CSRF_TOKEN_COOKIE_NAME, self.csrf_token()),
        ]
    }
}

impl From<AuthCookies> for HashMap<String, String> {
    fn from(value: AuthCookies) -> Self {
        HashMap::from([
            (SESSION_TOKEN_COOKIE_NAME.to_string(), value.session_token),
            (CSRF_TOKEN_COOKIE_NAME.to_string(), value.csrf_token),
        ])
    }
}

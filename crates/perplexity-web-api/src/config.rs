pub const API_BASE_URL: &str = "https://www.perplexity.ai";
pub const API_VERSION: &str = "2.18";

pub const ENDPOINT_AUTH_SESSION: &str = "/api/auth/session";
pub const ENDPOINT_SSE_ASK: &str = "/rest/sse/perplexity_ask";
pub const ENDPOINT_UPLOAD_URL: &str = "/rest/uploads/create_upload_url";

pub const VALID_MODES: &[&str] = &["auto", "pro", "reasoning", "deep research"];
pub const VALID_SOURCES: &[&str] = &["web", "scholar", "social"];

pub fn model_preference(mode: &str, model: Option<&str>) -> Option<&'static str> {
    match (mode, model) {
        ("auto", None) => Some("turbo"),
        ("pro", None) => Some("pplx_pro"),
        ("pro", Some("sonar")) => Some("experimental"),
        ("pro", Some("gpt-5.2")) => Some("gpt52"),
        ("pro", Some("claude-4.5-sonnet")) => Some("claude45sonnet"),
        ("pro", Some("grok-4.1")) => Some("grok41nonreasoning"),
        ("reasoning", None) => Some("pplx_reasoning"),
        ("reasoning", Some("gpt-5.2-thinking")) => Some("gpt52_thinking"),
        ("reasoning", Some("claude-4.5-sonnet-thinking")) => Some("claude45sonnetthinking"),
        ("reasoning", Some("gemini-3.0-pro")) => Some("gemini30pro"),
        ("reasoning", Some("kimi-k2-thinking")) => Some("kimik2thinking"),
        ("reasoning", Some("grok-4.1-reasoning")) => Some("grok41reasoning"),
        ("deep research", None) => Some("pplx_alpha"),
        _ => None,
    }
}

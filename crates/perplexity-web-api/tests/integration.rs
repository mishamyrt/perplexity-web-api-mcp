use perplexity_web_api::{AuthCookies, Client, SearchMode, SearchRequest, SearchResponse};
use std::time::Duration;

const SESSION_TOKEN_ENV: &str = "PERPLEXITY_SESSION_TOKEN";
const CSRF_TOKEN_ENV: &str = "PERPLEXITY_CSRF_TOKEN";

const SEARCH_TIMEOUT: Duration = Duration::from_secs(60);
const RESEARCH_TIMEOUT: Duration = Duration::from_secs(300);
const REASON_TIMEOUT: Duration = Duration::from_secs(120);

const SEARCH_QUERY: &str = "What is Rust programming language?";
const RESEARCH_QUERY: &str =
    "Provide a concise research summary of Rust programming language evolution.";
const REASON_QUERY: &str = "If all mammals are warm-blooded and whales are mammals, are whales warm-blooded? Explain briefly.";

fn require_env(name: &str) -> String {
    std::env::var(name).ok().filter(|value| !value.trim().is_empty()).unwrap_or_else(|| {
        panic!(
            "Required environment variable `{name}` is not set.\n\n\
                 Integration tests require both variables:\n\
                   - {SESSION_TOKEN_ENV}\n\
                   - {CSRF_TOKEN_ENV}\n\n\
                 Usage:\n\
                   {SESSION_TOKEN_ENV}=<token> {CSRF_TOKEN_ENV}=<token> make test-integration"
        )
    })
}

fn ensure_required_env_vars() {
    let _ = require_env(SESSION_TOKEN_ENV);
    let _ = require_env(CSRF_TOKEN_ENV);
}

fn auth_cookies() -> AuthCookies {
    let session_token = require_env(SESSION_TOKEN_ENV);
    let csrf_token = require_env(CSRF_TOKEN_ENV);

    AuthCookies::new(session_token, csrf_token)
}

fn assert_response_has_answer(response: &SearchResponse, context: &str) {
    let has_answer =
        response.answer.as_deref().map(str::trim).is_some_and(|answer| !answer.is_empty());

    assert!(has_answer, "{context} returned an empty answer. Raw response: {}", response.raw);
}

#[tokio::test]
#[ignore]
async fn perplexity_search_without_authorization_returns_answer() {
    let client = Client::builder()
        .timeout(SEARCH_TIMEOUT)
        .build()
        .await
        .expect("Failed to build unauthenticated client for perplexity_search test");

    let response = client
        .search(SearchRequest::new(SEARCH_QUERY).mode(SearchMode::Auto).incognito(true))
        .await
        .expect("perplexity_search without authorization request failed");

    assert_response_has_answer(&response, "perplexity_search without authorization");
}

#[tokio::test]
#[ignore]
async fn perplexity_search_with_authorization_returns_answer() {
    ensure_required_env_vars();

    let client = Client::builder()
        .cookies(auth_cookies())
        .timeout(SEARCH_TIMEOUT)
        .build()
        .await
        .expect("Failed to build authenticated client for perplexity_search test");

    let response = client
        .search(SearchRequest::new(SEARCH_QUERY).mode(SearchMode::Auto).incognito(true))
        .await
        .expect("perplexity_search with authorization request failed");

    assert_response_has_answer(&response, "perplexity_search with authorization");
}

#[tokio::test]
#[ignore]
async fn perplexity_research_with_authorization_returns_answer() {
    ensure_required_env_vars();

    let client = Client::builder()
        .cookies(auth_cookies())
        .timeout(RESEARCH_TIMEOUT)
        .build()
        .await
        .expect("Failed to build authenticated client for perplexity_research test");

    let response = client
        .search(
            SearchRequest::new(RESEARCH_QUERY).mode(SearchMode::DeepResearch).incognito(true),
        )
        .await
        .expect("perplexity_research with authorization request failed");

    assert_response_has_answer(&response, "perplexity_research with authorization");
}

#[tokio::test]
#[ignore]
async fn perplexity_reason_with_authorization_returns_answer() {
    ensure_required_env_vars();

    let client = Client::builder()
        .cookies(auth_cookies())
        .timeout(REASON_TIMEOUT)
        .build()
        .await
        .expect("Failed to build authenticated client for perplexity_reason test");

    let response = client
        .search(SearchRequest::new(REASON_QUERY).mode(SearchMode::Reasoning).incognito(true))
        .await
        .expect("perplexity_reason with authorization request failed");

    assert_response_has_answer(&response, "perplexity_reason with authorization");
}

#[tokio::test]
#[ignore]
async fn invalid_or_expired_cookies_fail_during_build() {
    let result = Client::builder()
        .cookies(AuthCookies::new("invalid-session-token", "invalid-csrf-token"))
        .timeout(SEARCH_TIMEOUT)
        .build()
        .await;

    match result {
        Ok(_) => panic!("invalid cookies should fail during authenticated warm-up"),
        Err(error) => {
            assert!(matches!(error, perplexity_web_api::Error::AuthenticationFailed))
        }
    }
}

use crate::auth::AuthCookies;
use crate::config::{
    API_BASE_URL, API_MODE_CONCISE, API_MODE_COPILOT, API_VERSION, ENDPOINT_AUTH_SESSION,
    ENDPOINT_SSE_ASK,
};
use crate::error::{Error, Result};
use crate::http::ensure_success_response;
use crate::sse::SseStream;
use crate::types::{
    AskParams, AskPayload, FollowUpContext, SearchEvent, SearchMode, SearchRequest,
    SearchResponse, UploadFile,
};
use crate::upload::upload_files;
use futures_util::{Stream, StreamExt};
use rquest::{Client as HttpClient, cookie::Jar};
use rquest_util::Emulation;
use serde_json::Value;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
use uuid::Uuid;

/// Default request timeout (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builder for creating a configured [`Client`] instance.
pub struct ClientBuilder {
    cookies: Option<AuthCookies>,
    http_client: Option<HttpClient>,
    timeout: Duration,
}

impl ClientBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self { cookies: None, http_client: None, timeout: DEFAULT_TIMEOUT }
    }

    /// Sets authentication cookies for the client.
    ///
    /// Required for enhanced features like file uploads and pro/reasoning modes.
    pub fn cookies(mut self, cookies: AuthCookies) -> Self {
        self.cookies = Some(cookies);
        self
    }

    /// Sets a custom HTTP client.
    ///
    /// Use this to provide a pre-configured rquest client with custom settings.
    pub fn http_client(mut self, client: HttpClient) -> Self {
        self.http_client = Some(client);
        self
    }

    /// Sets the request timeout.
    ///
    /// Default is 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Builds the client and performs initial session warm-up.
    ///
    /// This mirrors the Python client's behavior of making an initial
    /// GET request to `/api/auth/session` to establish a session.
    pub async fn build(self) -> Result<Client> {
        let Self { cookies, http_client, timeout } = self;
        let has_cookies = cookies.is_some();

        let http = match http_client {
            Some(client) => client,
            None => {
                let jar = Arc::new(Jar::default());
                let url = API_BASE_URL.parse().map_err(|_| Error::InvalidBaseUrl)?;

                if let Some(auth_cookies) = &cookies {
                    for (name, value) in auth_cookies.as_pairs() {
                        let cookie =
                            format!("{name}={value}; Domain=www.perplexity.ai; Path=/");
                        jar.add_cookie_str(&cookie, &url);
                    }
                }

                HttpClient::builder()
                    .emulation(Emulation::Chrome136)
                    .cookie_provider(jar)
                    .build()
                    .map_err(Error::HttpClientInit)?
            }
        };

        let warmup_started = Instant::now();
        let session_fut =
            http.get(format!("{}{}", API_BASE_URL, ENDPOINT_AUTH_SESSION)).send();
        let session_response = tokio::time::timeout(timeout, session_fut)
            .await
            .map_err(|_| Error::Timeout(timeout))?
            .map_err(Error::SessionWarmup)?;
        let remaining_timeout = remaining_timeout(timeout, warmup_started)?;
        validate_session_warmup(session_response, has_cookies, timeout, remaining_timeout)
            .await?;

        Ok(Client { http, has_cookies, timeout })
    }
}

impl Default for ClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}

/// Async client for interacting with the Perplexity AI Web API.
///
/// Create a client using [`Client::builder()`]:
///
/// ```no_run
/// # async fn example() -> perplexity_web_api::Result<()> {
/// let client = perplexity_web_api::Client::builder()
///     .build()
///     .await?;
///
/// let response = client.search(
///     perplexity_web_api::SearchRequest::new("What is Rust?")
/// ).await?;
///
/// if let Some(answer) = response.answer {
///     println!("{}", answer);
/// }
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    has_cookies: bool,
    timeout: Duration,
}

impl Client {
    /// Creates a new [`ClientBuilder`] for configuring the client.
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Performs a search query and returns the final response.
    ///
    /// This method consumes the entire SSE stream and returns the final result.
    /// For streaming responses, use [`search_stream`](Self::search_stream) instead.
    pub async fn search(&self, request: SearchRequest) -> Result<SearchResponse> {
        let mut stream = Box::pin(self.search_stream(request).await?);
        let mut last_event: Option<SearchEvent> = None;

        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => last_event = Some(event),
                Err(e) => return Err(e),
            }
        }

        let SearchEvent { answer, web_results, backend_uuid, attachments, raw } =
            last_event.ok_or(Error::UnexpectedEndOfStream)?;
        let follow_up = FollowUpContext { backend_uuid, attachments };

        Ok(SearchResponse { answer, web_results, follow_up, raw })
    }

    /// Performs a search query and returns a stream of events.
    ///
    /// Each event contains partial or complete response data as it arrives.
    /// The stream ends when the server sends `event: end_of_stream`.
    pub async fn search_stream(
        &self,
        request: SearchRequest,
    ) -> Result<impl Stream<Item = Result<SearchEvent>>> {
        self.validate_request(&request)?;

        let file_refs: Vec<&UploadFile> = request.files.iter().collect();
        let mut attachments = upload_files(&self.http, &file_refs, self.timeout).await?;

        if let Some(ref follow_up) = request.follow_up {
            attachments.extend(follow_up.attachments.clone());
        }

        let mode_str = match request.mode {
            SearchMode::Auto => API_MODE_CONCISE,
            SearchMode::Pro | SearchMode::Reasoning | SearchMode::DeepResearch => {
                API_MODE_COPILOT
            }
        };

        let model_pref = request
            .model_preference
            .map(|preference| preference.as_str())
            .unwrap_or_else(|| request.mode.default_preference());

        let sources_str: Vec<&'static str> =
            request.sources.iter().map(|s| s.as_str()).collect();

        let payload = AskPayload {
            query_str: &request.query,
            params: AskParams {
                attachments,
                frontend_context_uuid: Uuid::new_v4().to_string(),
                frontend_uuid: Uuid::new_v4().to_string(),
                is_incognito: request.incognito,
                language: &request.language,
                last_backend_uuid: request.follow_up.and_then(|f| f.backend_uuid),
                mode: mode_str,
                model_preference: model_pref,
                source: "default",
                sources: sources_str,
                version: API_VERSION,
            },
        };

        let request_fut = self
            .http
            .post(format!("{}{}", API_BASE_URL, ENDPOINT_SSE_ASK))
            .json(&payload)
            .send();

        let response = tokio::time::timeout(self.timeout, request_fut)
            .await
            .map_err(|_| Error::Timeout(self.timeout))?
            .map_err(Error::SearchRequest)?;
        let response = ensure_success_response(response)?;

        Ok(SseStream::new(response.bytes_stream()))
    }

    /// Uploads multiple files in a single batch and returns their S3 object URLs.
    ///
    /// All files are registered with the backend in one request, then uploaded
    /// to S3 in parallel, and finally processed server-side.
    /// Requires authentication cookies.
    pub async fn upload_files(&self, files: &[&UploadFile]) -> Result<Vec<String>> {
        if !files.is_empty() && !self.has_cookies {
            return Err(Error::FileUploadRequiresAuth);
        }
        upload_files(&self.http, files, self.timeout).await
    }

    fn validate_request(&self, request: &SearchRequest) -> Result<()> {
        if !request.files.is_empty() && !self.has_cookies {
            return Err(Error::FileUploadRequiresAuth);
        }

        if !self.has_cookies
            && crate::request_requires_authentication(request.mode, request.model_preference)
        {
            return Err(Error::AuthenticatedModeRequiresAuth);
        }

        Ok(())
    }
}

async fn validate_session_warmup(
    response: rquest::Response,
    has_cookies: bool,
    total_timeout: Duration,
    remaining_timeout: Duration,
) -> Result<()> {
    let response = ensure_success_response(response)?;
    if !has_cookies {
        return Ok(());
    }

    let body_fut = response.bytes();
    let body: bytes::Bytes = tokio::time::timeout(remaining_timeout, body_fut)
        .await
        .map_err(|_| Error::Timeout(total_timeout))?
        .map_err(Error::SessionWarmup)?;
    let payload: Value = serde_json::from_slice(&body)?;

    // Authenticated NextAuth session payloads are non-empty JSON objects with at
    // least one non-null field (for example `user` or `expires`). Empty objects
    // or `null` mean the cookies did not resolve to a logged-in session.
    match payload {
        Value::Object(fields)
            if !fields.is_empty() && fields.values().any(|value| !value.is_null()) =>
        {
            Ok(())
        }
        Value::Null | Value::Object(_) => Err(Error::AuthenticationFailed),
        _ => Err(Error::InvalidAuthenticationResponse),
    }
}

fn remaining_timeout(timeout: Duration, started_at: Instant) -> Result<Duration> {
    timeout.checked_sub(started_at.elapsed()).ok_or(Error::Timeout(timeout))
}

#[cfg(test)]
mod tests {
    use super::Client;
    use crate::{SearchMode, SearchModel, SearchRequest, UploadFile};
    use std::time::Duration;

    #[test]
    fn unauthenticated_client_rejects_file_uploads() {
        let request =
            SearchRequest::new("test").file(UploadFile::from_text("notes.txt", "contents"));

        let error = build_request_validator(false, &request).unwrap_err();

        assert!(matches!(error, crate::Error::FileUploadRequiresAuth));
    }

    #[test]
    fn unauthenticated_client_rejects_premium_modes() {
        let request = SearchRequest::new("test").mode(SearchMode::Reasoning);

        let error = build_request_validator(false, &request).unwrap_err();

        assert!(matches!(error, crate::Error::AuthenticatedModeRequiresAuth));
    }

    #[test]
    fn unauthenticated_client_allows_explicit_turbo_model() {
        let request = SearchRequest::new("test").model(SearchModel::Turbo);

        build_request_validator(false, &request).unwrap();
    }

    #[test]
    fn unauthenticated_client_rejects_non_turbo_explicit_models() {
        let request = SearchRequest::new("test").model(SearchModel::ProAuto);

        let error = build_request_validator(false, &request).unwrap_err();

        assert!(matches!(error, crate::Error::AuthenticatedModeRequiresAuth));
    }

    #[test]
    fn authenticated_client_allows_premium_modes_and_files() {
        let request = SearchRequest::new("test")
            .mode(SearchMode::Pro)
            .model(SearchModel::ProAuto)
            .file(UploadFile::from_text("notes.txt", "contents"));

        build_request_validator(true, &request).unwrap();
    }

    fn build_request_validator(
        has_cookies: bool,
        request: &SearchRequest,
    ) -> crate::Result<()> {
        let client = Client {
            http: rquest::Client::builder().build().unwrap(),
            has_cookies,
            timeout: Duration::from_secs(30),
        };
        client.validate_request(request)
    }
}

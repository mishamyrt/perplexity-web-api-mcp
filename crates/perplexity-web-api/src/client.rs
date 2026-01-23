use crate::config::{
    API_BASE_URL, API_VERSION, ENDPOINT_AUTH_SESSION, ENDPOINT_SSE_ASK, model_preference,
};
use crate::error::{Error, Result};
use crate::sse::SseStream;
use crate::types::SearchMode;
use crate::types::{AskParams, AskPayload, SearchEvent, SearchRequest, SearchResponse};
use crate::upload::upload_file;
use futures_util::{Stream, StreamExt};
use rquest::{Client as HttpClient, cookie::Jar};
use rquest_util::Emulation;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

/// Default request timeout (30 seconds).
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Builder for creating a configured [`Client`] instance.
pub struct ClientBuilder {
    cookies: HashMap<String, String>,
    http_client: Option<HttpClient>,
    timeout: Duration,
}

impl ClientBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self { cookies: HashMap::new(), http_client: None, timeout: DEFAULT_TIMEOUT }
    }

    /// Sets authentication cookies for the client.
    ///
    /// Required for enhanced features like file uploads and pro/reasoning modes.
    pub fn cookies(mut self, cookies: HashMap<String, String>) -> Self {
        self.cookies = cookies;
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
        let timeout = self.timeout;
        let http = match self.http_client {
            Some(client) => client,
            None => {
                let jar = Arc::new(Jar::default());
                let url = API_BASE_URL.parse().expect("Invalid API base URL");

                for (name, value) in &self.cookies {
                    let cookie =
                        format!("{}={}; Domain=www.perplexity.ai; Path=/", name, value);
                    jar.add_cookie_str(&cookie, &url);
                }

                HttpClient::builder()
                    .emulation(Emulation::Chrome131)
                    .cookie_provider(jar)
                    .build()
                    .map_err(Error::Http)?
            }
        };

        let session_fut =
            http.get(format!("{}{}", API_BASE_URL, ENDPOINT_AUTH_SESSION)).send();
        tokio::time::timeout(timeout, session_fut)
            .await
            .map_err(|_| Error::Timeout(timeout))?
            .map_err(Error::Http)?;

        Ok(Client { http, has_cookies: !self.cookies.is_empty(), timeout })
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

        let event = last_event.ok_or(Error::UnexpectedEndOfStream)?;

        Ok(SearchResponse {
            answer: event.answer.clone(),
            web_results: event.web_results.clone(),
            follow_up: event.as_follow_up(),
            raw: serde_json::to_value(&event).map_err(Error::Json)?,
        })
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

        let mut attachments = Vec::new();

        for file in &request.files {
            let url = upload_file(&self.http, file, self.timeout).await?;
            attachments.push(url);
        }

        if let Some(ref follow_up) = request.follow_up {
            attachments.extend(follow_up.attachments.clone());
        }

        let mode_str: &'static str = match request.mode {
            SearchMode::Auto => "concise",
            _ => "copilot",
        };

        let model_pref = model_preference(request.mode, request.model).ok_or_else(|| {
            Error::InvalidModelForMode {
                model: request.model.map(|m| m.as_str()).unwrap_or("default").to_string(),
                mode: request.mode.to_string(),
            }
        })?;

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
            .map_err(Error::Http)?
            .error_for_status()
            .map_err(|e| Error::Server {
                status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                message: e.to_string(),
            })?;

        Ok(SseStream::new(response.bytes_stream()))
    }

    fn validate_request(&self, request: &SearchRequest) -> Result<()> {
        // Mode and sources are now validated at compile time via enums.
        // Only runtime validation needed is for file uploads requiring auth.
        if !request.files.is_empty() && !self.has_cookies {
            return Err(Error::FileUploadRequiresAuth);
        }

        Ok(())
    }
}

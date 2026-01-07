use crate::config::{
    model_preference, API_BASE_URL, API_VERSION, ENDPOINT_AUTH_SESSION,
    ENDPOINT_SSE_ASK, VALID_MODES, VALID_SOURCES,
};
use crate::error::{Error, Result};
use crate::sse::SseStream;
use crate::types::{AskParams, AskPayload, SearchEvent, SearchRequest, SearchResponse};
use crate::upload::upload_file;
use futures_util::{Stream, StreamExt};
use rquest::{cookie::Jar, Client as HttpClient};
use rquest_util::Emulation;
use std::collections::HashMap;
use std::sync::Arc;
use uuid::Uuid;

/// Builder for creating a configured [`Client`] instance.
pub struct ClientBuilder {
    cookies: HashMap<String, String>,
    http_client: Option<HttpClient>,
}

impl ClientBuilder {
    /// Creates a new builder with default settings.
    pub fn new() -> Self {
        Self {
            cookies: HashMap::new(),
            http_client: None,
        }
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

    /// Builds the client and performs initial session warm-up.
    ///
    /// This mirrors the Python client's behavior of making an initial
    /// GET request to `/api/auth/session` to establish a session.
    pub async fn build(self) -> Result<Client> {
        let http = match self.http_client {
            Some(client) => client,
            None => {
                let jar = Arc::new(Jar::default());
                let url = API_BASE_URL.parse().map_err(|_| {
                    Error::Validation("Invalid API base URL".to_string())
                })?;

                for (name, value) in &self.cookies {
                    let cookie = format!("{}={}; Domain=www.perplexity.ai; Path=/", name, value);
                    jar.add_cookie_str(&cookie, &url);
                }

                HttpClient::builder()
                    .emulation(Emulation::Chrome131)
                    .cookie_provider(jar)
                    .build()
                    .map_err(Error::Http)?
            }
        };

        http.get(format!("{}{}", API_BASE_URL, ENDPOINT_AUTH_SESSION))
            .send()
            .await?;

        Ok(Client {
            http,
            has_cookies: !self.cookies.is_empty(),
        })
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
            chunks: event.chunks.clone(),
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
            let url = upload_file(&self.http, file).await?;
            attachments.push(url);
        }

        if let Some(ref follow_up) = request.follow_up {
            attachments.extend(follow_up.attachments.clone());
        }

        let mode_str = if request.mode == "auto" {
            "concise"
        } else {
            "copilot"
        };

        let model_pref = model_preference(&request.mode, request.model.as_deref())
            .ok_or_else(|| {
                Error::Validation(format!(
                    "Invalid model '{}' for mode '{}'",
                    request.model.as_deref().unwrap_or("default"),
                    request.mode
                ))
            })?;

        let payload = AskPayload {
            query_str: request.query,
            params: AskParams {
                attachments,
                frontend_context_uuid: Uuid::new_v4().to_string(),
                frontend_uuid: Uuid::new_v4().to_string(),
                is_incognito: request.incognito,
                language: request.language,
                last_backend_uuid: request.follow_up.and_then(|f| f.backend_uuid),
                mode: mode_str.to_string(),
                model_preference: model_pref.to_string(),
                source: "default".to_string(),
                sources: request.sources,
                version: API_VERSION.to_string(),
            },
        };

        let response = self
            .http
            .post(format!("{}{}", API_BASE_URL, ENDPOINT_SSE_ASK))
            .json(&payload)
            .send()
            .await?
            .error_for_status()
            .map_err(|e| {
                Error::Server {
                    status: e.status().map(|s| s.as_u16()).unwrap_or(0),
                    message: e.to_string(),
                }
            })?;

        Ok(SseStream::new(response.bytes_stream()))
    }

    fn validate_request(&self, request: &SearchRequest) -> Result<()> {
        if !VALID_MODES.contains(&request.mode.as_str()) {
            return Err(Error::Validation(format!(
                "Invalid mode '{}'. Valid modes: {:?}",
                request.mode, VALID_MODES
            )));
        }

        for source in &request.sources {
            if !VALID_SOURCES.contains(&source.as_str()) {
                return Err(Error::Validation(format!(
                    "Invalid source '{}'. Valid sources: {:?}",
                    source, VALID_SOURCES
                )));
            }
        }

        if !request.files.is_empty() && !self.has_cookies {
            return Err(Error::Validation(
                "File uploads require authentication cookies".to_string(),
            ));
        }

        Ok(())
    }
}

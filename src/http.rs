//! HTTP client utilities
//!
//! This module provides HTTP client functions with support for retries,
//! configurable timeouts, and file downloads.
//!
//! ## Examples
//!
//! ```rust,no_run
//! use xx::http;
//!
//! #[tokio::main]
//! async fn main() {
//!     // Simple GET request
//!     let resp = http::get("https://httpbin.org/get").await.unwrap();
//!     println!("Status: {}", resp.status);
//!
//!     // GET with options
//!     let resp = http::Client::new()
//!         .timeout(std::time::Duration::from_secs(30))
//!         .retries(3)
//!         .get("https://httpbin.org/get")
//!         .await
//!         .unwrap();
//!
//!     // Download a file
//!     http::download("https://example.com/file.zip", "/tmp/file.zip")
//!         .await
//!         .unwrap();
//! }
//! ```

use std::path::Path;
use std::time::Duration;

use reqwest::IntoUrl;

use crate::{XXError, XXResult, error, file};

/// Default request timeout
pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// Default number of retries
pub const DEFAULT_RETRIES: u32 = 3;

/// HTTP response
pub struct XXHTTPResponse {
    /// HTTP status code
    pub status: reqwest::StatusCode,
    /// Response headers
    pub headers: reqwest::header::HeaderMap,
    /// Response body as string
    pub body: String,
}

/// Maximum retry delay cap (10 seconds)
pub const MAX_RETRY_DELAY: Duration = Duration::from_secs(10);

/// HTTP client with configurable options
pub struct Client {
    timeout: Duration,
    retries: u32,
    retry_delay: Duration,
    user_agent: Option<String>,
}

impl Default for Client {
    fn default() -> Self {
        Self::new()
    }
}

impl Client {
    /// Create a new HTTP client with default settings
    pub fn new() -> Self {
        Self {
            timeout: DEFAULT_TIMEOUT,
            retries: DEFAULT_RETRIES,
            retry_delay: Duration::from_millis(500),
            user_agent: None,
        }
    }

    /// Set the request timeout
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the number of retries for failed requests
    pub fn retries(mut self, retries: u32) -> Self {
        self.retries = retries;
        self
    }

    /// Set the base delay between retries (uses exponential backoff)
    pub fn retry_delay(mut self, delay: Duration) -> Self {
        self.retry_delay = delay;
        self
    }

    /// Set a custom user agent
    pub fn user_agent<S: Into<String>>(mut self, agent: S) -> Self {
        self.user_agent = Some(agent.into());
        self
    }

    /// Perform a GET request
    pub async fn get(&self, url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();

        let resp = self
            .request_with_retry(&client, &url, |resp| {
                let url_str = url_str.clone();
                async move {
                    Ok(XXHTTPResponse {
                        status: resp.status(),
                        headers: resp.headers().clone(),
                        body: resp
                            .text()
                            .await
                            .map_err(|err| XXError::HTTPError(err, url_str))?,
                    })
                }
            })
            .await?;

        Ok(resp)
    }

    /// Perform a GET request and return bytes
    pub async fn get_bytes(&self, url: impl IntoUrl) -> XXResult<Vec<u8>> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();

        self.request_with_retry(&client, &url, |resp| {
            let url_str = url_str.clone();
            async move {
                resp.bytes()
                    .await
                    .map(|b| b.to_vec())
                    .map_err(|err| XXError::HTTPError(err, url_str))
            }
        })
        .await
    }

    /// Download a file
    pub async fn download(&self, url: impl IntoUrl, to: impl AsRef<Path>) -> XXResult<()> {
        let to = to.as_ref();
        let bytes = self.get_bytes(url).await?;

        file::mkdirp(to.parent().unwrap())?;
        file::write(to, &bytes)?;
        Ok(())
    }

    /// Internal helper for request with retry logic using exponential backoff
    async fn request_with_retry<T, F, Fut>(
        &self,
        client: &reqwest::Client,
        url: &reqwest::Url,
        process_response: F,
    ) -> XXResult<T>
    where
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.retries {
            if attempt > 0 {
                // Exponential backoff: base_delay * 2^(attempt-1), capped at MAX_RETRY_DELAY
                let delay = self.retry_delay * 2_u32.pow(attempt - 1);
                let delay = delay.min(MAX_RETRY_DELAY);
                trace!("Retry attempt {} for {} (delay: {:?})", attempt, url, delay);
                tokio::time::sleep(delay).await;
            }

            match client.get(url.clone()).send().await {
                Ok(resp) => {
                    if resp.status().is_server_error() && attempt < self.retries {
                        // Server error, retry
                        last_error = Some(error!("Server error: {}", resp.status()));
                        continue;
                    }

                    resp.error_for_status_ref()
                        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;

                    return process_response(resp).await;
                }
                Err(err) => {
                    if (err.is_timeout() || err.is_connect()) && attempt < self.retries {
                        // Transient error, retry
                        last_error = Some(XXError::HTTPError(err, url.to_string()));
                        continue;
                    }
                    return Err(XXError::HTTPError(err, url.to_string()));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| error!("Request failed after {} retries", self.retries)))
    }

    fn build_client(&self) -> XXResult<reqwest::Client> {
        let mut builder = reqwest::Client::builder().timeout(self.timeout);

        if let Some(agent) = &self.user_agent {
            builder = builder.user_agent(agent.clone());
        }

        builder
            .build()
            .map_err(|err| error!("Failed to build HTTP client: {}", err))
    }
}

/// Get the contents of a URL
///
/// This is a convenience function that uses default settings.
/// For more control, use `Client::new()`.
///
/// # Arguments
/// * `url` - A URL to get
///
/// # Returns
/// A response with status, headers, and body
///
/// # Errors
/// Returns an error if the URL cannot be fetched
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::get;
///     let body = get("https://postman-echo.com/get").await.unwrap().body;
///     println!("{}", body);
/// }
/// ```
pub async fn get(url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
    Client::new().get(url).await
}

/// Get the contents of a URL as bytes
///
/// # Arguments
/// * `url` - A URL to get
///
/// # Returns
/// The response body as bytes
///
/// # Errors
/// Returns an error if the URL cannot be fetched
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::get_bytes;
///     let data = get_bytes("https://example.com/file.bin").await.unwrap();
/// }
/// ```
pub async fn get_bytes(url: impl IntoUrl) -> XXResult<Vec<u8>> {
    Client::new().get_bytes(url).await
}

/// Download a file from a URL
///
/// This is a convenience function that uses default settings (including retries).
/// For more control, use `Client::new()`.
///
/// # Arguments
/// * `url` - A URL to download
/// * `to` - A path to save the file
///
/// # Errors
/// Returns an error if the file cannot be downloaded or saved
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::download;
///     download("https://postman-echo.com/get", "/tmp/test.txt").await.unwrap();
/// }
/// ```
pub async fn download(url: impl IntoUrl, to: impl AsRef<Path>) -> XXResult<()> {
    Client::new().download(url, to).await
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_log::test;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{method, path},
    };

    use super::*;

    async fn setup_mock_server() -> MockServer {
        let mock_server = MockServer::start().await;

        // Mock the /get endpoint
        Mock::given(method("GET"))
            .and(path("/get"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("Date", "Wed, 21 Oct 2015 07:28:00 GMT")
                    .set_body_string(r#"{"url": "http://localhost/get"}"#),
            )
            .mount(&mock_server)
            .await;

        mock_server
    }

    #[test(tokio::test)]
    async fn test_get() {
        let mock_server = setup_mock_server().await;
        let resp = get(format!("{}/get", mock_server.uri())).await.unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("localhost"));
        assert!(resp.headers.contains_key("Date"));
    }

    #[test(tokio::test)]
    async fn test_download() {
        let mock_server = setup_mock_server().await;
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        download(format!("{}/get", mock_server.uri()), &file)
            .await
            .unwrap();
        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("localhost"));
    }
}

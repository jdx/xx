//! HTTP client utilities
//!
//! This module provides HTTP client functions with support for retries,
//! configurable timeouts, file downloads, and various HTTP methods.
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
//!
//!     // POST with JSON body
//!     let resp = http::Client::new()
//!         .header("Content-Type", "application/json")
//!         .post_json("https://httpbin.org/post", &serde_json::json!({"key": "value"}))
//!         .await
//!         .unwrap();
//!
//!     // POST with form data
//!     let resp = http::Client::new()
//!         .post_form("https://httpbin.org/post", &[("key", "value")])
//!         .await
//!         .unwrap();
//!
//!     // With authentication
//!     let resp = http::Client::new()
//!         .bearer_token("my-token")
//!         .get("https://api.example.com/protected")
//!         .await
//!         .unwrap();
//! }
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::time::Duration;

use reqwest::IntoUrl;
use serde::Serialize;

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

/// Authentication type for HTTP requests
#[derive(Clone)]
pub enum Auth {
    /// HTTP Basic authentication
    Basic { username: String, password: String },
    /// Bearer token authentication
    Bearer(String),
}

/// HTTP client with configurable options
pub struct Client {
    timeout: Duration,
    retries: u32,
    retry_delay: Duration,
    user_agent: Option<String>,
    headers: HashMap<String, String>,
    auth: Option<Auth>,
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
            headers: HashMap::new(),
            auth: None,
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

    /// Add a custom header to the request
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .header("X-Custom-Header", "custom-value")
    ///         .header("Accept", "application/json")
    ///         .get("https://httpbin.org/get")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub fn header<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Add multiple headers at once
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .headers([
    ///             ("X-First", "value1"),
    ///             ("X-Second", "value2"),
    ///         ])
    ///         .get("https://httpbin.org/get")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub fn headers<I, K, V>(mut self, headers: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: Into<String>,
        V: Into<String>,
    {
        for (k, v) in headers {
            self.headers.insert(k.into(), v.into());
        }
        self
    }

    /// Set HTTP Basic authentication
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .basic_auth("username", "password")
    ///         .get("https://httpbin.org/basic-auth/username/password")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub fn basic_auth<U: Into<String>, P: Into<String>>(
        mut self,
        username: U,
        password: P,
    ) -> Self {
        self.auth = Some(Auth::Basic {
            username: username.into(),
            password: password.into(),
        });
        self
    }

    /// Set Bearer token authentication
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .bearer_token("my-api-token")
    ///         .get("https://api.example.com/protected")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub fn bearer_token<T: Into<String>>(mut self, token: T) -> Self {
        self.auth = Some(Auth::Bearer(token.into()));
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

        // file::write handles parent directory creation
        file::write(to, &bytes)?;
        Ok(())
    }

    /// Perform a POST request with a JSON body
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .post_json("https://httpbin.org/post", &json!({"key": "value"}))
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn post_json<T: Serialize + ?Sized>(
        &self,
        url: impl IntoUrl,
        body: &T,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_json_body(reqwest::Method::POST, url, body)
            .await
    }

    /// Perform a POST request with a raw body
    pub async fn post(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_body(reqwest::Method::POST, url, body.into())
            .await
    }

    /// Perform a POST request with no body
    pub async fn post_empty(&self, url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
        self.request_with_body(reqwest::Method::POST, url, String::new())
            .await
    }

    /// Perform a POST request with form data
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .post_form("https://httpbin.org/post", &[("key", "value"), ("foo", "bar")])
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn post_form<T: Serialize + ?Sized>(
        &self,
        url: impl IntoUrl,
        form: &T,
    ) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();

        // Serialize form data to URL-encoded string
        let form_body = serde_urlencoded::to_string(form)
            .map_err(|err| error!("Form serialization error: {}", err))?;

        self.request_form_with_retry(&client, reqwest::Method::POST, &url, form_body, |resp| {
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
        .await
    }

    /// Perform a PUT request with a JSON body
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .put_json("https://httpbin.org/put", &json!({"key": "updated"}))
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn put_json<T: Serialize + ?Sized>(
        &self,
        url: impl IntoUrl,
        body: &T,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_json_body(reqwest::Method::PUT, url, body)
            .await
    }

    /// Perform a PUT request with a raw body
    pub async fn put(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_body(reqwest::Method::PUT, url, body.into())
            .await
    }

    /// Perform a PUT request with no body
    pub async fn put_empty(&self, url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
        self.request_with_body(reqwest::Method::PUT, url, String::new())
            .await
    }

    /// Perform a PATCH request with a JSON body
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    /// use serde_json::json;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .patch_json("https://httpbin.org/patch", &json!({"key": "patched"}))
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn patch_json<T: Serialize + ?Sized>(
        &self,
        url: impl IntoUrl,
        body: &T,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_json_body(reqwest::Method::PATCH, url, body)
            .await
    }

    /// Perform a PATCH request with a raw body
    pub async fn patch(
        &self,
        url: impl IntoUrl,
        body: impl Into<String>,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_body(reqwest::Method::PATCH, url, body.into())
            .await
    }

    /// Perform a DELETE request
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .delete("https://httpbin.org/delete")
    ///         .await
    ///         .unwrap();
    /// }
    /// ```
    pub async fn delete(&self, url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();

        self.delete_with_retry(&client, &url, |resp| {
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
        .await
    }

    /// Perform a DELETE request with a JSON body
    pub async fn delete_json<T: Serialize + ?Sized>(
        &self,
        url: impl IntoUrl,
        body: &T,
    ) -> XXResult<XXHTTPResponse> {
        self.request_with_json_body(reqwest::Method::DELETE, url, body)
            .await
    }

    /// Perform a HEAD request
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use xx::http::Client;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let resp = Client::new()
    ///         .head("https://httpbin.org/get")
    ///         .await
    ///         .unwrap();
    ///     println!("Content-Length: {:?}", resp.headers.get("content-length"));
    /// }
    /// ```
    pub async fn head(&self, url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;

        self.head_with_retry(&client, &url, |resp| async move {
            Ok(XXHTTPResponse {
                status: resp.status(),
                headers: resp.headers().clone(),
                body: String::new(), // HEAD requests have no body
            })
        })
        .await
    }

    /// Internal helper for requests with JSON body
    async fn request_with_json_body<T: Serialize + ?Sized>(
        &self,
        method: reqwest::Method,
        url: impl IntoUrl,
        body: &T,
    ) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();
        let json_body = serde_json::to_string(body)
            .map_err(|err| error!("JSON serialization error: {}", err))?;

        self.body_request_with_retry(&client, method, &url, json_body, true, |resp| {
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
        .await
    }

    /// Internal helper for requests with raw body
    async fn request_with_body(
        &self,
        method: reqwest::Method,
        url: impl IntoUrl,
        body: String,
    ) -> XXResult<XXHTTPResponse> {
        let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
        let client = self.build_client()?;
        let url_str = url.to_string();

        self.body_request_with_retry(&client, method, &url, body, false, |resp| {
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
        .await
    }

    /// Internal helper for executing requests with retry logic using exponential backoff
    ///
    /// This is the unified retry implementation used by all HTTP methods.
    async fn execute_with_retry<T, B, F, Fut>(
        &self,
        client: &reqwest::Client,
        method: reqwest::Method,
        url: &reqwest::Url,
        body: Option<B>,
        content_type: Option<&str>,
        process_response: F,
    ) -> XXResult<T>
    where
        B: Clone + Into<reqwest::Body>,
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        let mut last_error = None;

        for attempt in 0..=self.retries {
            if attempt > 0 {
                // Exponential backoff: base_delay * 2^(attempt-1), capped at MAX_RETRY_DELAY
                let delay = self.retry_delay * 2_u32.saturating_pow(attempt - 1);
                let delay = delay.min(MAX_RETRY_DELAY);
                trace!("Retry attempt {} for {} (delay: {:?})", attempt, url, delay);
                tokio::time::sleep(delay).await;
            }

            let mut request = client.request(method.clone(), url.clone());

            // Add custom headers
            for (key, value) in &self.headers {
                request = request.header(key.as_str(), value.as_str());
            }

            // Add authentication
            if let Some(auth) = &self.auth {
                request = match auth {
                    Auth::Basic { username, password } => {
                        request.basic_auth(username, Some(password))
                    }
                    Auth::Bearer(token) => request.bearer_auth(token),
                };
            }

            // Add content-type if specified
            if let Some(ct) = content_type {
                request = request.header("Content-Type", ct);
            }

            // Add body if present
            if let Some(ref b) = body {
                request = request.body(b.clone());
            }

            match request.send().await {
                Ok(resp) => {
                    if resp.status().is_server_error() && attempt < self.retries {
                        last_error = Some(error!("Server error: {}", resp.status()));
                        continue;
                    }

                    resp.error_for_status_ref()
                        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;

                    return process_response(resp).await;
                }
                Err(err) => {
                    if (err.is_timeout() || err.is_connect()) && attempt < self.retries {
                        last_error = Some(XXError::HTTPError(err, url.to_string()));
                        continue;
                    }
                    return Err(XXError::HTTPError(err, url.to_string()));
                }
            }
        }

        Err(last_error.unwrap_or_else(|| error!("Request failed after {} retries", self.retries)))
    }

    /// Convenience wrapper for GET requests with retry
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
        self.execute_with_retry::<T, String, F, Fut>(
            client,
            reqwest::Method::GET,
            url,
            None,
            None,
            process_response,
        )
        .await
    }

    /// Convenience wrapper for requests with JSON body
    async fn body_request_with_retry<T, F, Fut>(
        &self,
        client: &reqwest::Client,
        method: reqwest::Method,
        url: &reqwest::Url,
        body: String,
        is_json: bool,
        process_response: F,
    ) -> XXResult<T>
    where
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        let content_type = if is_json {
            Some("application/json")
        } else {
            None
        };
        self.execute_with_retry(
            client,
            method,
            url,
            Some(body),
            content_type,
            process_response,
        )
        .await
    }

    /// Convenience wrapper for form requests
    async fn request_form_with_retry<T, F, Fut>(
        &self,
        client: &reqwest::Client,
        method: reqwest::Method,
        url: &reqwest::Url,
        form_body: String,
        process_response: F,
    ) -> XXResult<T>
    where
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        self.execute_with_retry(
            client,
            method,
            url,
            Some(form_body),
            Some("application/x-www-form-urlencoded"),
            process_response,
        )
        .await
    }

    /// Convenience wrapper for DELETE requests
    async fn delete_with_retry<T, F, Fut>(
        &self,
        client: &reqwest::Client,
        url: &reqwest::Url,
        process_response: F,
    ) -> XXResult<T>
    where
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        self.execute_with_retry::<T, String, F, Fut>(
            client,
            reqwest::Method::DELETE,
            url,
            None,
            None,
            process_response,
        )
        .await
    }

    /// Convenience wrapper for HEAD requests
    async fn head_with_retry<T, F, Fut>(
        &self,
        client: &reqwest::Client,
        url: &reqwest::Url,
        process_response: F,
    ) -> XXResult<T>
    where
        F: Fn(reqwest::Response) -> Fut,
        Fut: std::future::Future<Output = XXResult<T>>,
    {
        self.execute_with_retry::<T, String, F, Fut>(
            client,
            reqwest::Method::HEAD,
            url,
            None,
            None,
            process_response,
        )
        .await
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

/// Perform a POST request with a JSON body
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::post_json;
///     use serde_json::json;
///     let resp = post_json("https://httpbin.org/post", &json!({"key": "value"})).await.unwrap();
/// }
/// ```
pub async fn post_json<T: Serialize + ?Sized>(
    url: impl IntoUrl,
    body: &T,
) -> XXResult<XXHTTPResponse> {
    Client::new().post_json(url, body).await
}

/// Perform a POST request with form data
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::post_form;
///     let resp = post_form("https://httpbin.org/post", &[("key", "value")]).await.unwrap();
/// }
/// ```
pub async fn post_form<T: Serialize + ?Sized>(
    url: impl IntoUrl,
    form: &T,
) -> XXResult<XXHTTPResponse> {
    Client::new().post_form(url, form).await
}

/// Perform a PUT request with a JSON body
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::put_json;
///     use serde_json::json;
///     let resp = put_json("https://httpbin.org/put", &json!({"key": "value"})).await.unwrap();
/// }
/// ```
pub async fn put_json<T: Serialize + ?Sized>(
    url: impl IntoUrl,
    body: &T,
) -> XXResult<XXHTTPResponse> {
    Client::new().put_json(url, body).await
}

/// Perform a PATCH request with a JSON body
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::patch_json;
///     use serde_json::json;
///     let resp = patch_json("https://httpbin.org/patch", &json!({"key": "value"})).await.unwrap();
/// }
/// ```
pub async fn patch_json<T: Serialize + ?Sized>(
    url: impl IntoUrl,
    body: &T,
) -> XXResult<XXHTTPResponse> {
    Client::new().patch_json(url, body).await
}

/// Perform a DELETE request
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::delete;
///     let resp = delete("https://httpbin.org/delete").await.unwrap();
/// }
/// ```
pub async fn delete(url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
    Client::new().delete(url).await
}

/// Perform a HEAD request
///
/// # Example
/// ```no_run
/// #[tokio::main]
/// async fn main() {
///     use xx::http::head;
///     let resp = head("https://httpbin.org/get").await.unwrap();
/// }
/// ```
pub async fn head(url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
    Client::new().head(url).await
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_log::test;
    use wiremock::{
        Mock, MockServer, ResponseTemplate,
        matchers::{header, method, path},
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

        // Mock POST endpoint
        Mock::given(method("POST"))
            .and(path("/post"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"method": "POST", "success": true}"#),
            )
            .mount(&mock_server)
            .await;

        // Mock PUT endpoint
        Mock::given(method("PUT"))
            .and(path("/put"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"method": "PUT", "success": true}"#),
            )
            .mount(&mock_server)
            .await;

        // Mock PATCH endpoint
        Mock::given(method("PATCH"))
            .and(path("/patch"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"method": "PATCH", "success": true}"#),
            )
            .mount(&mock_server)
            .await;

        // Mock DELETE endpoint
        Mock::given(method("DELETE"))
            .and(path("/delete"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_string(r#"{"method": "DELETE", "success": true}"#),
            )
            .mount(&mock_server)
            .await;

        // Mock HEAD endpoint
        Mock::given(method("HEAD"))
            .and(path("/head"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("X-Custom", "head-response")
                    .insert_header("Content-Length", "100"),
            )
            .mount(&mock_server)
            .await;

        // Mock endpoint with custom header check
        Mock::given(method("GET"))
            .and(path("/custom-header"))
            .and(header("X-Custom-Header", "custom-value"))
            .respond_with(
                ResponseTemplate::new(200).set_body_string(r#"{"custom_header": "received"}"#),
            )
            .mount(&mock_server)
            .await;

        // Mock endpoint with bearer auth check
        Mock::given(method("GET"))
            .and(path("/bearer-auth"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"authenticated": true}"#))
            .mount(&mock_server)
            .await;

        // Mock endpoint with basic auth check
        Mock::given(method("GET"))
            .and(path("/basic-auth"))
            .and(header("Authorization", "Basic dXNlcjpwYXNz")) // base64("user:pass")
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"authenticated": true}"#))
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

    #[test(tokio::test)]
    async fn test_post_json() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .post_json(
                format!("{}/post", mock_server.uri()),
                &serde_json::json!({"key": "value"}),
            )
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("POST"));
    }

    #[test(tokio::test)]
    async fn test_post_form() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .post_form(
                format!("{}/post", mock_server.uri()),
                &[("key", "value"), ("foo", "bar")],
            )
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("POST"));
    }

    #[test(tokio::test)]
    async fn test_put_json() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .put_json(
                format!("{}/put", mock_server.uri()),
                &serde_json::json!({"updated": true}),
            )
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("PUT"));
    }

    #[test(tokio::test)]
    async fn test_patch_json() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .patch_json(
                format!("{}/patch", mock_server.uri()),
                &serde_json::json!({"patched": true}),
            )
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("PATCH"));
    }

    #[test(tokio::test)]
    async fn test_delete() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .delete(format!("{}/delete", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("DELETE"));
    }

    #[test(tokio::test)]
    async fn test_head() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .head(format!("{}/head", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.headers.contains_key("x-custom"));
        assert!(resp.body.is_empty()); // HEAD has no body
    }

    #[test(tokio::test)]
    async fn test_custom_headers() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .header("X-Custom-Header", "custom-value")
            .get(format!("{}/custom-header", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("custom_header"));
    }

    #[test(tokio::test)]
    async fn test_bearer_auth() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .bearer_token("test-token")
            .get(format!("{}/bearer-auth", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("authenticated"));
    }

    #[test(tokio::test)]
    async fn test_basic_auth() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .basic_auth("user", "pass")
            .get(format!("{}/basic-auth", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("authenticated"));
    }

    #[test(tokio::test)]
    async fn test_multiple_headers() {
        let mock_server = setup_mock_server().await;

        // Add a new mock that checks multiple headers
        Mock::given(method("GET"))
            .and(path("/multi-header"))
            .and(header("X-First", "first-value"))
            .and(header("X-Second", "second-value"))
            .respond_with(ResponseTemplate::new(200).set_body_string(r#"{"multi": true}"#))
            .mount(&mock_server)
            .await;

        let resp = Client::new()
            .headers([("X-First", "first-value"), ("X-Second", "second-value")])
            .get(format!("{}/multi-header", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
    }

    #[test(tokio::test)]
    async fn test_post_empty() {
        let mock_server = setup_mock_server().await;
        let resp = Client::new()
            .post_empty(format!("{}/post", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
    }

    #[test(tokio::test)]
    async fn test_convenience_functions() {
        let mock_server = setup_mock_server().await;

        // Test post_json convenience
        let resp = post_json(
            format!("{}/post", mock_server.uri()),
            &serde_json::json!({"test": true}),
        )
        .await
        .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);

        // Test post_form convenience
        let resp = post_form(format!("{}/post", mock_server.uri()), &[("k", "v")])
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);

        // Test delete convenience
        let resp = delete(format!("{}/delete", mock_server.uri()))
            .await
            .unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);

        // Test head convenience
        let resp = head(format!("{}/head", mock_server.uri())).await.unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
    }
}

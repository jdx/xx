use std::io::Cursor;
use std::path::Path;

use reqwest::IntoUrl;

use crate::{XXError, XXResult, error, file};

pub struct XXHTTPResponse {
    pub status: reqwest::StatusCode,
    pub headers: reqwest::header::HeaderMap,
    pub body: String,
}

/// Get the contents of a URL
/// # Arguments
/// * `url` - A URL to get
/// # Returns
/// A string with the contents of the URL
/// # Errors
/// Returns an error if the URL cannot be fetched
/// # Example
/// ```
/// #[tokio::main]
/// async fn main() {
///     use xx::http::get;
///     let body = get("https://httpbin.org/get").await.unwrap().body;
///     assert!(body.contains("httpbin"));
/// }
/// ```
pub async fn get(url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
    let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
    let resp = reqwest::get(url.clone())
        .await
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    resp.error_for_status_ref()
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    Ok(XXHTTPResponse {
        status: resp.status(),
        headers: resp.headers().clone(),
        body: resp
            .text()
            .await
            .map_err(|err| XXError::HTTPError(err, url.to_string()))?,
    })
}

/// Download a file from a URL
/// # Arguments
/// * `url` - A URL to download
/// * `to` - A path to save the file
/// # Errors
/// Returns an error if the file cannot be downloaded or saved
/// # Example
/// ```
/// #[tokio::main]
/// async fn main() {
///     use xx::http::download;
///     download("https://httpbin.org/get", "/tmp/test.txt").await.unwrap();
/// }
/// ```
pub async fn download(url: impl IntoUrl, to: impl AsRef<Path>) -> XXResult<XXHTTPResponse> {
    let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
    let to = to.as_ref();
    let resp = reqwest::get(url.clone())
        .await
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    resp.error_for_status_ref()
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    file::mkdirp(to.parent().unwrap())?;
    let mut file =
        std::fs::File::create(to).map_err(|err| XXError::FileError(err, to.to_path_buf()))?;
    let out = XXHTTPResponse {
        status: resp.status(),
        headers: resp.headers().clone(),
        body: "".to_string(),
    };
    let mut content = Cursor::new(
        resp.bytes()
            .await
            .map_err(|err| XXError::HTTPError(err, url.to_string()))?,
    );
    std::io::copy(&mut content, &mut file)
        .map_err(|err| XXError::FileError(err, to.to_path_buf()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_log::test;

    use super::*;

    #[test(tokio::test)]
    async fn test_get() {
        let resp = get("https://httpbin.org/get").await.unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("httpbin"));
        assert!(resp.headers.contains_key("Date"));
    }

    #[test(tokio::test)]
    async fn test_download() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        let resp = download("https://httpbin.org/get", &file).await.unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert_eq!(resp.body, "");
        assert!(resp.headers.contains_key("Date"));
        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("httpbin"));
    }
}

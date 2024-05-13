use crate::{error, XXError, XXResult};
use reqwest::IntoUrl;
use std::path::Path;

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
/// use xx::http::get;
/// let body = get("https://httpbin.org/get").unwrap().body;
/// ```
pub fn get(url: impl IntoUrl) -> XXResult<XXHTTPResponse> {
    let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
    let resp = reqwest::blocking::get(url.clone())
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    resp.error_for_status_ref()
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    Ok(XXHTTPResponse {
        status: resp.status(),
        headers: resp.headers().clone(),
        body: resp
            .text()
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
/// use xx::http::download;
/// download("https://httpbin.org/get", "/tmp/test.txt").unwrap();
/// ```
pub fn download(url: impl IntoUrl, to: impl AsRef<Path>) -> XXResult<XXHTTPResponse> {
    let url = url.into_url().map_err(|err| error!("url error: {}", err))?;
    let to = to.as_ref();
    let mut resp = reqwest::blocking::get(url.clone())
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    resp.error_for_status_ref()
        .map_err(|err| XXError::HTTPError(err, url.to_string()))?;
    let mut file =
        std::fs::File::create(to).map_err(|err| XXError::FileError(err, to.to_path_buf()))?;
    let out = XXHTTPResponse {
        status: resp.status(),
        headers: resp.headers().clone(),
        body: "".to_string(),
    };
    std::io::copy(&mut resp, &mut file).map_err(|err| XXError::FileError(err, to.to_path_buf()))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;
    use test_log::test;

    use super::*;

    static TEST_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_get() {
        let _t = TEST_MUTEX.lock().unwrap();
        let resp = get("https://httpbin.org/get").unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert!(resp.body.contains("httpbin"));
        assert!(resp.headers.contains_key("Date"));
    }

    #[test]
    fn test_download() {
        let _t = TEST_MUTEX.lock().unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("test.txt");
        let resp = download("https://httpbin.org/get", &file).unwrap();
        assert_eq!(resp.status, reqwest::StatusCode::OK);
        assert_eq!(resp.body, "");
        assert!(resp.headers.contains_key("Date"));
        let contents = std::fs::read_to_string(&file).unwrap();
        assert!(contents.contains("httpbin"));
    }
}

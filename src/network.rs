use log::info;
use std::result::Result;
use thiserror::Error;
use url::Url;

#[derive(Debug)]
pub struct Webpage {
    pub content: String,
    pub content_type: ContentType,
}

#[derive(Debug)]
pub struct ImageResponse {
    pub data: Vec<u8>,
    pub content_type: String,
}

#[derive(Debug)]
pub enum ContentType {
    HTML,
    Other(String),
}

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Failed to send request: {0}")]
    FailedToSendRequest(String),
    #[error("Received status code {0}")]
    NonSuccessStatusCode(u16),
    #[error("Missing Content-Type header")]
    MissingContentType,
    #[error("Incorrect or unsupported image Content-Type: {0}")]
    IncorrectContentType(String),
    #[error("Unknown error: {0}")]
    UnknownError(String),
}

pub async fn fetch_webpage(url: &str) -> Result<Webpage, FetchError> {
    let client = reqwest::Client::new();
    let request = client.get(url).header("Accept", "text/html");
    let send_result = request.send().await;

    if let Err(err) = send_result {
        return Err(FetchError::FailedToSendRequest(err.to_string()));
    }
    let response = send_result.unwrap();

    let status_code = response.status();

    if !status_code.is_success() {
        return Err(FetchError::NonSuccessStatusCode(status_code.as_u16()));
    }

    let content_type = match response.headers().get("Content-Type") {
        None => ContentType::Other("unknown".to_string()),
        Some(header_value) => {
            let header_value_str = header_value.to_str().unwrap();
            if header_value_str.starts_with("text/html") {
                ContentType::HTML
            } else {
                ContentType::Other(header_value_str.to_string())
            }
        }
    };

    let body = response.text().await.unwrap();

    let webpage = Webpage {
        content: body.to_string(),
        content_type,
    };

    return Ok(webpage);
}

pub async fn fetch_image(image_url: &str) -> Result<ImageResponse, FetchError> {
    info!("Fetching image: {}", image_url);

    let client = reqwest::Client::new();
    let request = client.get(image_url);
    let send_result = request.send().await;

    if let Err(err) = send_result {
        return Err(FetchError::FailedToSendRequest(err.to_string()));
    }
    let response = send_result.unwrap();

    let status_code = response.status();

    if !status_code.is_success() {
        return Err(FetchError::NonSuccessStatusCode(status_code.as_u16()));
    }

    let content_type = response.headers().get("Content-Type");
    if content_type.is_none() {
        return Err(FetchError::MissingContentType);
    }
    let content_type = content_type.unwrap().to_str().unwrap().to_string();

    if !is_supported_image_content_type(&content_type) {
        return Err(FetchError::IncorrectContentType(content_type));
    }

    let body = response.bytes().await;
    if let Err(err) = body {
        return Err(FetchError::UnknownError(err.to_string()));
    }
    let body = body.unwrap();
    let data = body.to_vec();

    return Ok(ImageResponse { data, content_type });
}

pub fn is_supported_image_content_type(content_type: &str) -> bool {
    if !content_type.starts_with("image/") {
        return false;
    }

    match content_type {
        "image/jpeg" => true,
        "image/png" => true,
        "image/gif" => true,
        "image/webp" => true,
        _ => false,
    }
}

pub fn resolve_url(webpage_url: &str, href: &str) -> String {
    let url = Url::parse(webpage_url).unwrap();
    let base_url = url.join(href).unwrap();
    return base_url.to_string();
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn resolve_absolute_path() {
        let webpage_url = "https://example.com";
        let href = "/path/to/image.jpg";
        let resolved = resolve_url(webpage_url, href);
        assert_eq!(resolved, "https://example.com/path/to/image.jpg");
    }

    #[test]
    fn resolve_relative_path() {
        let webpage_url = "https://example.com";
        let href = "image.jpg";
        let resolved = resolve_url(webpage_url, href);
        assert_eq!(resolved, "https://example.com/image.jpg");
    }

    #[test]
    fn resolve_full_url() {
        let webpage_url = "https://example.com";
        let href = "https://http.cat/images/200.jpg";
        let resolved = resolve_url(webpage_url, href);
        assert_eq!(resolved, "https://http.cat/images/200.jpg");
    }
}

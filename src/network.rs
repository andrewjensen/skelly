use std::result::Result;
use thiserror::Error;

#[derive(Debug)]
pub struct Webpage {
    pub content: String,
    pub content_type: ContentType,
}

#[derive(Debug)]
pub enum ContentType {
    HTML,
    Other(String),
}

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("Received status code {0}")]
    NonSuccessStatusCode(u16),
}

pub fn fetch_webpage(url: &str) -> Result<Webpage, FetchError> {
    let client = reqwest::blocking::Client::new();
    let request = client.get(url).header("Accept", "text/html");
    let response = request.send().unwrap();

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

    let body = response.text().unwrap();

    let webpage = Webpage {
        content: body.to_string(),
        content_type,
    };

    return Ok(webpage);
}

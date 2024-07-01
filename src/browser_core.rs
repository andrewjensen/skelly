use log::{error, info};
use std::process;

use crate::network::{fetch_webpage, ContentType};
use crate::parsing::parse_webpage;
use crate::rendering::Renderer;

pub struct BrowserCore {
    current_url: Option<String>,
    page_canvases: Vec<image::RgbaImage>,
}

impl BrowserCore {
    pub fn new() -> Self {
        Self {
            current_url: None,
            page_canvases: Vec::new(),
        }
    }

    pub fn navigate_to(&mut self, url: &str) {
        info!("Navigating to {}", url);

        self.current_url = Some(url.to_string());

        info!("Fetching webpage...");
        let fetch_result = fetch_webpage(url);
        if let Err(err) = fetch_result {
            error!("Failed to fetch webpage: {}", err);
            process::exit(1);
        }
        let page = fetch_result.unwrap();
        if let ContentType::Other(content_type) = page.content_type {
            error!("Expected HTML content type, got: {:?}", content_type);
            process::exit(1);
        }

        info!("Parsing...");
        let parse_result = parse_webpage(&page.content);
        if let Err(err) = parse_result {
            error!("Failed to parse webpage: {}", err);
            process::exit(1);
        }
        let document = parse_result.unwrap();
        // info!("Parsed document: {:#?}", document);

        info!("Rendering pages...");
        let mut renderer = Renderer::new();
        self.page_canvases = renderer.render_document(&document);
    }

    pub fn get_pages(&self) -> &Vec<image::RgbaImage> {
        &self.page_canvases
    }
}

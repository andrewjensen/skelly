use log::{error, info};
use std::process;

use crate::app_backend::AppBackend;
use crate::network::{fetch_webpage, ContentType};
use crate::parsing::parse_webpage;
use crate::rendering::Renderer;

pub struct App<B: AppBackend> {
    temp_url: String,
    backend: B,
    page_canvases: Vec<image::RgbaImage>,
}

impl<B: AppBackend> App<B> {
    pub fn new(backend: B) -> App<B> {
        App {
            temp_url: "".to_string(),
            backend,
            page_canvases: Vec::new(),
        }
    }

    pub fn temp_set_initial_url(&mut self, url: &str) {
        self.temp_url = url.to_string();
    }

    pub fn run(&mut self) {
        info!("Running app!");

        info!("Fetching webpage...");
        let fetch_result = fetch_webpage(&self.temp_url);
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

        info!("Sending to backend...");
        let first_page_canvas = self.page_canvases.first().unwrap();
        self.backend.render(0, first_page_canvas);
    }
}

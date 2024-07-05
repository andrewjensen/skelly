use log::{error, info};

use crate::network::{fetch_webpage, ContentType};
use crate::parsing::parse_webpage;
use crate::rendering::Renderer;
use crate::settings::Settings;

pub enum BrowserState {
    Initial,
    LoadingPage {
        url: String,
    },
    ViewingPage {
        url: String,
        page_canvases: Vec<image::RgbaImage>,
    },
    PageError {
        url: String,
        error: String,
    },
}

pub struct BrowserCore {
    pub settings: Settings,
    pub state: BrowserState,
}

impl BrowserCore {
    pub fn new(settings: Settings) -> Self {
        Self {
            settings,
            state: BrowserState::Initial,
        }
    }

    pub async fn navigate_to(&mut self, url: &str) {
        info!("Navigating to {}", url);

        self.state = BrowserState::LoadingPage {
            url: url.to_string(),
        };

        info!("Fetching webpage...");
        let fetch_result = fetch_webpage(url).await;
        if let Err(err) = fetch_result {
            error!("Failed to fetch webpage: {}", err);
            self.state = BrowserState::PageError {
                url: url.to_string(),
                error: err.to_string(),
            };
            return;
        }
        let page = fetch_result.unwrap();
        if let ContentType::Other(content_type) = page.content_type {
            error!("Expected HTML content type, got: {:?}", content_type);
            self.state = BrowserState::PageError {
                url: url.to_string(),
                error: format!("Expected HTML content type, got: {:?}", content_type),
            };
            return;
        }

        info!("Parsing...");
        let parse_result = parse_webpage(&page.content);
        if let Err(err) = parse_result {
            error!("Failed to parse webpage: {}", err);
            self.state = BrowserState::PageError {
                url: url.to_string(),
                error: err.to_string(),
            };
            return;
        }
        let document = parse_result.unwrap();
        // info!("Parsed document: {:#?}", document);

        info!("Rendering pages...");
        let mut renderer = Renderer::new(&self.settings.rendering);
        let page_canvases = renderer.render_document(&document);

        self.state = BrowserState::ViewingPage {
            url: url.to_string(),
            page_canvases,
        };
    }

    pub fn get_pages(&self) -> &Vec<image::RgbaImage> {
        if let BrowserState::ViewingPage { page_canvases, .. } = &self.state {
            return page_canvases;
        } else {
            // TODO: gracefully handle this
            panic!("Browser is not in viewing state");
        }
    }
}

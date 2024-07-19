use std::collections::HashMap;

use image::RgbaImage;
use log::{error, info, warn};

use crate::network::{fetch_image, fetch_webpage, resolve_url, ContentType, ImageResponse};
use crate::parsing::{parse_webpage, Block, Document};
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

        info!("Fetching images...");
        let _images = fetch_images(url, &document);

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

async fn fetch_images(
    webpage_url: &str,
    document: &Document,
) -> HashMap<String, Option<RgbaImage>> {
    let mut images = HashMap::new();

    let image_urls = get_image_urls(webpage_url, document);
    for image_url in image_urls {
        let image_response = fetch_image(&image_url).await;
        if let Err(err) = image_response {
            warn!("Failed to fetch image: {}", err);
            images.insert(image_url, None);
            continue;
        }
        let image_response = image_response.unwrap();

        let image = load_image(image_response);
        images.insert(image_url, image);
    }

    images
}

fn load_image(image_response: ImageResponse) -> Option<RgbaImage> {
    let image = image::load_from_memory(&image_response.data);
    if let Err(err) = image {
        warn!("Failed to load image: {}", err);
        return None;
    }
    let image = image.unwrap().to_rgba8();

    Some(image)
}

fn get_image_urls(webpage_url: &str, document: &Document) -> Vec<String> {
    let mut image_urls = vec![];

    for block in document.blocks.iter() {
        if let Block::Image { url, .. } = block {
            let resolved_url = resolve_url(webpage_url, url);
            image_urls.push(resolved_url.to_string());
        }
    }

    image_urls
}

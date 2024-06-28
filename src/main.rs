use log::{error, info};
use std::env;
use std::process;

mod debugging;
mod layout;
mod network;
mod parsing;
mod rendering;

use crate::network::{fetch_webpage, ContentType};
use crate::parsing::parse_webpage;
use crate::rendering::Renderer;

pub const CANVAS_WIDTH: u32 = 1404;
pub const CANVAS_HEIGHT: u32 = 1872;
pub const CANVAS_MARGIN_X: u32 = 100;
pub const CANVAS_MARGIN_TOP: u32 = 200;
pub const CANVAS_MARGIN_BOTTOM: u32 = 400;

fn main() {
    env_logger::init();

    // Get the first command line argument and log it out
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Please provide a URL as the first argument");
        process::exit(1);
    }

    let url = args.get(1).unwrap();
    info!("The URL argument is: {}", url);

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
    let pages = renderer.render_document(&document);

    info!("Saving pages to disk...");
    for (page_idx, page_canvas) in pages.iter().enumerate() {
        info!("  Page {}...", page_idx);

        let file_path = format!("./output/page-{}.png", page_idx);
        page_canvas.save(&file_path).expect("Failed to save image");
    }

    info!("Done");
}

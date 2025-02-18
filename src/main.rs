use image::{ImageFormat, RgbaImage};
use log::{error, info};
use std::env;
use std::io::Cursor;
use std::process;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;

mod application;
mod backend;
mod browser_core;
mod debugging;
mod keyboard;
mod network;
mod parsing;
mod rendering;
mod settings;

#[cfg(feature = "remarkable")]
mod remarkable;

#[cfg(feature = "desktop")]
mod desktop_backend;

use crate::application::Application;
use crate::backend::Backend;
use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::load_settings_with_fallback;

pub const CANVAS_WIDTH: u32 = 1404;
pub const CANVAS_HEIGHT: u32 = 1872;
pub const CANVAS_MARGIN_TOP: u32 = 150;
pub const CANVAS_MARGIN_BOTTOM: u32 = 150;
pub const DEBUG_LAYOUT: bool = false;

#[cfg(feature = "static")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    env_logger::init();

    // Get the first command line argument and log it out
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        error!("Please provide a URL as the first argument");
        process::exit(1);
    }

    let url = args.get(1).unwrap().to_string();
    info!("The URL argument is: {}", url);

    let settings_file_path = "./settings.json";
    let settings = load_settings_with_fallback(settings_file_path).await;
    info!("Settings: {:#?}", settings);

    let mut browser = BrowserCore::new(settings.clone());
    browser.navigate_to(&url).await;

    if let BrowserState::PageError { url: _, error } = browser.state {
        error!(
            "Error loading page! Cannot render the result. Reason: {}",
            error
        );
        process::exit(1);
    }

    info!("Saving pages to PNG files...");
    let handles: Vec<_> = browser
        .get_pages()
        .iter()
        .enumerate()
        .map(|(page_idx, page_canvas)| {
            let file_path = format!("./output/page-{}.png", page_idx);
            let page_canvas = page_canvas.clone();

            tokio::spawn(async move { save_page_canvas(page_canvas, &file_path).await })
        })
        .collect();

    for handle in handles {
        handle.await??;
    }

    info!("Done");

    Ok(())
}

async fn save_page_canvas(
    page_canvas: RgbaImage,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut png_buffer = Cursor::new(Vec::new());
    page_canvas.write_to(&mut png_buffer, ImageFormat::Png)?;

    let mut file = File::create(file_path).await?;
    file.write_all(&png_buffer.into_inner()).await?;

    Ok(())
}

#[cfg(feature = "desktop")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    info!("Running in desktop mode");

    let settings_file_path = "/home/root/.config/skelly/settings.json";
    let settings = load_settings_with_fallback(settings_file_path).await;
    info!("Settings: {:#?}", settings);

    let mut app = Application::new(settings.clone());

    let mut desktop_backend = desktop_backend::create_desktop_backend(settings.clone());

    app.connect_to_backend(&mut desktop_backend);

    app.run().await?;

    Ok(())
}

#[cfg(feature = "remarkable")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let settings_file_path = "/home/root/.config/skelly/settings.json";
    let settings = load_settings_with_fallback(settings_file_path).await;
    info!("Settings: {:#?}", settings);

    let mut app = remarkable::RemarkableApp::new(settings);
    app.run().await?;

    Ok(())
}

use log::{error, info};
use std::env;
use std::process;
use tokio::task::spawn_blocking;

mod browser_core;
mod debugging;
mod layout;
mod network;
mod parsing;
mod rendering;
mod settings;

#[cfg(feature = "remarkable")]
mod remarkable;

use crate::browser_core::BrowserCore;
use crate::settings::load_settings;

pub const CANVAS_WIDTH: u32 = 1404;
pub const CANVAS_HEIGHT: u32 = 1872;
pub const CANVAS_MARGIN_X: u32 = 100;
pub const CANVAS_MARGIN_TOP: u32 = 150;
pub const CANVAS_MARGIN_BOTTOM: u32 = 150;
pub const DEBUG_LAYOUT: bool = false;

#[cfg(feature = "static")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    let settings = load_settings(settings_file_path).await.unwrap();
    info!("Settings: {:#?}", settings);

    let mut browser = BrowserCore::new(settings.clone());
    browser.navigate_to(&url).await;

    spawn_blocking(move || {
        for page in browser.get_pages().iter().enumerate() {
            let (page_idx, page_canvas) = page;

            let file_path = format!("./output/page-{}.png", page_idx);
            page_canvas.save(&file_path).expect("Failed to save image");
        }
    });

    info!("Done");

    Ok(())
}

#[cfg(feature = "remarkable")]
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    env_logger::init();

    let settings_file_path = "/home/root/.config/skelly/settings.json";
    let settings = load_settings(settings_file_path).await.unwrap();
    info!("Settings: {:#?}", settings);

    let mut app = remarkable::RemarkableApp::new(settings);
    app.run().await?;

    Ok(())
}

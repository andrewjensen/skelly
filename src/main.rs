use image::{ImageFormat, RgbaImage};
use log::{error, info};
use std::env;
use std::io::Cursor;
use std::process;
use std::sync::mpsc::channel;

mod application;
mod backend;
mod browser_core;
mod debugging;
mod keyboard;
mod network;
mod parsing;
mod rendering;
mod settings;
mod web_server;

#[cfg(feature = "desktop")]
mod desktop_backend;

#[cfg(feature = "remarkable")]
mod remarkable_backend;

use crate::application::{Application, OutputEvent, UserInputEvent};
use crate::backend::Backend;
use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::load_settings_with_fallback;
use crate::web_server::run_web_server;

pub const CANVAS_WIDTH: u32 = 1404;
pub const CANVAS_HEIGHT: u32 = 1872;
pub const CANVAS_MARGIN_TOP: u32 = 150;
pub const CANVAS_MARGIN_BOTTOM: u32 = 150;
pub const DEBUG_LAYOUT: bool = false;

fn main() {
    #[cfg(feature = "static")]
    {
        main_static();
    }

    #[cfg(not(feature = "static"))]
    {
        env_logger::init();

        let settings_file_path = "/home/root/.config/skelly/settings.json";
        let settings = load_settings_with_fallback(settings_file_path);
        info!("Settings: {:#?}", settings);

        let (user_input_tx, user_input_rx) = channel::<UserInputEvent>();
        let (output_tx, output_rx) = channel::<OutputEvent>();

        // Start the core application...
        let mut app = Application::new(settings.clone(), user_input_rx, output_tx);
        let app_handle = std::thread::spawn(move || {
            app.run()
                .map_err(|e| -> Box<dyn std::error::Error + Send + Sync> {
                    Box::new(std::io::Error::new(
                        std::io::ErrorKind::Other,
                        e.to_string(),
                    ))
                })
        });

        // Then start the web server...
        let user_input_tx_for_web_server = user_input_tx.clone();
        let web_server_handle = std::thread::spawn(move || {
            run_web_server(user_input_tx_for_web_server);
        });

        // Then start the platform-specific backend...
        let user_input_tx_for_backend = user_input_tx.clone();

        #[cfg(feature = "desktop")]
        let mut backend =
            desktop_backend::DesktopBackend::new(user_input_tx_for_backend, output_rx);

        #[cfg(feature = "remarkable")]
        let mut backend =
            remarkable_backend::RemarkableBackend::new(user_input_tx_for_backend, output_rx);

        backend.run().unwrap();

        // Wait for the application to finish and the other processes will be finished too
        app_handle.join().unwrap().unwrap();
    }
}

#[allow(dead_code)]
fn main_static() {
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
    let settings = load_settings_with_fallback(settings_file_path);
    info!("Settings: {:#?}", settings);

    let mut browser = BrowserCore::new(settings.clone());
    browser.navigate_to(&url);

    if let BrowserState::PageError { url: _, error } = browser.state {
        error!(
            "Error loading page! Cannot render the result. Reason: {}",
            error
        );
        process::exit(1);
    }

    info!("Saving pages to PNG files...");
    let mut handles = vec![];
    for (page_idx, page_canvas) in browser.get_pages().iter().enumerate() {
        let page_canvas = page_canvas.clone();
        let file_path = format!("./output/page-{}.png", page_idx);
        let handle = std::thread::spawn(move || {
            save_page_canvas(page_canvas, &file_path).unwrap();
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.join().unwrap();
    }

    info!("Done");
}

#[allow(dead_code)]
fn save_page_canvas(
    page_canvas: RgbaImage,
    file_path: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut png_buffer = Cursor::new(Vec::new());
    page_canvas.write_to(&mut png_buffer, ImageFormat::Png)?;

    std::fs::write(file_path, png_buffer.into_inner())?;

    Ok(())
}

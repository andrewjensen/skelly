use log::{error, info};
use std::env;
use std::process;

mod app;
mod app_backend;
mod debugging;
mod layout;
mod network;
mod parsing;
mod rendering;

use crate::app::App;
use crate::app_backend::get_app_backend;

pub const CANVAS_WIDTH: u32 = 1404;
pub const CANVAS_HEIGHT: u32 = 1872;
pub const CANVAS_MARGIN_X: u32 = 100;
pub const CANVAS_MARGIN_TOP: u32 = 150;
pub const CANVAS_MARGIN_BOTTOM: u32 = 150;
pub const DEBUG_LAYOUT: bool = false;

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

    let app_backend = get_app_backend();
    let mut app = App::new(app_backend);

    app.temp_set_initial_url(url);

    app.run();

    info!("Done");
}

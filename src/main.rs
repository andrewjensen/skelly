use cgmath::Point2;
use cosmic_text::{Buffer, Color, FontSystem, LayoutRun, Metrics, SwashCache};
use image::{Pixel, Rgba, RgbaImage};
use log::{error, info};
use std::env;
use std::process;

mod debugging;
mod layout;
mod network;
mod parsing;
mod rendering;

use crate::layout::split_runs_into_pages;
use crate::network::{fetch_webpage, ContentType};
use crate::parsing::parse_webpage;
use crate::rendering::{draw_box_border, draw_layout_runs, set_buffer_text};

const CANVAS_WIDTH: u32 = 1404;
const CANVAS_HEIGHT: u32 = 1872;
const CANVAS_MARGIN_X: u32 = 100;
const CANVAS_MARGIN_TOP: u32 = 200;
const CANVAS_MARGIN_BOTTOM: u32 = 400;

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

    info!("Creating cosmic-text buffer...");

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    let display_scale: f32 = 1.0;
    let metrics = Metrics::new(32.0, 44.0);
    let mut buffer = Buffer::new_empty(metrics.scale(display_scale));

    let content_height = CANVAS_HEIGHT - CANVAS_MARGIN_TOP - CANVAS_MARGIN_BOTTOM;

    let buffer_width = CANVAS_WIDTH - CANVAS_MARGIN_X * 2;
    let buffer_height = i32::MAX; // No limit on height so the buffer will calculate everything

    buffer.set_size(
        &mut font_system,
        Some(buffer_width as f32),
        Some(buffer_height as f32),
    );

    let text_color = Color::rgba(0x34, 0x34, 0x34, 0xFF);

    set_buffer_text(&mut buffer, &mut font_system, &document);

    info!("Splitting text into pages...");
    let all_runs: Vec<LayoutRun> = buffer.layout_runs().collect();
    let all_runs_refs: Vec<&LayoutRun> = all_runs.iter().collect();
    let pages = split_runs_into_pages(all_runs_refs, content_height);
    info!("Split into {} total pages", pages.len());

    info!("Rendering pages...");

    for (page_idx, page) in pages.iter().enumerate() {
        info!("  Page {}...", page_idx);

        let mut pixel_data = RgbaImage::new(CANVAS_WIDTH, CANVAS_HEIGHT);
        pixel_data.pixels_mut().for_each(|pixel| {
            pixel.0 = [0xFF, 0xFF, 0xFF, 0xFF];
        });

        draw_layout_runs(
            &page.runs,
            (page.offset * -1.0).round() as i32,
            &mut font_system,
            &mut swash_cache,
            text_color,
            |buffer_x, buffer_y, color| {
                let canvas_x = buffer_x + CANVAS_MARGIN_X as i32;
                let canvas_y = buffer_y + CANVAS_MARGIN_TOP as i32;

                if canvas_x < 0 || canvas_x >= CANVAS_WIDTH as i32 {
                    return;
                }
                if canvas_y < 0 || canvas_y >= CANVAS_HEIGHT as i32 {
                    return;
                }

                let canvas_x = canvas_x as u32;
                let canvas_y = canvas_y as u32;

                let (fg_r, fg_g, fg_b, fg_a) = color.as_rgba_tuple();
                let fg = Rgba([fg_r, fg_g, fg_b, fg_a]);

                let bg = pixel_data.get_pixel(canvas_x, canvas_y);
                let mut result = bg.clone();
                result.blend(&fg);
                pixel_data.put_pixel(canvas_x, canvas_y, result);
            },
        );

        let box_top_left = Point2::<u32> {
            x: CANVAS_MARGIN_X,
            y: CANVAS_MARGIN_TOP,
        };
        let box_bottom_right = Point2::<u32> {
            x: CANVAS_WIDTH - CANVAS_MARGIN_X,
            y: CANVAS_HEIGHT - CANVAS_MARGIN_BOTTOM,
        };
        draw_box_border(
            box_top_left,
            box_bottom_right,
            Rgba([0xFF, 0x00, 0x00, 0xFF]),
            &mut pixel_data,
        );

        let file_path = format!("./output/page-{}.png", page_idx);

        pixel_data.save(&file_path).expect("Failed to save image");
    }

    info!("Done");
}

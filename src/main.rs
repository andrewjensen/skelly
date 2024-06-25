use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::Color;
use cosmic_text::Shaping;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, SwashCache, Weight};
use image::Pixel;
use image::{Rgba, RgbaImage};
use log::{error, info};
use std::env;
use std::process;

mod network;
mod parsing;

use crate::network::{fetch_webpage, ContentType};
use crate::parsing::{parse_webpage, Block, Document};

const CANVAS_WIDTH: usize = 1404;
const CANVAS_HEIGHT: usize = 1872;
const CANVAS_MARGIN_X: usize = 100;
const CANVAS_MARGIN_TOP: usize = 200;

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

    let mut pixel_data = RgbaImage::new(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32);
    pixel_data.pixels_mut().for_each(|pixel| {
        pixel.0 = [0xFF, 0xFF, 0xFF, 0xFF];
    });

    info!("Creating cosmic-text buffer...");

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    let display_scale: f32 = 1.0;
    let metrics = Metrics::new(32.0, 44.0);
    let mut buffer = Buffer::new_empty(metrics.scale(display_scale));

    let buffer_width = CANVAS_WIDTH - CANVAS_MARGIN_X * 2;
    let buffer_height = CANVAS_HEIGHT - CANVAS_MARGIN_TOP;
    buffer.set_size(
        &mut font_system,
        Some(buffer_width as f32),
        Some(buffer_height as f32),
    );

    let mut buffer = buffer.borrow_with(&mut font_system);

    let text_color = Color::rgba(0x34, 0x34, 0x34, 0xFF);

    set_buffer_text(&mut buffer, &document);

    info!("Drawing text...");

    buffer.draw(&mut swash_cache, text_color, |x, y, w, h, color| {
        if w > 1 || h > 1 {
            info!("Drawing a rectangle with bigger width/height");
        }

        for buffer_x in x..(x + w as i32) {
            for buffer_y in y..(y + h as i32) {
                let canvas_x = buffer_x + CANVAS_MARGIN_X as i32;
                let canvas_y = buffer_y + CANVAS_MARGIN_TOP as i32;

                if canvas_x < 0 || canvas_x >= CANVAS_WIDTH as i32 {
                    continue;
                }
                if canvas_y < 0 || canvas_y >= CANVAS_HEIGHT as i32 {
                    continue;
                }

                let (fg_r, fg_g, fg_b, fg_a) = color.as_rgba_tuple();
                let fg = Rgba([fg_r, fg_g, fg_b, fg_a]);

                let bg = pixel_data.get_pixel(canvas_x as u32, canvas_y as u32);
                let mut result = bg.clone();
                result.blend(&fg);
                pixel_data.put_pixel(canvas_x as u32, canvas_y as u32, result);
            }
        }
    });

    info!("Saving image...");
    pixel_data
        .save("./output/screen.png")
        .expect("Failed to save image");

    info!("Done");
}

fn set_buffer_text<'a>(buffer: &mut BorrowedWithFontSystem<'a, Buffer>, document: &Document) {
    let attrs_default = Attrs::new();
    let attrs_paragraph = attrs_default.metrics(Metrics::relative(32.0, 1.2));

    let attrs_heading = attrs_default.weight(Weight::BOLD).family(Family::Monospace);
    let attrs_h1 = attrs_heading.metrics(Metrics::relative(64.0, 1.2));
    let attrs_h2 = attrs_heading.metrics(Metrics::relative(48.0, 1.2));
    let attrs_h3 = attrs_heading.metrics(Metrics::relative(40.0, 1.2));
    let attrs_h4 = attrs_heading.metrics(Metrics::relative(32.0, 1.2));
    let attrs_h5 = attrs_heading.metrics(Metrics::relative(32.0, 1.2));
    let attrs_h6 = attrs_heading.metrics(Metrics::relative(32.0, 1.2));

    let mut spans: Vec<(&str, Attrs)> = Vec::new();

    for block in document.blocks.iter() {
        match block {
            Block::Heading { level, content } => {
                let attrs_this_heading = match level {
                    1 => attrs_h1,
                    2 => attrs_h2,
                    3 => attrs_h3,
                    4 => attrs_h4,
                    5 => attrs_h5,
                    6 => attrs_h6,
                    _ => unreachable!("Invalid heading level"),
                };

                spans.push((content, attrs_this_heading));
                spans.push(("\n\n", attrs_this_heading));
            }
            Block::Paragraph { content } => {
                spans.push((content, attrs_paragraph));
                spans.push(("\n\n", attrs_paragraph));
            }
            Block::List => {
                spans.push(("(TODO: render list)", attrs_paragraph));
                spans.push(("\n\n", attrs_paragraph));
            }
        }
    }

    buffer.set_rich_text(spans.iter().copied(), attrs_default, Shaping::Advanced);
}

use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::Color;
use cosmic_text::Shaping;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, SwashCache, Weight};
use image::{ImageBuffer, RgbaImage};
use log::{error, info};
use std::env;
use std::process;

mod network;
mod parsing;

use crate::network::fetch_webpage;
use crate::parsing::{parse_webpage, Block, Document};

const CANVAS_WIDTH: usize = 1404;
const CANVAS_HEIGHT: usize = 1872;
const CANVAS_MARGIN_X: usize = 100;
const CANVAS_MARGIN_TOP: usize = 200;

// const FILE_FONT: &str = "./assets/Roboto-Regular.ttf";

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
    let page_html = fetch_webpage(url);

    info!("Parsing...");
    let document = parse_webpage(&page_html);

    // info!("Parsed document: {:#?}", document);

    // Create a vector of pixel data (RGB format)
    let mut pixel_data = vec![255; CANVAS_WIDTH * CANVAS_HEIGHT * 4]; // Initialize with white

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
                let alpha: f32 = fg_a as f32 / 255.0;

                let idx = (canvas_y * (CANVAS_WIDTH as i32) + canvas_x) * 4;
                let idx = idx as usize;

                let bg_r = pixel_data[idx];
                let bg_g = pixel_data[idx + 1];
                let bg_b = pixel_data[idx + 2];

                let result_r = (fg_r as f32 * alpha + bg_r as f32 * (1.0 - alpha)) as u8;
                let result_g = (fg_g as f32 * alpha + bg_g as f32 * (1.0 - alpha)) as u8;
                let result_b = (fg_b as f32 * alpha + bg_b as f32 * (1.0 - alpha)) as u8;

                pixel_data[idx] = result_r;
                pixel_data[idx + 1] = result_g;
                pixel_data[idx + 2] = result_b;
            }
        }
    });

    info!("Saving image...");

    let img: RgbaImage =
        ImageBuffer::from_vec(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32, pixel_data)
            .expect("Failed to create image buffer");

    // Save the image as a PNG file
    img.save("./output/screen.png")
        .expect("Failed to save image");

    info!("Done");
}

fn set_buffer_text<'a>(buffer: &mut BorrowedWithFontSystem<'a, Buffer>, document: &Document) {
    let attrs_default = Attrs::new();
    // let serif_attrs = attrs.family(Family::Serif);
    // let mono_attrs = attrs.family(Family::Monospace);

    let attrs_paragraph = attrs_default.metrics(Metrics::relative(32.0, 1.2));

    let attrs_heading = attrs_default
        .metrics(Metrics::relative(64.0, 1.2))
        .weight(Weight::BOLD)
        .family(Family::Monospace);

    let mut spans: Vec<(&str, Attrs)> = Vec::new();

    for block in document.blocks.iter() {
        match block {
            Block::Heading {
                level: _level,
                content,
            } => {
                spans.push((content, attrs_heading));
                spans.push(("\n\n", attrs_heading));
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

    // let spans: &[(&str, Attrs)] = &[
    //     (
    //         "Document title\n\n",
    //         attrs.metrics(Metrics::relative(64.0, 1.2)),
    //     ),
    //     // (LOREM_IPSUM, attrs.metrics(Metrics::relative(24.0, 1.2))),
    //     ("Sans-Serif Normal ", attrs),
    //     ("Sans-Serif Bold ", attrs.weight(Weight::BOLD)),
    //     ("Sans-Serif Italic ", attrs.style(Style::Italic)),
    //     ("Serif Normal ", serif_attrs),
    //     ("Serif Bold ", serif_attrs.weight(Weight::BOLD)),
    //     ("Serif Italic ", serif_attrs.style(Style::Italic)),
    //     (
    //         "Serif Bold Italic\n",
    //         serif_attrs.weight(Weight::BOLD).style(Style::Italic),
    //     ),
    //     ("Mono Normal ", mono_attrs),
    //     ("Mono Bold ", mono_attrs.weight(Weight::BOLD)),
    //     ("Mono Italic ", mono_attrs.style(Style::Italic)),
    //     (
    //         "Mono Bold Italic\n",
    //         mono_attrs.weight(Weight::BOLD).style(Style::Italic),
    //     ),
    //     ("สวัสดีครับ\n", attrs.color(Color::rgb(0xFF, 0x00, 0x00))),
    // ];

    buffer.set_rich_text(spans.iter().copied(), attrs_default, Shaping::Advanced);
}

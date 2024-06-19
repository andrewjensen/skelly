use cgmath::Point2;
use image::{ImageBuffer, RgbImage};
use log::info;
use once_cell::sync::Lazy;
use rusttype::{point, Font, Scale};
use std::fs;

const CANVAS_WIDTH: usize = 700;
const CANVAS_HEIGHT: usize = 1000;

const FONT_SIZE: f32 = 24.0;

const FILE_INPUT: &str = "./assets/simple.md";
// const FILE_INPUT: &str = "./assets/oneline.md";
// const FILE_INPUT: &str = "./assets/thai.md";

pub static DEFAULT_FONT: Lazy<Font<'static>> = Lazy::new(|| {
    Font::try_from_bytes(include_bytes!("../assets/Roboto-Regular.ttf").as_slice())
        .expect("corrupted font data")
});

fn main() {
    env_logger::init();

    info!("Reading input file...");
    let input_text = fs::read_to_string(FILE_INPUT).expect("Failed to read input markdown file");

    let lines: Vec<&str> = input_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    info!("Drawing text...");

    // Create a vector of pixel data (RGB format)
    let mut pixel_data = vec![255; CANVAS_WIDTH * CANVAS_HEIGHT * 3]; // Initialize with white

    for (idx, line_text) in lines.iter().enumerate() {
        info!("Drawing line: {}", line_text);

        let pos: Point2<f32> = Point2 {
            x: 50.0,
            y: 50.0 * ((idx + 1) as f32),
        };

        draw_text(&mut pixel_data, pos, line_text, FONT_SIZE);
    }

    info!("Saving image...");

    // Create an RgbImage from the pixel data
    let img: RgbImage =
        ImageBuffer::from_vec(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32, pixel_data)
            .expect("Failed to create image buffer");

    // Save the image as a PNG file
    img.save("./output/screen.png")
        .expect("Failed to save image");

    info!("Done");
}

fn draw_text(pixel_data: &mut Vec<u8>, pos: Point2<f32>, text: &str, font_size: f32) {
    let scale = Scale::uniform(font_size);
    // The starting positioning of the glyphs (top left corner)
    let start = point(pos.x, pos.y);

    // Loop through the glyphs in the text, positing each one on a line
    for glyph in DEFAULT_FONT.layout(&text, scale, start) {
        if let Some(bounding_box) = glyph.pixel_bounding_box() {
            glyph.draw(|x, y, v| {
                let value = ((1.0 - v).min(1.0) * 255.0).floor() as u8;
                let pixel_x = (x + bounding_box.min.x as u32) as usize;
                let pixel_y = (y + bounding_box.min.y as u32) as usize;

                let offset = (pixel_y * CANVAS_WIDTH + pixel_x) * 3;
                pixel_data[offset] = value; // Red channel
                pixel_data[offset + 1] = value; // Green channel
                pixel_data[offset + 2] = value; // Blue channel
            });
        }
    }
}

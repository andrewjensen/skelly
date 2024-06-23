use cosmic_text::BorrowedWithFontSystem;
use cosmic_text::Color;
use cosmic_text::Shaping;
use cosmic_text::Style;
use cosmic_text::{Attrs, Buffer, Family, FontSystem, Metrics, SwashCache, Weight};
use image::{ImageBuffer, RgbaImage};
use log::{error, info};
use std::fs;

const CANVAS_WIDTH: usize = 1404;
const CANVAS_HEIGHT: usize = 1872;
const CANVAS_MARGIN_X: usize = 100;
const CANVAS_MARGIN_TOP: usize = 200;

const LOREM_IPSUM: &str = "Lorem ipsum dolor sit amet, consectetur adipiscing elit, sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Fermentum leo vel orci porta non. Eget sit amet tellus cras adipiscing enim. Vitae proin sagittis nisl rhoncus mattis rhoncus urna neque. Mi bibendum neque egestas congue quisque egestas. Pellentesque pulvinar pellentesque habitant morbi. Eget est lorem ipsum dolor. Quis imperdiet massa tincidunt nunc pulvinar. Sapien faucibus et molestie ac. Felis donec et odio pellentesque diam volutpat commodo sed. Sed faucibus turpis in eu mi bibendum. Sit amet consectetur adipiscing elit pellentesque habitant morbi tristique senectus. A arcu cursus vitae congue. Venenatis lectus magna fringilla urna porttitor rhoncus dolor. Amet purus gravida quis blandit turpis cursus in hac habitasse. Tortor consequat id porta nibh venenatis cras sed. Pellentesque diam volutpat commodo sed egestas egestas fringilla phasellus. Sit amet facilisis magna etiam tempor orci eu lobortis elementum. Varius duis at consectetur lorem donec massa sapien faucibus. Cursus vitae congue mauris rhoncus aenean vel elit scelerisque mauris.
";

const FILE_INPUT: &str = "./assets/simple.md";
// const FILE_INPUT: &str = "./assets/oneline.md";
// const FILE_INPUT: &str = "./assets/thai.md";

const FILE_FONT: &str = "./assets/Roboto-Regular.ttf";

fn main() {
    env_logger::init();

    info!("Reading input file...");
    let input_text = fs::read_to_string(FILE_INPUT).expect("Failed to read input markdown file");

    let lines: Vec<&str> = input_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    // Create a vector of pixel data (RGB format)
    let mut pixel_data = vec![255; CANVAS_WIDTH * CANVAS_HEIGHT * 4]; // Initialize with white

    info!("Creating cosmic-text buffer...");

    let mut font_system = FontSystem::new();
    let mut swash_cache = SwashCache::new();

    let mut display_scale: f32 = 1.0;
    let metrics = Metrics::new(32.0, 44.0);

    let buffer_width = CANVAS_WIDTH - CANVAS_MARGIN_X * 2;
    let buffer_height = CANVAS_HEIGHT - CANVAS_MARGIN_TOP;

    let mut buffer = Buffer::new_empty(metrics.scale(display_scale));
    buffer.set_size(
        &mut font_system,
        Some(buffer_width as f32),
        Some(buffer_height as f32),
    );

    let mut buffer = buffer.borrow_with(&mut font_system);

    let text_color = Color::rgba(0x34, 0x34, 0x34, 0xFF);

    set_buffer_text(&mut buffer);

    info!("Drawing text...");

    buffer.draw(&mut swash_cache, text_color, |x, y, w, h, color| {
        if w > 1 || h > 1 {
            info!("Drawing a rectangle with bigger width/height");
        }

        for buffer_x in x..(x + w as i32) {
            for buffer_y in y..(y + h as i32) {
                let canvas_x = buffer_x + CANVAS_MARGIN_X as i32;
                let canvas_y = buffer_y + CANVAS_MARGIN_TOP as i32;

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

fn set_buffer_text<'a>(buffer: &mut BorrowedWithFontSystem<'a, Buffer>) {
    let attrs = Attrs::new();
    let serif_attrs = attrs.family(Family::Serif);
    let mono_attrs = attrs.family(Family::Monospace);

    let spans: &[(&str, Attrs)] = &[
        (
            "Document title\n\n",
            attrs.metrics(Metrics::relative(64.0, 1.2)),
        ),
        (LOREM_IPSUM, attrs.metrics(Metrics::relative(24.0, 1.2))),
        ("Sans-Serif Normal ", attrs),
        ("Sans-Serif Bold ", attrs.weight(Weight::BOLD)),
        ("Sans-Serif Italic ", attrs.style(Style::Italic)),
        ("Serif Normal ", serif_attrs),
        ("Serif Bold ", serif_attrs.weight(Weight::BOLD)),
        ("Serif Italic ", serif_attrs.style(Style::Italic)),
        (
            "Serif Bold Italic\n",
            serif_attrs.weight(Weight::BOLD).style(Style::Italic),
        ),
        ("Mono Normal ", mono_attrs),
        ("Mono Bold ", mono_attrs.weight(Weight::BOLD)),
        ("Mono Italic ", mono_attrs.style(Style::Italic)),
        (
            "Mono Bold Italic\n",
            mono_attrs.weight(Weight::BOLD).style(Style::Italic),
        ),
        ("สวัสดีครับ\n", attrs.color(Color::rgb(0xFF, 0x00, 0x00))),
    ];

    buffer.set_rich_text(spans.iter().copied(), attrs, Shaping::Advanced);
}

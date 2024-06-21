use cgmath::Point2;
use image::{ImageBuffer, RgbaImage};
use log::{error, info};
// use once_cell::sync::Lazy;
use std::fs;
use swash::scale::image::Image;
use swash::scale::{Render, ScaleContext, Source, StrikeWith};
use swash::shape::ShapeContext;
use swash::text::cluster::{CharCluster, CharInfo, Parser, Token};
use swash::text::Script;
use swash::zeno::{Format, Vector};
use swash::{FontRef, GlyphId};

const CANVAS_WIDTH: usize = 700;
const CANVAS_HEIGHT: usize = 300;

const FONT_SIZE: f32 = 24.0;

const FILE_INPUT: &str = "./assets/simple.md";
// const FILE_INPUT: &str = "./assets/oneline.md";
// const FILE_INPUT: &str = "./assets/thai.md";

const FILE_FONT: &str = "./assets/Roboto-Regular.ttf";

const DEBUG_GUIDE_LINE: bool = false;

fn main() {
    env_logger::init();

    info!("Reading input file...");
    let input_text = fs::read_to_string(FILE_INPUT).expect("Failed to read input markdown file");

    let lines: Vec<&str> = input_text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .collect();

    let first_line = lines.first().unwrap();

    info!("First line: {}", first_line);

    info!("Drawing text...");

    // Create a vector of pixel data (RGB format)
    let mut pixel_data = vec![255; CANVAS_WIDTH * CANVAS_HEIGHT * 4]; // Initialize with white

    let font_data = std::fs::read(FILE_FONT).unwrap();
    let font = FontRef::from_index(&font_data, 0).unwrap();

    let mut shape_context = ShapeContext::new();
    let mut shaper = shape_context
        .builder(font)
        .script(Script::Latin)
        .size(FONT_SIZE)
        .features(&[("dlig", 1)])
        .build();

    // We'll need the character map for our font
    let charmap = font.charmap();
    // And some storage for the cluster we're working with
    let mut cluster = CharCluster::new();
    // Now we build a cluster parser which takes a script and
    // an iterator that yields a Token per character
    let mut parser = Parser::new(
        Script::Latin,
        first_line.char_indices().map(|(i, ch)| Token {
            // The character
            ch,
            // Offset of the character in code units
            offset: i as u32,
            // Length of the character in code units
            len: ch.len_utf8() as u8,
            // Character information
            info: ch.into(),
            // Pass through user data
            data: 0,
        }),
    );
    // Loop over all of the clusters
    while parser.next(&mut cluster) {
        // info!("Handling cluster: {:?}", cluster.chars());

        // Map all of the characters in the cluster
        // to nominal glyph identifiers
        cluster.map(|ch| charmap.map(ch));
        // Add the cluster to the shaper
        shaper.add_cluster(&cluster);
    }

    let mut scale_context = ScaleContext::new();

    let text_pos = Point2::<u32> { x: 20, y: 100 };

    let mut run_offset_x: f32 = 0.0;

    shaper.shape_with(|cluster| {
        info!("Rendering a cluster...");

        cluster.glyphs.iter().for_each(|glyph| {
            info!("Rendering a glyph...");

            let image = render_glyph(
                &mut scale_context,
                &font,
                FONT_SIZE,
                true,
                glyph.id,
                glyph.x + run_offset_x,
                glyph.y,
            )
            .unwrap();

            let glyph_image_data = image.data.as_slice();

            info!(
                "Image placement: top {} height {}",
                image.placement.top, image.placement.height
            );

            let glyph_x_min: i32 = image.placement.left;
            let glyph_x_max: i32 = image.placement.left + image.placement.width as i32;
            let glyph_y_min: i32 = image.placement.top;
            let glyph_y_max: i32 = image.placement.top + image.placement.height as i32;

            for glyph_x in glyph_x_min..glyph_x_max {
                for glyph_y in glyph_y_min..glyph_y_max {
                    // Get the value of this pixel
                    let glyph_byte_offset = (glyph_x - glyph_x_min
                        + (glyph_y - glyph_y_min) * image.placement.width as i32)
                        * 4;

                    if glyph_byte_offset < 0 {
                        error!("Glyph byte offset is less than 0, cannot lookup pixel data");
                        panic!("Invalid glyph byte offset");
                    }

                    let glyph_byte_offset = glyph_byte_offset as usize;

                    let pixel_r = 255 - glyph_image_data[glyph_byte_offset];
                    let pixel_g = 255 - glyph_image_data[glyph_byte_offset + 1];
                    let pixel_b = 255 - glyph_image_data[glyph_byte_offset + 2];

                    if pixel_r == 255 && pixel_g == 255 && pixel_b == 255 {
                        // Blank pixel, skip
                        continue;
                    }

                    // Copy the value onto the canvas, at an offset position
                    let canvas_x = glyph_x + text_pos.x as i32 + run_offset_x.round() as i32; // TODO: I think we might be missing a glyph offset
                    let canvas_y = glyph_y + text_pos.y as i32 - (glyph_y_min * 2);

                    let canvas_byte_offset = (canvas_y * (CANVAS_WIDTH as i32) + canvas_x) * 4;

                    if canvas_byte_offset < 0 {
                        error!("canvas byte offset is less than 0, cannot set pixel data");
                        panic!("Invalid canvas byte offset");
                    }

                    let canvas_byte_offset = canvas_byte_offset as usize;

                    pixel_data[canvas_byte_offset] = pixel_r;
                    pixel_data[canvas_byte_offset + 1] = pixel_g;
                    pixel_data[canvas_byte_offset + 2] = pixel_b;
                }
            }

            run_offset_x += glyph.advance.round();
        });
    });

    if DEBUG_GUIDE_LINE {
        info!("Drawing guide line...");
        for x in 0..CANVAS_WIDTH {
            let y = text_pos.y as usize;
            let byte_offset: usize = (y * CANVAS_WIDTH + x) * 4;
            pixel_data[byte_offset] = 255;
            pixel_data[byte_offset + 1] = 0;
            pixel_data[byte_offset + 2] = 0;
        }
    }

    info!("Saving image...");

    let img: RgbaImage =
        ImageBuffer::from_vec(CANVAS_WIDTH as u32, CANVAS_HEIGHT as u32, pixel_data)
            .expect("Failed to create image buffer");

    // Save the image as a PNG file
    img.save("./output/screen.png")
        .expect("Failed to save image");

    info!("Done");
}

fn render_glyph(
    context: &mut ScaleContext,
    font: &FontRef,
    size: f32,
    hint: bool,
    glyph_id: GlyphId,
    x: f32,
    y: f32,
) -> Option<Image> {
    // Build the scaler
    let mut scaler = context.builder(*font).size(size).hint(hint).build();
    // Compute the fractional offset-- you'll likely want to quantize this
    // in a real renderer

    let offset = Vector::new(x.fract(), y.fract());
    // let offset = Vector::new(0.0, 0.0);
    // let offset = Vector::new(x, y);

    // Select our source order
    Render::new(&[
        Source::ColorOutline(0),
        Source::ColorBitmap(StrikeWith::BestFit),
        Source::Outline,
    ])
    // Select a subpixel format
    .format(Format::Subpixel)
    // Apply the fractional offset
    .offset(offset)
    // Render the image
    .render(&mut scaler, glyph_id)
}

use cgmath::Point2;
use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, LayoutRun, Metrics, Shaping, SwashCache, Weight,
};
use image::{Rgba, RgbaImage};

use crate::parsing::{Block, Document};

pub fn set_buffer_text<'a>(buffer: &mut Buffer, font_system: &mut FontSystem, document: &Document) {
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

    buffer.set_rich_text(
        font_system,
        spans.iter().copied(),
        attrs_default,
        Shaping::Advanced,
    );
}

pub fn draw_layout_runs<F>(
    runs: &Vec<&LayoutRun>,
    offset_y: i32,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    default_color: Color,
    mut f: F,
) where
    F: FnMut(i32, i32, Color),
{
    for run in runs.iter() {
        for glyph in run.glyphs.iter() {
            let physical_glyph = glyph.physical((0., 0.), 1.0);

            let glyph_color = match glyph.color_opt {
                Some(some) => some,
                None => default_color,
            };

            cache.with_pixels(
                font_system,
                physical_glyph.cache_key,
                glyph_color,
                |x, y, color| {
                    f(
                        physical_glyph.x + x,
                        offset_y + run.line_y as i32 + physical_glyph.y + y,
                        color,
                    );
                },
            );
        }
    }
}

pub fn draw_box_border(
    box_top_left: Point2<u32>,
    box_bottom_right: Point2<u32>,
    color: Rgba<u8>,
    pixel_data: &mut RgbaImage,
) {
    // Top and bottom borders
    for x in box_top_left.x..box_bottom_right.x {
        pixel_data.put_pixel(x, box_top_left.y, color);
        pixel_data.put_pixel(x, box_bottom_right.y, color);
    }

    // Left and right borders
    for y in box_top_left.y..box_bottom_right.y {
        pixel_data.put_pixel(box_top_left.x, y, color);
        pixel_data.put_pixel(box_bottom_right.x, y, color);
    }
}

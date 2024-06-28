use cgmath::Point2;
use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, LayoutRun, Metrics, Shaping, SwashCache, Weight,
};
use image::{Pixel, Rgba, RgbaImage};
use log::info;

use crate::layout::split_runs_into_pages;
use crate::parsing::{Block, Document};

use crate::{
    CANVAS_HEIGHT, CANVAS_MARGIN_BOTTOM, CANVAS_MARGIN_TOP, CANVAS_MARGIN_X, CANVAS_WIDTH,
};

pub struct Renderer {
    buffer: Buffer,
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl Renderer {
    pub fn new() -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();

        let display_scale: f32 = 1.0;
        let metrics = Metrics::new(32.0, 44.0);
        let mut buffer = Buffer::new_empty(metrics.scale(display_scale));

        let buffer_width = CANVAS_WIDTH - CANVAS_MARGIN_X * 2;
        let buffer_height = i32::MAX; // No limit on height so the buffer will calculate everything

        buffer.set_size(
            &mut font_system,
            Some(buffer_width as f32),
            Some(buffer_height as f32),
        );

        Renderer {
            buffer,
            font_system,
            swash_cache,
        }
    }

    pub fn render_document(&mut self, document: &Document) -> Vec<RgbaImage> {
        let content_height = CANVAS_HEIGHT - CANVAS_MARGIN_TOP - CANVAS_MARGIN_BOTTOM;

        let text_color = Color::rgba(0x34, 0x34, 0x34, 0xFF);

        self.set_buffer_text(&document);

        info!("Splitting text into pages...");
        let all_runs: Vec<LayoutRun> = self.buffer.layout_runs().collect();
        let all_runs_refs: Vec<&LayoutRun> = all_runs.iter().collect();
        let pages = split_runs_into_pages(all_runs_refs, content_height);
        info!("Split into {} total pages", pages.len());

        info!("Rendering each page...");

        let mut page_canvases = vec![];
        for (page_idx, page) in pages.iter().enumerate() {
            info!("  Page {}...", page_idx);

            let mut page_canvas = RgbaImage::new(CANVAS_WIDTH, CANVAS_HEIGHT);
            page_canvas.pixels_mut().for_each(|pixel| {
                pixel.0 = [0xFF, 0xFF, 0xFF, 0xFF];
            });

            draw_layout_runs(
                &page.runs,
                (page.offset * -1.0).round() as i32,
                &mut self.font_system,
                &mut self.swash_cache,
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

                    let bg = page_canvas.get_pixel(canvas_x, canvas_y);
                    let mut result = bg.clone();
                    result.blend(&fg);
                    page_canvas.put_pixel(canvas_x, canvas_y, result);
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
                &mut page_canvas,
            );

            page_canvases.push(page_canvas);
        }

        page_canvases
    }

    fn set_buffer_text(&mut self, document: &Document) {
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

        self.buffer.set_rich_text(
            &mut self.font_system,
            spans.iter().copied(),
            attrs_default,
            Shaping::Advanced,
        );
    }
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
    page_canvas: &mut RgbaImage,
) {
    // Top and bottom borders
    for x in box_top_left.x..box_bottom_right.x {
        page_canvas.put_pixel(x, box_top_left.y, color);
        page_canvas.put_pixel(x, box_bottom_right.y, color);
    }

    // Left and right borders
    for y in box_top_left.y..box_bottom_right.y {
        page_canvas.put_pixel(box_top_left.x, y, color);
        page_canvas.put_pixel(box_bottom_right.x, y, color);
    }
}
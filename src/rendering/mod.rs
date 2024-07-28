use cgmath::Point2;
use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, LayoutRun, Metrics, Shaping, Style, SwashCache,
    Weight,
};
use image::{Pixel, Rgba, RgbaImage};
use log::info;
use std::fmt;

use crate::keyboard::{add_keyboard_overlay, KeyboardState};
use crate::layout::split_runs_into_pages;
use crate::parsing::{Block, Document, Span, SpanStyle};
use crate::settings::RenderingSettings;

use crate::{CANVAS_HEIGHT, CANVAS_MARGIN_BOTTOM, CANVAS_MARGIN_TOP, CANVAS_WIDTH, DEBUG_LAYOUT};

mod progress;

use progress::add_progress_overlay;

const COLOR_BACKGROUND: Rgba<u8> = Rgba([0xFF, 0xFF, 0xFF, 0xFF]);
const COLOR_DEBUG_LAYOUT: Rgba<u8> = Rgba([0x00, 0xFF, 0xFF, 0xFF]);

const COLOR_LINK: Color = Color::rgba(0x00, 0x00, 0xFF, 0xFF);

const LINK_UNDERLINE_OFFSET_Y: i32 = 2;
const LINK_UNDERLINE_THICKNESS: i32 = 2;

// TODO: make this smaller to waste less RAM and grow when needed
const BLOCK_CANVAS_INITIAL_HEIGHT: u32 = 1000;

pub struct RenderedBlock {
    pub height: u32,
    pub canvas: RgbaImage,
    pub breakpoints: Vec<u32>,
}

impl fmt::Debug for RenderedBlock {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RenderedBlock")
            .field("height", &self.height)
            .field("breakpoints", &self.breakpoints.len())
            .finish()
    }
}

pub struct Renderer<'a> {
    rendering_settings: &'a RenderingSettings,
    buffer: Buffer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    keyboard_state: KeyboardState,
}

impl<'a> Renderer<'a> {
    pub fn new(rendering_settings: &'a RenderingSettings) -> Self {
        let mut font_system = FontSystem::new();
        let swash_cache = SwashCache::new();

        let display_scale: f32 = 2.0;
        let metrics = Metrics::relative(
            rendering_settings.font_size as f32,
            rendering_settings.line_height,
        );
        let mut buffer = Buffer::new_empty(metrics.scale(display_scale));

        let buffer_width = CANVAS_WIDTH - rendering_settings.screen_margin_x * 2;

        buffer.set_size(&mut font_system, Some(buffer_width as f32), None);

        Renderer {
            rendering_settings,
            buffer,
            font_system,
            swash_cache,
            keyboard_state: KeyboardState::Hidden,
        }
    }

    pub fn render_document(&mut self, document: &Document) -> Vec<RgbaImage> {
        let mut finished_page_canvases = vec![];
        let mut current_page_canvas =
            create_blank_canvas(CANVAS_WIDTH, CANVAS_HEIGHT, COLOR_BACKGROUND);

        let mut current_offset_y = CANVAS_MARGIN_TOP;

        let max_y = CANVAS_HEIGHT - CANVAS_MARGIN_BOTTOM;

        for (block_idx, block) in document.blocks.iter().enumerate() {
            info!("Rendering block {}...", block_idx);

            let rendered_block = self.render_block(block);

            info!("Rendered block: {:?}", rendered_block);

            if current_offset_y + rendered_block.height >= max_y {
                info!("Starting a new page for the block");

                finished_page_canvases.push(current_page_canvas);
                current_page_canvas =
                    create_blank_canvas(CANVAS_WIDTH, CANVAS_HEIGHT, COLOR_BACKGROUND);
                current_offset_y = CANVAS_MARGIN_TOP;
            }

            info!("Adding block to current page");

            let block_top_left = Point2::new(0, 0);
            let block_bottom_right = Point2::new(CANVAS_WIDTH - 1, rendered_block.height);

            copy_block_to_page_canvas(
                &rendered_block.canvas,
                &mut current_page_canvas,
                block_top_left,
                block_bottom_right,
                current_offset_y,
            );

            current_offset_y += rendered_block.height;
        }

        finished_page_canvases.push(current_page_canvas);

        let total_pages = finished_page_canvases.len();
        info!("Rendered {} total pages", total_pages);

        for (page_idx, page_canvas) in finished_page_canvases.iter_mut().enumerate() {
            info!("Adding overlays to page {}", page_idx);

            if DEBUG_LAYOUT {
                let box_top_left = Point2::<u32> {
                    x: self.rendering_settings.screen_margin_x,
                    y: CANVAS_MARGIN_TOP,
                };
                let box_bottom_right = Point2::<u32> {
                    x: CANVAS_WIDTH - self.rendering_settings.screen_margin_x,
                    y: CANVAS_HEIGHT - CANVAS_MARGIN_BOTTOM,
                };
                draw_box_border(
                    box_top_left,
                    box_bottom_right,
                    Rgba([0xFF, 0x00, 0x00, 0xFF]),
                    page_canvas,
                );
            }

            add_keyboard_overlay(
                page_canvas,
                &mut self.font_system,
                &mut self.swash_cache,
                &self.keyboard_state,
            );

            add_progress_overlay(
                page_idx,
                total_pages,
                page_canvas,
                &mut self.font_system,
                &mut self.swash_cache,
            );
        }

        finished_page_canvases
    }

    fn render_block(&mut self, block: &Block) -> RenderedBlock {
        let mut canvas = RgbaImage::new(CANVAS_WIDTH, BLOCK_CANVAS_INITIAL_HEIGHT);
        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        let mut breakpoints = vec![];

        self.set_buffer_text(block);

        let text_color = Color::rgba(0x00, 0x00, 0x00, 0xFF);

        let layout_runs: Vec<LayoutRun> = self.buffer.layout_runs().collect();

        let rendered_block_height = layout_runs.last().map_or(0, |layout_run| {
            (layout_run.line_top + layout_run.line_height).ceil() as u32
        });

        for layout_run in layout_runs.iter() {
            draw_layout_run(
                layout_run,
                0,
                &mut self.font_system,
                &mut self.swash_cache,
                text_color,
                |buffer_x, buffer_y, color| {
                    let canvas_x = buffer_x + self.rendering_settings.screen_margin_x as i32;
                    let canvas_y = buffer_y;

                    if canvas_x < 0 || canvas_x >= canvas.width() as i32 {
                        return;
                    }
                    if canvas_y < 0 || canvas_y >= canvas.height() as i32 {
                        // TODO: resize the canvas before this can ever happen
                        return;
                    }

                    let canvas_x = canvas_x as u32;
                    let canvas_y = canvas_y as u32;

                    let (fg_r, fg_g, fg_b, fg_a) = color.as_rgba_tuple();
                    let fg = Rgba([fg_r, fg_g, fg_b, fg_a]);

                    let bg = canvas.get_pixel(canvas_x, canvas_y);
                    let mut result = bg.clone();
                    result.blend(&fg);
                    canvas.put_pixel(canvas_x, canvas_y, result);
                },
            );

            let run_y = layout_run.line_top.round() as u32;
            breakpoints.push(run_y);

            if DEBUG_LAYOUT {
                draw_horizontal_line(
                    self.rendering_settings.screen_margin_x,
                    CANVAS_WIDTH - self.rendering_settings.screen_margin_x,
                    run_y,
                    COLOR_DEBUG_LAYOUT,
                    &mut canvas,
                );
            }
        }

        RenderedBlock {
            height: rendered_block_height,
            canvas,
            breakpoints,
        }
    }

    fn set_buffer_text(&mut self, block: &Block) {
        let display_scale: f32 = 2.0;

        let font_size = self.rendering_settings.font_size as f32;
        let line_height = self.rendering_settings.line_height;

        let attrs_default = Attrs::new();
        let attrs_paragraph =
            attrs_default.metrics(Metrics::relative(font_size, line_height).scale(display_scale));

        let attrs_heading = attrs_default.weight(Weight::BOLD);
        let attrs_h1 = attrs_heading
            .metrics(Metrics::relative(font_size * 2.0, line_height).scale(display_scale));
        let attrs_h2 = attrs_heading
            .metrics(Metrics::relative(font_size * 1.5, line_height).scale(display_scale));
        let attrs_h3 = attrs_heading
            .metrics(Metrics::relative(font_size * 1.25, line_height).scale(display_scale));
        let attrs_h4 =
            attrs_heading.metrics(Metrics::relative(font_size, line_height).scale(display_scale));
        let attrs_h5 =
            attrs_heading.metrics(Metrics::relative(font_size, line_height).scale(display_scale));
        let attrs_h6 =
            attrs_heading.metrics(Metrics::relative(font_size, line_height).scale(display_scale));

        let attrs_block_quote = attrs_default
            .style(Style::Italic)
            .color(Color::rgba(0x99, 0x99, 0x99, 0xFF))
            .metrics(Metrics::relative(font_size, line_height).scale(display_scale));

        let attrs_code_block = attrs_default
            .family(Family::Monospace)
            .color(Color::rgba(0x00, 0x00, 0x00, 0xFF))
            .metrics(Metrics::relative(font_size, line_height).scale(display_scale));

        let mut spans: Vec<(&str, Attrs)> = Vec::new();

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

                for span in content.iter() {
                    // TODO: refactor
                    match &span {
                        &Span::Text {
                            content,
                            style: span_style,
                        } => {
                            let attrs = match span_style {
                                SpanStyle::Normal => attrs_this_heading,
                                SpanStyle::Bold => attrs_this_heading.weight(Weight::BOLD),
                                SpanStyle::Italic => attrs_this_heading.style(Style::Italic),
                                SpanStyle::BoldItalic => {
                                    attrs_this_heading.weight(Weight::BOLD).style(Style::Italic)
                                }
                                SpanStyle::Code => attrs_this_heading.family(Family::Monospace),
                            };

                            spans.push((&content, attrs));
                        }
                        &Span::Link(link) => {
                            spans.push((&link.text, attrs_this_heading.color(COLOR_LINK)));
                        }
                    }
                }

                spans.push(("\n\n", attrs_this_heading));
            }
            Block::Paragraph { content } => {
                for span in content.iter() {
                    // TODO: refactor
                    match &span {
                        &Span::Text {
                            content,
                            style: span_style,
                        } => {
                            let attrs = match span_style {
                                SpanStyle::Normal => attrs_paragraph,
                                SpanStyle::Bold => attrs_paragraph.weight(Weight::BOLD),
                                SpanStyle::Italic => attrs_paragraph.style(Style::Italic),
                                SpanStyle::BoldItalic => {
                                    attrs_paragraph.weight(Weight::BOLD).style(Style::Italic)
                                }
                                SpanStyle::Code => attrs_paragraph.family(Family::Monospace),
                            };

                            spans.push((&content, attrs));
                        }
                        &Span::Link(link) => {
                            spans.push((&link.text, attrs_paragraph.color(COLOR_LINK)));
                        }
                    }
                }
                spans.push(("\n\n", attrs_paragraph));
            }
            Block::List { items } => {
                for item in items.iter() {
                    spans.push(("• ", attrs_paragraph));
                    for span in item.content.iter() {
                        // TODO: refactor
                        match &span {
                            &Span::Text {
                                content,
                                style: span_style,
                            } => {
                                let attrs = match span_style {
                                    SpanStyle::Normal => attrs_paragraph,
                                    SpanStyle::Bold => attrs_paragraph.weight(Weight::BOLD),
                                    SpanStyle::Italic => attrs_paragraph.style(Style::Italic),
                                    SpanStyle::BoldItalic => {
                                        attrs_paragraph.weight(Weight::BOLD).style(Style::Italic)
                                    }
                                    SpanStyle::Code => attrs_paragraph.family(Family::Monospace),
                                };

                                spans.push((&content, attrs));
                            }
                            &Span::Link(link) => {
                                spans.push((&link.text, attrs_paragraph.color(COLOR_LINK)));
                            }
                        }
                    }
                    spans.push(("\n", attrs_paragraph));
                }
                spans.push(("\n", attrs_paragraph));
            }
            Block::Image { alt_text, url } => {
                spans.push(("(TODO: render Block::Image)", attrs_paragraph));
                spans.push(("URL:", attrs_paragraph));
                spans.push((url, attrs_paragraph));
                if let Some(alt_text) = alt_text {
                    spans.push((" Alt text:", attrs_paragraph));
                    spans.push((alt_text, attrs_paragraph));
                }
                spans.push(("\n\n", attrs_paragraph));
            }
            Block::BlockQuote { content } => {
                spans.push((content, attrs_block_quote));
                spans.push(("\n\n", attrs_block_quote));
            }
            Block::ThematicBreak => {
                spans.push(("---\n\n", attrs_default));
            }
            Block::CodeBlock { language, content } => {
                match language {
                    Some(language) => {
                        spans.push(("```", attrs_code_block));
                        spans.push((language, attrs_code_block));
                        spans.push(("\n", attrs_code_block));
                    }
                    None => {
                        spans.push(("```\n", attrs_code_block));
                    }
                }
                spans.push((content, attrs_code_block));
                spans.push(("\n", attrs_code_block));
                spans.push(("```", attrs_code_block));
                spans.push(("\n\n", attrs_code_block));
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

pub fn draw_layout_run<F>(
    run: &LayoutRun,
    offset_y: i32,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    default_color: Color,
    mut f: F,
) where
    F: FnMut(i32, i32, Color),
{
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

        if let Some(color) = glyph.color_opt {
            if color == COLOR_LINK {
                // Draw a blue line underneath the glyph
                let x1 = glyph.x as u32;
                let x2 = (glyph.x + glyph.w) as u32;

                let y = offset_y + run.line_y as i32 + glyph.y as i32 + LINK_UNDERLINE_OFFSET_Y;

                for x in x1..x2 {
                    for y_offset in 0..LINK_UNDERLINE_THICKNESS {
                        f(x as i32, y + y_offset, color);
                    }
                    f(x as i32, y, color);
                }
            }
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
    for x in box_top_left.x..box_bottom_right.x + 1 {
        page_canvas.put_pixel(x, box_top_left.y, color);
        page_canvas.put_pixel(x, box_bottom_right.y, color);
    }

    // Left and right borders
    for y in box_top_left.y..box_bottom_right.y + 1 {
        page_canvas.put_pixel(box_top_left.x, y, color);
        page_canvas.put_pixel(box_bottom_right.x, y, color);
    }
}

pub fn draw_filled_rectangle(
    box_top_left: Point2<u32>,
    box_bottom_right: Point2<u32>,
    color: Rgba<u8>,
    screen: &mut RgbaImage,
) {
    for x in box_top_left.x..box_bottom_right.x + 1 {
        for y in box_top_left.y..box_bottom_right.y + 1 {
            screen.put_pixel(x, y, color);
        }
    }
}

pub fn draw_horizontal_line(x1: u32, x2: u32, y: u32, color: Rgba<u8>, canvas: &mut RgbaImage) {
    for x in x1..x2 + 1 {
        canvas.put_pixel(x, y, color);
    }
}

fn create_blank_canvas(width: u32, height: u32, background_color: Rgba<u8>) -> RgbaImage {
    let mut canvas = RgbaImage::new(width, height);
    for pixel in canvas.pixels_mut() {
        *pixel = background_color;
    }

    canvas
}

fn copy_block_to_page_canvas(
    block_image: &RgbaImage,
    destination_canvas: &mut RgbaImage,
    block_top_left: Point2<u32>,
    block_bottom_right: Point2<u32>,
    offset_y: u32,
) {
    for block_x in block_top_left.x..block_bottom_right.x + 1 {
        for block_y in block_top_left.y..block_bottom_right.y + 1 {
            let pixel = block_image.get_pixel(block_x, block_y);

            let destination_x = block_x;
            let destination_y = block_y + offset_y;
            destination_canvas.put_pixel(destination_x, destination_y, *pixel);
        }
    }
}

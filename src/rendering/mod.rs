use cgmath::Point2;
use cosmic_text::{
    Attrs, Buffer, Color, Family, FontSystem, LayoutRun, Metrics, Shaping, Style, SwashCache,
    Weight,
};
use image::{Pixel, Rgba, RgbaImage};
use log::{debug, info};
use std::fmt;

use crate::browser_core::ImagesByUrl;
use crate::keyboard::{add_keyboard_overlay, KeyboardState};
use crate::network::resolve_url;
use crate::parsing::{Block, Document, ListItem, Span, SpanStyle, TableRow, TableCell};
use crate::settings::RenderingSettings;

use crate::{CANVAS_HEIGHT, CANVAS_MARGIN_BOTTOM, CANVAS_MARGIN_TOP, CANVAS_WIDTH, DEBUG_LAYOUT};

mod helpers;
mod images;
mod progress;

use helpers::{create_blank_canvas, draw_box_border, draw_filled_rectangle, draw_horizontal_line, draw_vertical_line};
use images::{render_placeholder_image_block, rescale_image};
use progress::add_progress_overlay;

const COLOR_BACKGROUND: Rgba<u8> = Rgba([0xFF, 0xFF, 0xFF, 0xFF]);
const COLOR_DEBUG_LAYOUT: Rgba<u8> = Rgba([0x00, 0xFF, 0xFF, 0xFF]);
const COLOR_TABLE_ROW_BORDER: Rgba<u8> = Rgba([0x33, 0x33, 0x33, 0xFF]);
const COLOR_TABLE_CELL_BORDER: Rgba<u8> = Rgba([0x99, 0x99, 0x99, 0xFF]);
const COLOR_BLOCKQUOTE_BORDER: Rgba<u8> = Rgba([0x00, 0x00, 0x00, 0xFF]);

// Using cosmic_text Colors here
const COLOR_TEXT: Color = Color::rgba(0x00, 0x00, 0x00, 0xFF);
const COLOR_LINK: Color = Color::rgba(0x00, 0x00, 0xFF, 0xFF);

const LINK_UNDERLINE_OFFSET_Y: i32 = 2;
const LINK_UNDERLINE_THICKNESS: i32 = 2;

const INDENT_MARGIN_LEFT_EMS: u32 = 2;

const BLOCKQUOTE_BORDER_WIDTH: u32 = 5;

pub struct RenderedBlock {
    pub height: u32,
    pub canvas: RgbaImage,
    pub breakpoints: Vec<u32>,
}

#[derive(Debug, Clone)]
struct BlockRenderSettings {
    pub canvas_width: u32,
    pub margin_left: u32,
    pub margin_right: u32,
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
    webpage_url: String,
    images: ImagesByUrl,
    buffer: Buffer,
    font_system: FontSystem,
    swash_cache: SwashCache,
    keyboard_state: KeyboardState,
}

impl<'a> Renderer<'a> {
    pub fn new(
        rendering_settings: &'a RenderingSettings,
        webpage_url: &str,
        images: ImagesByUrl,
    ) -> Self {
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
            webpage_url: webpage_url.to_string(),
            images,
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

        let mut page_offset_y = CANVAS_MARGIN_TOP;

        let max_y = CANVAS_HEIGHT - CANVAS_MARGIN_BOTTOM;

        let default_render_settings = BlockRenderSettings {
            canvas_width: CANVAS_WIDTH,
            margin_left: self.rendering_settings.screen_margin_x,
            margin_right: self.rendering_settings.screen_margin_x,
        };

        for (block_idx, block) in document.blocks.iter().enumerate() {
            info!("Rendering block {}...", block_idx);

            let rendered_block = self.render_block(block, &default_render_settings);

            info!("Rendered block: {:?}", rendered_block);

            for (breakpoint_idx, breakpoint_y) in rendered_block.breakpoints.iter().enumerate() {
                debug!(
                    "Breakpoint index {}, position {}",
                    breakpoint_idx, breakpoint_y
                );

                let block_segment_height = match rendered_block.breakpoints.get(breakpoint_idx + 1)
                {
                    Some(next_breakpoint_y) => next_breakpoint_y - breakpoint_y,
                    None => rendered_block.height - breakpoint_y,
                };

                if page_offset_y + block_segment_height >= max_y {
                    info!("Starting a new page");

                    finished_page_canvases.push(current_page_canvas);
                    current_page_canvas =
                        create_blank_canvas(CANVAS_WIDTH, CANVAS_HEIGHT, COLOR_BACKGROUND);
                    page_offset_y = CANVAS_MARGIN_TOP;
                }

                debug!(
                    "Adding block segment {} (block offset {}, height {}) to current page at page offset {}",
                    breakpoint_idx, breakpoint_y, block_segment_height, page_offset_y
                );

                let block_top_left = Point2::new(0, *breakpoint_y);
                let block_bottom_right =
                    Point2::new(CANVAS_WIDTH - 1, breakpoint_y + block_segment_height);

                let copy_offset_y = (page_offset_y as i32) - (*breakpoint_y as i32);

                copy_block_to_page_canvas(
                    &rendered_block.canvas,
                    &mut current_page_canvas,
                    block_top_left,
                    block_bottom_right,
                    copy_offset_y,
                );

                page_offset_y += block_segment_height;
            }
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

    fn render_block(&mut self, block: &Block, settings: &BlockRenderSettings) -> RenderedBlock {
        match block {
            Block::Image { url, alt_text } => {
                return self.render_image_block(&url.clone(), alt_text.clone(), settings);
            }
            Block::BlockQuote { content } => {
                return self.render_blockquote_block(content, settings);
            }
            Block::List { items } => {
                return self.render_list_block(items, settings);
            }
            Block::Table { rows } => {
                return self.render_table_block(rows, settings);
            }
            _ => {
                return self.render_text_based_block(block, settings);
            }
        }
    }

    fn render_image_block(
        &mut self,
        url: &str,
        _alt_text: Option<String>,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let resolved_url = resolve_url(&self.webpage_url, url);

        let image_find_result = self.images.get(&resolved_url);

        if let None = image_find_result {
            return render_placeholder_image_block(settings.canvas_width, settings.margin_left);
        }

        let image_load_result = image_find_result.unwrap();

        if let None = image_load_result {
            return render_placeholder_image_block(settings.canvas_width, settings.margin_left);
        }
        let image: &RgbaImage = image_load_result.as_ref().unwrap();

        let image_width = image.width();
        let available_content_width =
            settings.canvas_width - settings.margin_left - settings.margin_right;

        info!("Available content width: {}", available_content_width);

        let image: &RgbaImage = {
            if image_width <= available_content_width {
                image
            } else {
                &rescale_image(image, available_content_width)
            }
        };

        let image_width = image.width();
        let image_height = image.height();

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, image_height);

        for (canvas_x, canvas_y, canvas_pixel) in canvas.enumerate_pixels_mut() {
            if canvas_x < settings.margin_left
                || canvas_x >= settings.canvas_width - settings.margin_right
                || canvas_x >= image_width + settings.margin_left
            {
                *canvas_pixel = COLOR_BACKGROUND;
                continue;
            }

            if canvas_y >= image_height {
                *canvas_pixel = COLOR_BACKGROUND;
                continue;
            }

            let image_x = canvas_x - settings.margin_left;
            let image_y = canvas_y;

            *canvas_pixel = *image.get_pixel(image_x, image_y);
        }

        let breakpoints = vec![0];

        RenderedBlock {
            height: image_height,
            canvas,
            breakpoints,
        }
    }

    fn render_blockquote_block(
        &mut self,
        content: &Vec<Block>,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let mut child_settings: BlockRenderSettings = settings.clone();
        let indent_left = INDENT_MARGIN_LEFT_EMS * self.rendering_settings.font_size;
        child_settings.margin_left += indent_left;

        let mut offset_y = 0;
        let mut breakpoints = vec![];
        let mut rendered_children: Vec<RenderedBlock> = vec![];

        for child_block in content {
            let rendered_child = self.render_block(child_block, &child_settings);

            for breakpoint in rendered_child.breakpoints.iter() {
                breakpoints.push(offset_y + breakpoint);
            }

            offset_y += rendered_child.height;

            rendered_children.push(rendered_child);
        }

        let total_height = offset_y;

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, total_height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        offset_y = 0;
        for rendered_child in rendered_children.iter() {
            for y in 0..rendered_child.height {
                for x in 0..settings.canvas_width {
                    let canvas_pixel = canvas.get_pixel_mut(x, y + offset_y);
                    let child_pixel = rendered_child.canvas.get_pixel(x, y);

                    *canvas_pixel = *child_pixel;
                }
            }

            offset_y += rendered_child.height;
        }

        for y in 0..total_height {
            for x in settings.margin_left..settings.margin_left + BLOCKQUOTE_BORDER_WIDTH {
                *canvas.get_pixel_mut(x, y) = COLOR_BLOCKQUOTE_BORDER;
            }
        }

        RenderedBlock {
            height: total_height,
            canvas,
            breakpoints,
        }
    }

    fn render_list_block(
        &mut self,
        list_items: &Vec<ListItem>,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let list_item_settings: BlockRenderSettings = settings.clone();

        let mut offset_y = 0;
        let mut breakpoints = vec![];
        let mut rendered_list_items: Vec<RenderedBlock> = vec![];

        for child_list_item in list_items {
            let rendered_list_item = self.render_list_item(child_list_item, &list_item_settings);

            for breakpoint in rendered_list_item.breakpoints.iter() {
                breakpoints.push(offset_y + breakpoint);
            }

            offset_y += rendered_list_item.height;

            rendered_list_items.push(rendered_list_item);
        }

        let total_height = offset_y;

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, total_height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        offset_y = 0;
        for rendered_list_item in rendered_list_items.iter() {
            for y in 0..rendered_list_item.height {
                for x in 0..settings.canvas_width {
                    let canvas_pixel = canvas.get_pixel_mut(x, y + offset_y);
                    let child_pixel = rendered_list_item.canvas.get_pixel(x, y);

                    *canvas_pixel = *child_pixel;
                }
            }

            offset_y += rendered_list_item.height;
        }

        RenderedBlock {
            height: total_height,
            canvas,
            breakpoints,
        }
    }

    fn render_list_item(
        &mut self,
        list_item: &ListItem,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let mut child_settings: BlockRenderSettings = settings.clone();
        let indent_left = INDENT_MARGIN_LEFT_EMS * self.rendering_settings.font_size;
        child_settings.margin_left += indent_left;

        let mut offset_y = 0;
        let mut breakpoints = vec![];
        let mut rendered_children: Vec<RenderedBlock> = vec![];

        for child_block in list_item.content.iter() {
            let rendered_child = self.render_block(child_block, &child_settings);

            for breakpoint in rendered_child.breakpoints.iter() {
                breakpoints.push(offset_y + breakpoint);
            }

            offset_y += rendered_child.height;

            rendered_children.push(rendered_child);
        }

        let total_height = offset_y;

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, total_height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        offset_y = 0;
        for rendered_child in rendered_children.iter() {
            for y in 0..rendered_child.height {
                for x in 0..settings.canvas_width {
                    let canvas_pixel = canvas.get_pixel_mut(x, y + offset_y);
                    let child_pixel = rendered_child.canvas.get_pixel(x, y);

                    *canvas_pixel = *child_pixel;
                }
            }

            offset_y += rendered_child.height;
        }

        // TODO: draw bullet point instead of this funny thing
        for y in 0..BLOCKQUOTE_BORDER_WIDTH {
            for x in settings.margin_left..settings.margin_left + BLOCKQUOTE_BORDER_WIDTH {
                *canvas.get_pixel_mut(x, y) = COLOR_BLOCKQUOTE_BORDER;
            }
        }

        RenderedBlock {
            height: total_height,
            canvas,
            breakpoints,
        }
    }

    fn render_table_block(
        &mut self,
        rows: &Vec<TableRow>,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let row_settings: BlockRenderSettings = settings.clone();

        info!("Rendering table with {} rows", rows.len());

        let mut offset_y = 0;
        let mut breakpoints = vec![];
        let mut rendered_rows: Vec<RenderedBlock> = vec![];

        for row in rows {
            let rendered_row = self.render_table_row(row, &row_settings);

            info!("Rendered row: {:?}", rendered_row);

            for breakpoint in rendered_row.breakpoints.iter() {
                breakpoints.push(offset_y + breakpoint);
            }

            offset_y += rendered_row.height;
            rendered_rows.push(rendered_row);
        }

        let total_height = offset_y + 1; // Add 1 pixel for the bottom border
        let mut canvas = RgbaImage::new(CANVAS_WIDTH, total_height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        offset_y = 0;
        for rendered_row in rendered_rows.iter() {
            for y in 0..rendered_row.height {
                for x in 0..settings.canvas_width {
                    let canvas_pixel = canvas.get_pixel_mut(x, y + offset_y);
                    let row_pixel = rendered_row.canvas.get_pixel(x, y);
                    *canvas_pixel = *row_pixel;
                }
            }
            offset_y += rendered_row.height;
        }

        // Draw bottom border
        let bottom_border_y = total_height - 1;
        let bottom_border_start_x = settings.margin_left;
        let bottom_border_end_x = settings.canvas_width - settings.margin_right;
        draw_horizontal_line(
            bottom_border_start_x,
            bottom_border_end_x,
            bottom_border_y,
            COLOR_TABLE_ROW_BORDER,
            &mut canvas
        );

        RenderedBlock {
            height: total_height,
            canvas,
            breakpoints,
        }
    }

    fn render_table_row(
        &mut self,
        row: &TableRow,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let cell_settings: BlockRenderSettings = settings.clone();
        let mut rendered_cells: Vec<RenderedBlock> = vec![];

        info!("Rendering table row with {} cells", row.cells.len());

        // First render all cells
        for (cell_index, cell) in row.cells.iter().enumerate() {
            let rendered_cell = self.render_table_cell(cell, cell_index, &cell_settings);
            rendered_cells.push(rendered_cell);
        }

        // Stack cells vertically
        let mut offset_y = 0;
        let mut breakpoints = vec![];

        // Calculate total height needed for all cells
        let total_height: u32 = rendered_cells.iter()
            .map(|cell| cell.height)
            .sum();

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, total_height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        // Copy each cell's content into the row canvas, stacking vertically
        for rendered_cell in rendered_cells.iter() {
            for y in 0..rendered_cell.height {
                for x in 0..settings.canvas_width {
                    let canvas_pixel = canvas.get_pixel_mut(x, y + offset_y);
                    let cell_pixel = rendered_cell.canvas.get_pixel(x, y);
                    *canvas_pixel = *cell_pixel;
                }
            }

            // Add breakpoint at the start of each cell
            breakpoints.push(offset_y);

            offset_y += rendered_cell.height;
        }

        RenderedBlock {
            height: total_height,
            canvas,
            breakpoints,
        }
    }

    fn render_table_cell(
        &mut self,
        cell: &TableCell,
        cell_index: usize,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let height = 100; // Fixed height as requested
        let mut canvas = RgbaImage::new(CANVAS_WIDTH, height);

        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_DEBUG_LAYOUT;
        }

        // Draw horizontal line at top of cell
        let line_y = 0;
        let line_start_x = settings.margin_left;
        let line_end_x = settings.canvas_width - settings.margin_right;
        let top_line_color = if cell_index == 0 {
            COLOR_TABLE_ROW_BORDER
        } else {
            COLOR_TABLE_CELL_BORDER
        };
        draw_horizontal_line(
            line_start_x,
            line_end_x,
            line_y,
            top_line_color,
            &mut canvas
        );

        // Draw vertical line at left of cell
        let line_x = settings.margin_left;
        let line_start_y = 0;
        let line_end_y = height - 1;
        draw_vertical_line(
            line_x,
            line_start_y,
            line_end_y,
            COLOR_TABLE_ROW_BORDER,
            &mut canvas
        );

        // Draw vertical line at right of cell
        let line_x = settings.canvas_width - settings.margin_right;
        let line_start_y = 0;
        let line_end_y = height - 1;
        draw_vertical_line(
            line_x,
            line_start_y,
            line_end_y,
            COLOR_TABLE_ROW_BORDER,
            &mut canvas
        );

        RenderedBlock {
            height,
            canvas,
            breakpoints: vec![0],
        }
    }

    fn render_text_based_block(
        &mut self,
        block: &Block,
        settings: &BlockRenderSettings,
    ) -> RenderedBlock {
        let mut breakpoints = vec![];

        self.set_buffer_text(block);

        let buffer_width = settings.canvas_width - settings.margin_left - settings.margin_right;
        self.buffer
            .set_size(&mut self.font_system, Some(buffer_width as f32), None);

        let layout_runs: Vec<LayoutRun> = self.buffer.layout_runs().collect();

        let rendered_block_height = layout_runs.last().map_or(0, |layout_run| {
            (layout_run.line_top + layout_run.line_height).ceil() as u32
        });

        let mut canvas = RgbaImage::new(CANVAS_WIDTH, rendered_block_height);
        for pixel in canvas.pixels_mut() {
            *pixel = COLOR_BACKGROUND;
        }

        for layout_run in layout_runs.iter() {
            draw_layout_run(
                layout_run,
                0,
                &mut self.font_system,
                &mut self.swash_cache,
                COLOR_TEXT,
                |buffer_x, buffer_y, color| {
                    let canvas_x = buffer_x + settings.margin_left as i32;
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

        let attrs_heading = attrs_default.clone();
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
            Block::List { items: _ } => {
                unreachable!();
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
            Block::BlockQuote { content: _ } => {
                unreachable!();
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
            Block::Table { .. } => {
                unreachable!();
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

fn copy_block_to_page_canvas(
    block_image: &RgbaImage,
    destination_canvas: &mut RgbaImage,
    block_top_left: Point2<u32>,
    block_bottom_right: Point2<u32>,
    offset_y: i32,
) {
    for block_x in block_top_left.x..block_bottom_right.x + 1 {
        for block_y in block_top_left.y..block_bottom_right.y {
            let pixel = block_image.get_pixel(block_x, block_y);

            let destination_x = block_x;
            let destination_y = (block_y as i32) + offset_y;

            if destination_y < 0 || destination_y >= destination_canvas.height() as i32 {
                continue;
            }

            let destination_y = destination_y as u32;

            destination_canvas.put_pixel(destination_x, destination_y, *pixel);
        }
    }
}

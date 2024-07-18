use cgmath::Point2;
use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::{Pixel, Rgba, RgbaImage};

use crate::rendering::{draw_box_border, draw_filled_rectangle};

const BAR_WIDTH: u32 = 320;
const BAR_HEIGHT: u32 = 16;

const BAR_INNER_OFFSET: u32 = 3;
const OVERLAY_MARGIN_Y: u32 = 80;

const BAR_OUTER_COLOR: Rgba<u8> = Rgba([0xAA, 0xAA, 0xAA, 0xFF]);
const BAR_INNER_COLOR: Rgba<u8> = Rgba([0xDD, 0xDD, 0xDD, 0xFF]);

const TEXT_COLOR: Rgba<u8> = Rgba([0x99, 0x99, 0x99, 0xFF]);
const TEXT_FONT_SIZE: f32 = 20.0;
const TEXT_OFFSET_Y: u32 = 8;

pub fn add_progress_overlay(
    page_idx: usize,
    total_pages: usize,
    screen: &mut RgbaImage,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
) {
    let progress_percent = (page_idx + 1) as f32 / total_pages as f32;
    draw_progress_bar(progress_percent, screen);
    draw_text(page_idx, total_pages, screen, font_system, cache);
}

fn draw_progress_bar(progress_percent: f32, screen: &mut RgbaImage) {
    let outer_box_left_x = (screen.width() - BAR_WIDTH) / 2;
    let outer_box_top_y = screen.height() - OVERLAY_MARGIN_Y - BAR_HEIGHT;

    let inner_bar_width = BAR_WIDTH - BAR_INNER_OFFSET * 2;

    let outer_box_top_left = Point2::<u32> {
        x: outer_box_left_x,
        y: outer_box_top_y,
    };
    let outer_box_bottom_right = Point2::<u32> {
        x: outer_box_top_left.x + BAR_WIDTH,
        y: outer_box_top_y + BAR_HEIGHT,
    };

    draw_box_border(
        outer_box_top_left,
        outer_box_bottom_right,
        BAR_OUTER_COLOR,
        screen,
    );

    let inner_bar_top_left = Point2::<u32> {
        x: outer_box_top_left.x + BAR_INNER_OFFSET,
        y: outer_box_top_left.y + BAR_INNER_OFFSET,
    };

    let inner_bar_bottom_right = Point2::<u32> {
        x: inner_bar_top_left.x + (inner_bar_width as f32 * progress_percent).ceil() as u32,
        y: outer_box_bottom_right.y - BAR_INNER_OFFSET,
    };

    draw_filled_rectangle(
        inner_bar_top_left,
        inner_bar_bottom_right,
        BAR_INNER_COLOR,
        screen,
    );
}

fn draw_text(
    page_idx: usize,
    total_pages: usize,
    screen: &mut RgbaImage,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
) {
    let text = format!("Page {} / {}", page_idx + 1, total_pages);

    let metrics = Metrics::relative(TEXT_FONT_SIZE, 1.0);
    let attrs = Attrs::new().metrics(metrics);
    let text_color = Color::rgba(TEXT_COLOR[0], TEXT_COLOR[1], TEXT_COLOR[2], TEXT_COLOR[3]);

    let mut buffer = Buffer::new_empty(metrics);

    buffer.set_size(font_system, None, None);
    buffer.set_wrap(font_system, Wrap::None);

    buffer.set_text(font_system, &text, attrs, Shaping::Basic);
    buffer.shape_until_scroll(font_system, false);
    let layout_run = buffer.layout_runs().next().unwrap();
    let text_width = layout_run.line_w;

    buffer.draw(
        font_system,
        cache,
        text_color,
        |buffer_x, buffer_y, _, _, color| {
            let canvas_x = buffer_x + ((screen.width() as i32 - text_width as i32) / 2);
            let canvas_y =
                buffer_y + screen.height() as i32 - OVERLAY_MARGIN_Y as i32 + TEXT_OFFSET_Y as i32;

            if canvas_x < 0 || canvas_x >= screen.width() as i32 {
                return;
            }

            if canvas_y < 0 || canvas_y >= screen.height() as i32 {
                return;
            }

            let canvas_x = canvas_x as u32;
            let canvas_y = canvas_y as u32;

            let (fg_r, fg_g, fg_b, fg_a) = color.as_rgba_tuple();
            let fg = Rgba([fg_r, fg_g, fg_b, fg_a]);

            let bg = screen.get_pixel(canvas_x, canvas_y);
            let mut result = bg.clone();
            result.blend(&fg);
            screen.put_pixel(canvas_x, canvas_y, result);
        },
    );
}

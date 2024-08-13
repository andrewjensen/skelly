use cgmath::Point2;
use image::imageops::FilterType;
use image::{DynamicImage, Rgba, RgbaImage};
use log::info;

use super::helpers::{draw_box_border, draw_filled_rectangle};
use super::RenderedBlock;

const COLOR_BACKGROUND: Rgba<u8> = Rgba([0xFF, 0xFF, 0xFF, 0xFF]);
const COLOR_PLACEHOLDER_BG: Rgba<u8> = Rgba([0xEE, 0xEE, 0xEE, 0xFF]);
const COLOR_PLACEHOLDER_BORDER: Rgba<u8> = Rgba([0x99, 0x99, 0x99, 0xFF]);

const PLACEHOLDER_IMAGE_WIDTH: u32 = 300;
const PLACEHOLDER_IMAGE_HEIGHT: u32 = 300;

pub fn rescale_image(raw_image: &RgbaImage, destination_width: u32) -> RgbaImage {
    info!(
        "Resizing image from input resolution {} x {} to destination width {}",
        raw_image.width(),
        raw_image.height(),
        destination_width,
    );

    let raw_image = DynamicImage::ImageRgba8(raw_image.clone());
    let result = raw_image.resize(destination_width, u32::MAX, FilterType::Triangle);

    result.into_rgba8()
}

pub fn render_placeholder_image_block(canvas_width: u32, screen_margin_x: u32) -> RenderedBlock {
    let mut canvas = RgbaImage::new(canvas_width, PLACEHOLDER_IMAGE_HEIGHT);
    let box_top_left = Point2::<u32> {
        x: screen_margin_x,
        y: 0,
    };

    let box_bottom_right = Point2::<u32> {
        x: screen_margin_x + PLACEHOLDER_IMAGE_WIDTH,
        y: PLACEHOLDER_IMAGE_HEIGHT - 1,
    };

    for pixel in canvas.pixels_mut() {
        *pixel = COLOR_BACKGROUND;
    }

    draw_filled_rectangle(
        box_top_left,
        box_bottom_right,
        COLOR_PLACEHOLDER_BG,
        &mut canvas,
    );

    draw_box_border(
        box_top_left,
        box_bottom_right,
        COLOR_PLACEHOLDER_BORDER,
        &mut canvas,
    );

    RenderedBlock {
        height: PLACEHOLDER_IMAGE_HEIGHT,
        canvas,
        breakpoints: vec![0],
    }
}

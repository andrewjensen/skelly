use cgmath::Point2;
use image::{Rgba, RgbaImage};

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

pub fn draw_vertical_line(x: u32, y1: u32, y2: u32, color: Rgba<u8>, canvas: &mut RgbaImage) {
    for y in y1..y2 + 1 {
        canvas.put_pixel(x, y, color);
    }
}

pub fn create_blank_canvas(width: u32, height: u32, background_color: Rgba<u8>) -> RgbaImage {
    let mut canvas = RgbaImage::new(width, height);
    for pixel in canvas.pixels_mut() {
        *pixel = background_color;
    }

    canvas
}

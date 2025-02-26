use cgmath::Point2;
use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::{load_from_memory, Pixel, Rgba, RgbaImage};

use super::helpers::{draw_box_border, draw_filled_rectangle};

pub enum TopbarState {
    Minimized,
    Normal,
}

const MENU_ICON_MARGIN: u32 = 12;
const TOPBAR_HEIGHT: u32 = 72;

const URL_BAR_TEXT_SIZE: f32 = 32.0;
const URL_BAR_TEXT_MARGIN: u32 = 12;

const COLOR_TOPBAR_BACKGROUND: Rgba<u8> = Rgba([0xAA, 0xAA, 0xAA, 0xFF]);
const COLOR_URL_BAR_FILL: Rgba<u8> = Rgba([0xDD, 0xDD, 0xDD, 0xFF]);
const COLOR_URL_BAR_BORDER: Rgba<u8> = Rgba([0x99, 0x99, 0x99, 0xFF]);
const COLOR_URL_BAR_TEXT: Rgba<u8> = Rgba([0x66, 0x66, 0x66, 0xFF]);

const MOCK_URL: &str = "https://www.example.com";

pub fn add_topbar_overlay(
    screen: &mut RgbaImage,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    topbar_state: &TopbarState,
) {
    let icon_menu = load_from_memory(include_bytes!("../../assets/icons/menu-regular-24.png"))
        .unwrap()
        .to_rgba8();

    let menu_icon_offset_y = MENU_ICON_MARGIN;
    let menu_icon_offset_x = screen.width() - icon_menu.width() - MENU_ICON_MARGIN;

    match topbar_state {
        TopbarState::Minimized => {
            draw_icon(screen, &icon_menu, menu_icon_offset_x, menu_icon_offset_y);
        }
        TopbarState::Normal => {
            // Draw the background
            draw_filled_rectangle(
                Point2::new(0, 0),
                Point2::new(screen.width() - 1, TOPBAR_HEIGHT - 1),
                COLOR_TOPBAR_BACKGROUND,
                screen,
            );

            // Draw the menu icon on the right, same position as minimized
            draw_icon(screen, &icon_menu, menu_icon_offset_x, menu_icon_offset_y);

            let icon_arrow_left = load_from_memory(include_bytes!(
                "../../assets/icons/left-arrow-alt-regular-24.png"
            ))
            .unwrap()
            .to_rgba8();
            let icon_arrow_left_offset_x = MENU_ICON_MARGIN;
            draw_icon(
                screen,
                &icon_arrow_left,
                icon_arrow_left_offset_x,
                menu_icon_offset_y,
            );

            let icon_arrow_right = load_from_memory(include_bytes!(
                "../../assets/icons/right-arrow-alt-regular-24.png"
            ))
            .unwrap()
            .to_rgba8();
            let icon_arrow_right_offset_x = MENU_ICON_MARGIN * 2 + icon_arrow_left.width();
            draw_icon(
                screen,
                &icon_arrow_right,
                icon_arrow_right_offset_x,
                menu_icon_offset_y,
            );

            let url_bar_offset_x =
                MENU_ICON_MARGIN * 3 + icon_arrow_left.width() + icon_arrow_right.width();
            let url_bar_offset_y = MENU_ICON_MARGIN;
            let url_bar_width =
                screen.width() - url_bar_offset_x - MENU_ICON_MARGIN * 2 - icon_menu.width();
            let url_bar_height = TOPBAR_HEIGHT - MENU_ICON_MARGIN * 2;

            draw_filled_rectangle(
                Point2::new(url_bar_offset_x, url_bar_offset_y),
                Point2::new(
                    url_bar_offset_x + url_bar_width - 1,
                    url_bar_offset_y + url_bar_height - 1,
                ),
                COLOR_URL_BAR_FILL,
                screen,
            );

            draw_box_border(
                Point2::new(url_bar_offset_x, url_bar_offset_y),
                Point2::new(
                    url_bar_offset_x + url_bar_width - 1,
                    url_bar_offset_y + url_bar_height - 1,
                ),
                COLOR_URL_BAR_BORDER,
                screen,
            );

            let metrics = Metrics::new(URL_BAR_TEXT_SIZE, url_bar_height as f32);
            let attrs = Attrs::new().metrics(metrics);
            let text_color = Color::rgba(
                COLOR_URL_BAR_TEXT[0],
                COLOR_URL_BAR_TEXT[1],
                COLOR_URL_BAR_TEXT[2],
                COLOR_URL_BAR_TEXT[3],
            );

            let mut buffer = Buffer::new_empty(metrics);

            buffer.set_size(font_system, None, None);
            buffer.set_wrap(font_system, Wrap::None);
            buffer.lines.clear();
            buffer.set_text(font_system, MOCK_URL, attrs, Shaping::Basic);
            buffer.shape_until_scroll(font_system, false);

            let layout_run = buffer.layout_runs().next().unwrap();
            let text_width = layout_run.line_w;
            let text_height = 48.0;

            buffer.draw(
                font_system,
                cache,
                text_color,
                |buffer_x, buffer_y, _, _, color| {
                    let canvas_x =
                        buffer_x + (url_bar_offset_x as i32) + URL_BAR_TEXT_MARGIN as i32;

                    let canvas_y =
                        buffer_y + (url_bar_offset_y as i32) + (url_bar_height as i32 / 2)
                            - (text_height as i32 / 2);

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
    }
}

// TODO: Move to a separate module, make it more generic than icons
fn draw_icon(screen: &mut RgbaImage, icon: &RgbaImage, offset_x: u32, offset_y: u32) {
    let icon_width = icon.width();
    let icon_height = icon.height();

    for icon_x in 0..icon_width {
        for icon_y in 0..icon_height {
            let canvas_x = icon_x + offset_x;
            let canvas_y = icon_y + offset_y;

            let fg = icon.get_pixel(icon_x, icon_y);
            let bg = screen.get_pixel(canvas_x, canvas_y);
            let mut result = bg.clone();
            result.blend(&fg);
            screen.put_pixel(canvas_x, canvas_y, result);
        }
    }
}

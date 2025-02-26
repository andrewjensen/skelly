use image::{load_from_memory, Pixel, Rgba, RgbaImage};

pub enum TopbarState {
    Minimized,
    Normal,
}

const MENU_ICON_MARGIN: u32 = 12;
const TOPBAR_HEIGHT: u32 = 72;

const COLOR_TOPBAR_BACKGROUND: Rgba<u8> = Rgba([0xAA, 0xAA, 0xAA, 0xFF]);
const COLOR_URL_BAR_BACKGROUND: Rgba<u8> = Rgba([0x00, 0x00, 0x00, 0xFF]);

pub fn add_topbar_overlay(screen: &mut RgbaImage, topbar_state: &TopbarState) {
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
            for x in 0..screen.width() {
                for y in 0..TOPBAR_HEIGHT {
                    screen.put_pixel(x, y, COLOR_TOPBAR_BACKGROUND);
                }
            }

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

            for x in url_bar_offset_x..url_bar_offset_x + url_bar_width {
                for y in url_bar_offset_y..url_bar_offset_y + url_bar_height {
                    screen.put_pixel(x, y, COLOR_URL_BAR_BACKGROUND);
                }
            }
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

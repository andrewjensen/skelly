use image::{load_from_memory, Pixel, Rgba, RgbaImage};

pub enum TopbarState {
    Minimized,
    Normal,
}

const MENU_ICON_MARGIN: u32 = 12;

pub fn add_topbar_overlay(screen: &mut RgbaImage, topbar_state: &TopbarState) {
    if let TopbarState::Minimized = topbar_state {
        let icon_menu = load_from_memory(include_bytes!("../assets/icons/menu-regular-24.png"));
        let icon_menu = icon_menu.unwrap().to_rgba8();

        let offset_y = MENU_ICON_MARGIN;
        let offset_x = screen.width() - icon_menu.width() - MENU_ICON_MARGIN;

        draw_icon(screen, &icon_menu, offset_x, offset_y);

        return;
    }

    panic!("Not implemented");
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

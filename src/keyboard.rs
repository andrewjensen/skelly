use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::{Pixel, Rgba, RgbaImage};
use log::info;

pub enum KeyboardState {
    Normal,
    Shift,
}

pub enum KeyCode {
    // Lowercase
    LowercaseA,
    LowercaseB,
    LowercaseC,
    LowercaseD,
    LowercaseE,
    LowercaseF,
    LowercaseG,
    LowercaseH,
    LowercaseI,
    LowercaseJ,
    LowercaseK,
    LowercaseL,
    LowercaseM,
    LowercaseN,
    LowercaseO,
    LowercaseP,
    LowercaseQ,
    LowercaseR,
    LowercaseS,
    LowercaseT,
    LowercaseU,
    LowercaseV,
    LowercaseW,
    LowercaseX,
    LowercaseY,
    LowercaseZ,
    // Uppercase
    UppercaseA,
    UppercaseB,
    UppercaseC,
    UppercaseD,
    UppercaseE,
    UppercaseF,
    UppercaseG,
    UppercaseH,
    UppercaseI,
    UppercaseJ,
    UppercaseK,
    UppercaseL,
    UppercaseM,
    UppercaseN,
    UppercaseO,
    UppercaseP,
    UppercaseQ,
    UppercaseR,
    UppercaseS,
    UppercaseT,
    UppercaseU,
    UppercaseV,
    UppercaseW,
    UppercaseX,
    UppercaseY,
    UppercaseZ,
    // Numbers
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
}

const KEYBOARD_MARGIN_Y: u32 = 100;
const KEY_UNIT_WIDTH: u32 = 100;
const KEY_UNIT_HEIGHT: u32 = 80;
const KEY_GUTTER: u32 = 10;

pub fn add_keyboard_overlay(
    screen: &mut RgbaImage,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    keyboard_state: KeyboardState,
) {
    let keys = get_keys(&keyboard_state);

    let keyboard_keys_height =
        keys.len() as u32 * KEY_UNIT_HEIGHT + (keys.len() as u32 - 1) * KEY_GUTTER;
    let keyboard_total_height = keyboard_keys_height + (KEYBOARD_MARGIN_Y * 2);
    let keyboard_offset_y = screen.height() - keyboard_total_height;

    // Render the background
    let background_color = match &keyboard_state {
        KeyboardState::Normal => Rgba([255, 0, 0, 255]),
        KeyboardState::Shift => Rgba([0, 0, 255, 255]),
    };
    for x in 0..screen.width() {
        for y in keyboard_offset_y..keyboard_total_height + keyboard_offset_y {
            screen.put_pixel(x, y, background_color);
        }
    }

    let metrics = Metrics::new(40.0, 48.0);
    let attrs = Attrs::new().metrics(metrics);
    let text_color = Color::rgba(0xFF, 0xFF, 0xFF, 0xFF);

    let mut buffer = Buffer::new_empty(metrics);

    buffer.set_size(font_system, None, None);
    buffer.set_wrap(font_system, Wrap::None);

    // Render each key
    let key_background_color = Rgba([0, 0, 0, 255]);
    for (line_idx, key_line) in keys.iter().enumerate() {
        for (key_idx, key) in key_line.iter().enumerate() {
            let top_left_x = (KEY_UNIT_WIDTH + KEY_GUTTER) * key_idx as u32;
            let top_left_y = keyboard_offset_y
                + KEYBOARD_MARGIN_Y
                + (KEY_UNIT_HEIGHT + KEY_GUTTER) * line_idx as u32;

            // Render key background
            for key_x in top_left_x..top_left_x + KEY_UNIT_WIDTH {
                for key_y in top_left_y..top_left_y + KEY_UNIT_HEIGHT {
                    screen.put_pixel(key_x, key_y, key_background_color);
                }
            }

            // Render key text
            buffer.lines.clear();
            let key_text = get_key_text(key).unwrap();
            buffer.set_text(font_system, key_text, attrs, Shaping::Basic);
            buffer.shape_until_scroll(font_system, false);
            let layout_run = buffer.layout_runs().next().unwrap();
            let text_width = layout_run.line_w;
            let text_height = 48.0;

            buffer.draw(
                font_system,
                cache,
                text_color,
                |buffer_x, buffer_y, _, _, color| {
                    let canvas_x = buffer_x + (top_left_x as i32) + (KEY_UNIT_WIDTH as i32 / 2)
                        - ((text_width.round() / 2.0) as i32);

                    let canvas_y = buffer_y + (top_left_y as i32) + (KEY_UNIT_HEIGHT as i32 / 2)
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

fn get_keys(state: &KeyboardState) -> Vec<Vec<KeyCode>> {
    match state {
        KeyboardState::Normal => get_normal_keys(),
        KeyboardState::Shift => get_shift_keys(),
    }
}

fn get_normal_keys() -> Vec<Vec<KeyCode>> {
    vec![
        vec![
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
            KeyCode::Digit0,
        ],
        vec![
            KeyCode::LowercaseQ,
            KeyCode::LowercaseW,
            KeyCode::LowercaseE,
            KeyCode::LowercaseR,
            KeyCode::LowercaseT,
            KeyCode::LowercaseY,
            KeyCode::LowercaseU,
            KeyCode::LowercaseI,
            KeyCode::LowercaseO,
            KeyCode::LowercaseP,
        ],
        vec![
            KeyCode::LowercaseA,
            KeyCode::LowercaseS,
            KeyCode::LowercaseD,
            KeyCode::LowercaseF,
            KeyCode::LowercaseG,
            KeyCode::LowercaseH,
            KeyCode::LowercaseJ,
            KeyCode::LowercaseK,
            KeyCode::LowercaseL,
        ],
        vec![
            KeyCode::LowercaseZ,
            KeyCode::LowercaseX,
            KeyCode::LowercaseC,
            KeyCode::LowercaseV,
            KeyCode::LowercaseB,
            KeyCode::LowercaseN,
            KeyCode::LowercaseM,
        ],
    ]
}

fn get_shift_keys() -> Vec<Vec<KeyCode>> {
    vec![
        vec![
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
            KeyCode::Digit0,
        ],
        vec![
            KeyCode::UppercaseQ,
            KeyCode::UppercaseW,
            KeyCode::UppercaseE,
            KeyCode::UppercaseR,
            KeyCode::UppercaseT,
            KeyCode::UppercaseY,
            KeyCode::UppercaseU,
            KeyCode::UppercaseI,
            KeyCode::UppercaseO,
            KeyCode::UppercaseP,
        ],
        vec![
            KeyCode::UppercaseA,
            KeyCode::UppercaseS,
            KeyCode::UppercaseD,
            KeyCode::UppercaseF,
            KeyCode::UppercaseG,
            KeyCode::UppercaseH,
            KeyCode::UppercaseJ,
            KeyCode::UppercaseK,
            KeyCode::UppercaseL,
        ],
        vec![
            KeyCode::UppercaseZ,
            KeyCode::UppercaseX,
            KeyCode::UppercaseC,
            KeyCode::UppercaseV,
            KeyCode::UppercaseB,
            KeyCode::UppercaseN,
            KeyCode::UppercaseM,
        ],
    ]
}

fn get_key_text(key_code: &KeyCode) -> Option<&'static str> {
    match key_code {
        // Lowercase
        KeyCode::LowercaseA => Some("a"),
        KeyCode::LowercaseB => Some("b"),
        KeyCode::LowercaseC => Some("c"),
        KeyCode::LowercaseD => Some("d"),
        KeyCode::LowercaseE => Some("e"),
        KeyCode::LowercaseF => Some("f"),
        KeyCode::LowercaseG => Some("g"),
        KeyCode::LowercaseH => Some("h"),
        KeyCode::LowercaseI => Some("i"),
        KeyCode::LowercaseJ => Some("j"),
        KeyCode::LowercaseK => Some("k"),
        KeyCode::LowercaseL => Some("l"),
        KeyCode::LowercaseM => Some("m"),
        KeyCode::LowercaseN => Some("n"),
        KeyCode::LowercaseO => Some("o"),
        KeyCode::LowercaseP => Some("p"),
        KeyCode::LowercaseQ => Some("q"),
        KeyCode::LowercaseR => Some("r"),
        KeyCode::LowercaseS => Some("s"),
        KeyCode::LowercaseT => Some("t"),
        KeyCode::LowercaseU => Some("u"),
        KeyCode::LowercaseV => Some("v"),
        KeyCode::LowercaseW => Some("w"),
        KeyCode::LowercaseX => Some("x"),
        KeyCode::LowercaseY => Some("y"),
        KeyCode::LowercaseZ => Some("z"),
        // Uppercase
        KeyCode::UppercaseA => Some("A"),
        KeyCode::UppercaseB => Some("B"),
        KeyCode::UppercaseC => Some("C"),
        KeyCode::UppercaseD => Some("D"),
        KeyCode::UppercaseE => Some("E"),
        KeyCode::UppercaseF => Some("F"),
        KeyCode::UppercaseG => Some("G"),
        KeyCode::UppercaseH => Some("H"),
        KeyCode::UppercaseI => Some("I"),
        KeyCode::UppercaseJ => Some("J"),
        KeyCode::UppercaseK => Some("K"),
        KeyCode::UppercaseL => Some("L"),
        KeyCode::UppercaseM => Some("M"),
        KeyCode::UppercaseN => Some("N"),
        KeyCode::UppercaseO => Some("O"),
        KeyCode::UppercaseP => Some("P"),
        KeyCode::UppercaseQ => Some("Q"),
        KeyCode::UppercaseR => Some("R"),
        KeyCode::UppercaseS => Some("S"),
        KeyCode::UppercaseT => Some("T"),
        KeyCode::UppercaseU => Some("U"),
        KeyCode::UppercaseV => Some("V"),
        KeyCode::UppercaseW => Some("W"),
        KeyCode::UppercaseX => Some("X"),
        KeyCode::UppercaseY => Some("Y"),
        KeyCode::UppercaseZ => Some("Z"),
        // Numbers
        KeyCode::Digit0 => Some("0"),
        KeyCode::Digit1 => Some("1"),
        KeyCode::Digit2 => Some("2"),
        KeyCode::Digit3 => Some("3"),
        KeyCode::Digit4 => Some("4"),
        KeyCode::Digit5 => Some("5"),
        KeyCode::Digit6 => Some("6"),
        KeyCode::Digit7 => Some("7"),
        KeyCode::Digit8 => Some("8"),
        KeyCode::Digit9 => Some("9"),
    }
}

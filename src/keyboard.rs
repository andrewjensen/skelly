use cosmic_text::{Attrs, Buffer, Color, FontSystem, Metrics, Shaping, SwashCache, Wrap};
use image::{Pixel, Rgba, RgbaImage};
use log::info;

pub enum KeyboardState {
    Normal,
    Shift,
}

pub struct PositionedKey {
    pub key: KeyCode,
    pub position: (u32, u32),
    pub size: (u32, u32),
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
    // Symbols
    Semicolon,
    Comma,
    Period,
    Slash,
    Space,
    Shift,
    Backspace,
    Return,
}

const KEYBOARD_MARGIN_Y: u32 = 40;
const KEY_UNIT_WIDTH: u32 = 100;
const KEY_UNIT_HEIGHT: u32 = 80;
const KEY_GUTTER: u32 = 10;
const KEYBOARD_COLUMNS: u32 = 12;
const KEYBOARD_ROWS: u32 = 5;

const COLOR_KEYBOARD_BACKGROUND: Rgba<u8> = Rgba([0xAA, 0xAA, 0xAA, 0xFF]);
const COLOR_KEY_BACKGROUND: Rgba<u8> = Rgba([0x00, 0x00, 0x00, 0xFF]);
const COLOR_KEY_FOREGROUND: Rgba<u8> = Rgba([0xFF, 0xFF, 0xFF, 0xFF]);

pub fn add_keyboard_overlay(
    screen: &mut RgbaImage,
    font_system: &mut FontSystem,
    cache: &mut SwashCache,
    keyboard_state: KeyboardState,
) {
    let keys = get_keys(&keyboard_state);

    let keyboard_keys_width =
        KEYBOARD_COLUMNS * KEY_UNIT_WIDTH + (KEYBOARD_COLUMNS - 1) * KEY_GUTTER;
    let keyboard_keys_height = KEYBOARD_ROWS * KEY_UNIT_HEIGHT + (KEYBOARD_ROWS - 1) * KEY_GUTTER;
    let keyboard_total_height = keyboard_keys_height + (KEYBOARD_MARGIN_Y * 2);

    let keyboard_offset_x = (screen.width() - keyboard_keys_width) / 2;
    // let keyboard_offset_x = 5;
    let keyboard_offset_y = screen.height() - keyboard_total_height;

    // Render the background
    for x in 0..screen.width() {
        for y in keyboard_offset_y..keyboard_total_height + keyboard_offset_y {
            screen.put_pixel(x, y, COLOR_KEYBOARD_BACKGROUND);
        }
    }

    let metrics = Metrics::new(40.0, 48.0);
    let attrs = Attrs::new().metrics(metrics);
    let text_color = Color::rgba(
        COLOR_KEY_FOREGROUND[0],
        COLOR_KEY_FOREGROUND[1],
        COLOR_KEY_FOREGROUND[2],
        COLOR_KEY_FOREGROUND[3],
    );

    let mut buffer = Buffer::new_empty(metrics);

    buffer.set_size(font_system, None, None);
    buffer.set_wrap(font_system, Wrap::None);

    // Render each key
    for positioned_key in keys.iter() {
        let key_grid_x = positioned_key.position.0;
        let key_grid_y = positioned_key.position.1;
        let key_grid_width = positioned_key.size.0;
        let key_grid_height = positioned_key.size.1;

        let top_left_x = keyboard_offset_x + (KEY_UNIT_WIDTH + KEY_GUTTER) * key_grid_x;
        let top_left_y =
            keyboard_offset_y + KEYBOARD_MARGIN_Y + (KEY_UNIT_HEIGHT + KEY_GUTTER) * key_grid_y;

        let key_width = KEY_UNIT_WIDTH * key_grid_width + KEY_GUTTER * (key_grid_width - 1);
        let key_height = KEY_UNIT_HEIGHT * key_grid_height + KEY_GUTTER * (key_grid_height - 1);

        // Render key background
        for key_x in top_left_x..top_left_x + key_width {
            for key_y in top_left_y..top_left_y + key_height {
                screen.put_pixel(key_x, key_y, COLOR_KEY_BACKGROUND);
            }
        }

        // Render key text
        buffer.lines.clear();
        let key_text = get_key_text(&positioned_key.key).unwrap();
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
                let canvas_x = buffer_x + (top_left_x as i32) + (key_width as i32 / 2)
                    - ((text_width.round() / 2.0) as i32);

                let canvas_y = buffer_y + (top_left_y as i32) + (key_height as i32 / 2)
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

fn get_keys(state: &KeyboardState) -> Vec<PositionedKey> {
    match state {
        KeyboardState::Normal => get_normal_keys(),
        KeyboardState::Shift => get_shift_keys(),
    }
}

fn get_normal_keys() -> Vec<PositionedKey> {
    vec![
        // Numbers
        PositionedKey {
            key: KeyCode::Digit1,
            position: (0, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit2,
            position: (1, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit3,
            position: (2, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit4,
            position: (3, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit5,
            position: (4, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit6,
            position: (5, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit7,
            position: (6, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit8,
            position: (7, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit9,
            position: (8, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Digit0,
            position: (9, 0),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Backspace,
            position: (10, 0),
            size: (2, 1),
        },
        // Letters, row 1
        PositionedKey {
            key: KeyCode::LowercaseQ,
            position: (0, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseW,
            position: (1, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseE,
            position: (2, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseR,
            position: (3, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseT,
            position: (4, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseY,
            position: (5, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseU,
            position: (6, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseI,
            position: (7, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseO,
            position: (8, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseP,
            position: (9, 1),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Return,
            position: (10, 1),
            size: (2, 4),
        },
        // Letters, row 2
        PositionedKey {
            key: KeyCode::LowercaseA,
            position: (0, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseS,
            position: (1, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseD,
            position: (2, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseF,
            position: (3, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseG,
            position: (4, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseH,
            position: (5, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseJ,
            position: (6, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseK,
            position: (7, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseL,
            position: (8, 2),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Semicolon,
            position: (9, 2),
            size: (1, 1),
        },
        // Letters, row 3
        PositionedKey {
            key: KeyCode::LowercaseZ,
            position: (0, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseX,
            position: (1, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseC,
            position: (2, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseV,
            position: (3, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseB,
            position: (4, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseN,
            position: (5, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::LowercaseM,
            position: (6, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Comma,
            position: (7, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Period,
            position: (8, 3),
            size: (1, 1),
        },
        PositionedKey {
            key: KeyCode::Slash,
            position: (9, 3),
            size: (1, 1),
        },
        // Symbol row
        PositionedKey {
            key: KeyCode::Shift,
            position: (0, 4),
            size: (2, 1),
        },
        PositionedKey {
            key: KeyCode::Space,
            position: (2, 4),
            size: (8, 1),
        },
    ]
}

fn get_shift_keys() -> Vec<PositionedKey> {
    // TODO
    vec![]
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
        // Symbols
        KeyCode::Semicolon => Some(";"),
        KeyCode::Comma => Some(","),
        KeyCode::Period => Some("."),
        KeyCode::Slash => Some("/"),
        KeyCode::Space => Some("space"),
        KeyCode::Shift => Some("shift"),
        KeyCode::Backspace => Some("delete"),
        KeyCode::Return => Some("return"),
    }
}

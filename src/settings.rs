use log::warn;
use serde::Deserialize;
use std::fs::File;
use std::io::Read;

#[derive(Debug, Deserialize, Clone)]
pub struct Settings {
    pub rendering: RenderingSettings,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RenderingSettings {
    pub font_size: u32,
    pub screen_margin_x: u32,
    pub line_height: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            rendering: RenderingSettings {
                font_size: 12,
                screen_margin_x: 100,
                line_height: 1.2,
            },
        }
    }
}

pub fn load_settings_with_fallback(file_path: &str) -> Settings {
    match load_settings(file_path) {
        Ok(settings) => settings,
        Err(_) => {
            warn!("Failed to load settings from file, using default settings");

            Settings::default()
        }
    }
}

fn load_settings(file_path: &str) -> Result<Settings, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path)?;

    let mut contents = vec![];
    file.read_to_end(&mut contents)?;
    let settings = serde_json::from_slice(&contents)?;

    Ok(settings)
}

use log::warn;
use serde::Deserialize;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

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

pub async fn load_settings_with_fallback(file_path: &str) -> Settings {
    match load_settings(file_path).await {
        Ok(settings) => settings,
        Err(_) => {
            warn!("Failed to load settings from file, using default settings");

            Settings::default()
        }
    }
}

async fn load_settings(file_path: &str) -> Result<Settings, Box<dyn std::error::Error>> {
    let mut file = File::open(file_path).await?;

    let mut contents = vec![];
    file.read_to_end(&mut contents).await?;
    let settings = serde_json::from_slice(&contents)?;

    Ok(settings)
}

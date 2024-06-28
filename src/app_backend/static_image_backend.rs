use image::RgbaImage;
use log::info;

use super::AppBackend;

pub struct StaticImageBackend {}

impl AppBackend for StaticImageBackend {
    fn render(&self, page_idx: usize, page_canvas: &RgbaImage) {
        info!("Rendering for StaticImageBackend...");

        let file_path = format!("./output/page-{}.png", page_idx);
        page_canvas.save(&file_path).expect("Failed to save image");
    }
}

use image::RgbaImage;
use log::info;

use super::AppBackend;

pub struct RemarkableBackend {}

impl AppBackend for RemarkableBackend {
    fn render(&self, page_idx: usize, page_canvas: &RgbaImage) {
        info!("Rendering for RemarkableBackend...");
    }
}

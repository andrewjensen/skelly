use image::RgbaImage;

#[cfg(target_os = "linux")]
mod remarkable_backend;

#[cfg(target_os = "linux")]
use remarkable_backend::RemarkableBackend;

#[cfg(target_os = "macos")]
mod static_image_backend;

#[cfg(target_os = "macos")]
use static_image_backend::StaticImageBackend;

pub trait AppBackend {
    fn render(&self, page_idx: usize, page_canvas: &RgbaImage);
}

pub fn get_app_backend() -> impl AppBackend {
    #[cfg(target_os = "linux")]
    let backend = RemarkableBackend {};

    #[cfg(target_os = "macos")]
    let backend = StaticImageBackend {};

    backend
}

use image::RgbaImage;

#[cfg(target = "armv7-unknown-linux-musleabihf")]
mod remarkable_backend;

#[cfg(target = "armv7-unknown-linux-musleabihf")]
use remarkable_backend::RemarkableBackend;

#[cfg(not(target = "armv7-unknown-linux-musleabihf"))]
mod static_image_backend;

#[cfg(not(target = "armv7-unknown-linux-musleabihf"))]
use static_image_backend::StaticImageBackend;

pub trait AppBackend {
    fn render(&self, page_idx: usize, page_canvas: &RgbaImage);
}

pub fn get_app_backend() -> impl AppBackend {
    #[cfg(target = "armv7-unknown-linux-musleabihf")]
    let backend = RemarkableBackend {};

    #[cfg(not(target = "armv7-unknown-linux-musleabihf"))]
    let backend = StaticImageBackend {};

    backend
}

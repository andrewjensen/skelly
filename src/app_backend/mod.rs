use image::RgbaImage;

#[cfg(feature = "remarkable")]
mod remarkable_backend;

#[cfg(feature = "remarkable")]
use remarkable_backend::RemarkableBackend;

#[cfg(feature = "static")]
mod static_image_backend;

#[cfg(feature = "static")]
use static_image_backend::StaticImageBackend;

pub trait AppBackend {
    fn render(&self, page_idx: usize, page_canvas: &RgbaImage);
}

pub fn get_app_backend() -> impl AppBackend {
    #[cfg(feature = "remarkable")]
    let backend = RemarkableBackend {};

    #[cfg(feature = "static")]
    let backend = StaticImageBackend {};

    backend
}

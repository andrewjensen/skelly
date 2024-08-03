use image::imageops::FilterType;
use image::{DynamicImage, RgbaImage};
use log::info;

pub fn rescale_image(raw_image: &RgbaImage, destination_width: u32) -> RgbaImage {
    info!(
        "Resizing image from input resolution {} x {} to destination width {}",
        raw_image.width(),
        raw_image.height(),
        destination_width,
    );

    let raw_image = DynamicImage::ImageRgba8(raw_image.clone());
    let result = raw_image.resize(destination_width, u32::MAX, FilterType::Triangle);

    result.into_rgba8()
}

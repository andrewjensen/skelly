use cgmath::Point2;
use image::RgbaImage;
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent, MultitouchEvent};
use log::{error, info};
use std::sync::mpsc::channel;
use tokio::task::spawn_blocking;

use crate::browser_core::BrowserCore;
use crate::CANVAS_WIDTH;

pub struct RemarkableApp {}

impl RemarkableApp {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        // TODO: rewrite to be async
        spawn_blocking(move || {
            let url = "https://andrewjensen.io/pen-plotter-art/";
            info!("The URL argument is: {}", url);
            let mut browser = BrowserCore::new();
            browser.navigate_to(&url);

            let page_canvas = browser.get_pages().first().unwrap();

            let mut framebuffer = Framebuffer::new();

            for (x, y, pixel) in page_canvas.enumerate_pixels() {
                let pixel_pos = Point2::<u32>::new(x, y);
                framebuffer.write_pixel(
                    pixel_pos.cast().unwrap(),
                    color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
                );
            }

            framebuffer.full_refresh(
                waveform_mode::WAVEFORM_MODE_INIT,
                display_temp::TEMP_USE_AMBIENT,
                dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
                0,
                true,
            );
        })
        .await?;

        Ok(())
    }
}

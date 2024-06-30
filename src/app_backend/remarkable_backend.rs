use cgmath::Point2;
use image::RgbaImage;
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent};
use log::{error, info};
use std::sync::mpsc::channel;

use super::AppBackend;

pub struct RemarkableBackend {}

impl AppBackend for RemarkableBackend {
    fn render(&self, _page_idx: usize, page_canvas: &RgbaImage) {
        info!("Rendering for RemarkableBackend...");

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

        info!("Starting event loop...");

        let (input_tx, input_rx) = channel::<InputEvent>();

        EvDevContext::new(InputDevice::Multitouch, input_tx.clone()).start();

        info!("Waiting for input events...");
        while let Ok(event) = input_rx.recv() {
            info!("{:?}", event);
        }
        info!("All event loops were closed?!?");
    }
}

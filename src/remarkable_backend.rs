use cgmath::Point2;
use image::{load_from_memory, RgbaImage};
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent, MultitouchEvent};
use log::{info, warn};
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::application::{UserInputEvent, OutputEvent};
use crate::backend::Backend;

pub struct RemarkableBackend {
    framebuffer: Framebuffer,
    user_input_tx: Sender<UserInputEvent>,
    output_rx: Receiver<OutputEvent>,
}

impl RemarkableBackend {
    pub fn new(user_input_tx: Sender<UserInputEvent>, output_rx: Receiver<OutputEvent>) -> Self {
        Self {
            framebuffer: Framebuffer::new(),
            user_input_tx,
            output_rx,
        }
    }

    fn render_page(&mut self, page_canvas: &RgbaImage) {
        for (x, y, pixel) in page_canvas.enumerate_pixels() {
            let pixel_pos = Point2::<u32>::new(x, y);
            self.framebuffer.write_pixel(
                pixel_pos.cast().unwrap(),
                color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
            );
        }
    }

    fn refresh_screen(&mut self) {
        self.framebuffer.full_refresh(
            waveform_mode::WAVEFORM_MODE_INIT,
            display_temp::TEMP_USE_AMBIENT,
            dither_mode::EPDC_FLAG_USE_DITHERING_PASSTHROUGH,
            0,
            true,
        );
    }
}

impl Backend for RemarkableBackend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let user_input_tx_clone = self.user_input_tx.clone();

        let event_loop_join = std::thread::spawn(move || {
            let (hardware_input_tx, hardware_input_rx) = channel::<InputEvent>();
            EvDevContext::new(InputDevice::Multitouch, hardware_input_tx.clone()).start();
            info!("Waiting for input events...");
            while let Ok(event) = hardware_input_rx.recv() {
                // info!("{:?}", event);

                if let InputEvent::MultitouchEvent {
                    event: multitouch_event,
                } = event
                {
                    if let MultitouchEvent::Press { finger } = multitouch_event {
                        user_input_tx_clone
                            .send(UserInputEvent::Tap {
                                x: finger.pos.x as u32,
                                y: finger.pos.y as u32,
                            })
                            .unwrap();
                    }
                }
            }
        });

        self.user_input_tx.send(UserInputEvent::RequestInitialPaint).unwrap();

        while let Ok(output_event) = self.output_rx.recv() {
            match output_event {
                OutputEvent::RenderFullScreen(image) => {
                    self.render_page(&image);
                    self.refresh_screen();
                }
            }
        }

        event_loop_join.join().unwrap();

        Ok(())
    }
}

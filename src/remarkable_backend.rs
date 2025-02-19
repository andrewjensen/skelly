use cgmath::Point2;
use image::{load_from_memory, RgbaImage};
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent, MultitouchEvent};
use log::{info, warn};
use std::sync::mpsc::{Receiver, channel};

use crate::application::{UserInputEvent, OutputEvent};
use crate::backend::Backend;
use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::Settings;
use crate::CANVAS_WIDTH;

#[derive(Debug)]
enum AppEvent {
    Initialize,
    // Navigate(NavigateCommand),
    // Render(RenderCommand),
    Tap { x: u32, y: u32 },
}

pub struct RemarkableBackend {
    browser: BrowserCore,
    framebuffer: Framebuffer,
    current_page_idx: usize,
}

impl RemarkableBackend {
    pub fn new(settings: Settings) -> Self {
        Self {
            browser: BrowserCore::new(settings),
            framebuffer: Framebuffer::new(),
            current_page_idx: 0,
        }
    }

    fn handle_tap_on_screen(&mut self, x: u32, y: u32) {
        match self.browser.state {
            BrowserState::ViewingPage {
                url: _,
                page_canvases: _,
            } => {
                if x < CANVAS_WIDTH / 3 {
                    info!("Tap: Previous page");

                    match self.browser.get_pages().get(self.current_page_idx - 1) {
                        Some(page_canvas) => {
                            self.current_page_idx -= 1;
                            self.render_page(&page_canvas.clone());
                            self.refresh_screen();
                        }
                        None => {
                            warn!("No previous page to display, ignoring tap");
                        }
                    }
                } else {
                    info!("Tap: Next page");
                    match self.browser.get_pages().get(self.current_page_idx + 1) {
                        Some(page_canvas) => {
                            self.current_page_idx += 1;
                            self.render_page(&page_canvas.clone());
                            self.refresh_screen();
                        }
                        None => {
                            warn!("No next page to display, ignoring tap");
                        }
                    }
                }
            }
            _ => {
                info!("Ignoring tap event, not in viewing state");
            }
        };
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
    fn get_input_event_receiver(&mut self) -> Receiver<UserInputEvent> {
        unreachable!()
    }

    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (app_tx, mut app_rx) = channel::<AppEvent>();
        let app_tx_shared = app_tx.clone();

        app_tx.send(AppEvent::Initialize).unwrap();

        // Input event loop
        let event_loop_join = std::thread::spawn(move || {
            let (input_tx, input_rx) = channel::<InputEvent>();

            EvDevContext::new(InputDevice::Multitouch, input_tx.clone()).start();

            info!("Waiting for input events...");
            while let Ok(event) = input_rx.recv() {
                // info!("{:?}", event);

                if let InputEvent::MultitouchEvent {
                    event: multitouch_event,
                } = event
                {
                    if let MultitouchEvent::Press { finger } = multitouch_event {
                        app_tx
                            .send(AppEvent::Tap {
                                x: finger.pos.x as u32,
                                y: finger.pos.y as u32,
                            })
                            .unwrap();
                    }
                }
            }
        });

        info!("Starting app event loop...");
        while let Ok(event) = app_rx.recv() {
            match event {
                AppEvent::Initialize => {
                    info!("Time to show the initial view!");

                    let placeholder_view = load_from_memory(include_bytes!(
                        "../assets/placeholder-initial-view.png"
                    ))
                    .unwrap();
                    let placeholder_view = placeholder_view.to_rgba8();
                    self.render_page(&placeholder_view);
                    self.refresh_screen();
                }
                // AppEvent::Navigate(command) => {
                //     info!("Received event: Navigate to {}", command.url);

                //     let placeholder_loading_view = load_from_memory(include_bytes!(
                //         "../assets/placeholder-loading-view.png"
                //     ))
                //     .unwrap();
                //     let placeholder_loading_view = placeholder_loading_view.to_rgba8();
                //     self.render_page(&placeholder_loading_view);
                //     self.refresh_screen();

                //     self.browser.navigate_to(&command.url).await;

                //     match &self.browser.state {
                //         BrowserState::ViewingPage { url, page_canvases } => {
                //             info!("Page loaded successfully");

                //             self.current_page_idx = 0;
                //             let page_canvas = page_canvases.get(0).unwrap().clone();
                //             self.render_page(&page_canvas);
                //             self.refresh_screen();
                //         }
                //         BrowserState::PageError { url, error } => {
                //             warn!("Failed to load the page, time to show the error view!");

                //             let placeholder_view = load_from_memory(include_bytes!(
                //                 "../assets/placeholder-error-view.png"
                //             ))
                //             .unwrap();
                //             let placeholder_view = placeholder_view.to_rgba8();
                //             self.render_page(&placeholder_view);
                //             self.refresh_screen();
                //         }
                //         _ => {
                //             unreachable!("Unexpected browser state after navigation");
                //         }
                //     }
                // }
                // AppEvent::Render(command) => {
                //     info!("Received event: Render HTML {}", command.html);

                //     let placeholder_loading_view = load_from_memory(include_bytes!(
                //         "../assets/placeholder-loading-view.png"
                //     ))
                //     .unwrap();
                //     let placeholder_loading_view = placeholder_loading_view.to_rgba8();
                //     self.render_page(&placeholder_loading_view);
                //     self.refresh_screen();

                //     self.browser.render(&command.html, &command.page_url).await;

                //     match &self.browser.state {
                //         BrowserState::ViewingPage { url, page_canvases } => {
                //             info!("Page loaded successfully");

                //             self.current_page_idx = 0;
                //             let page_canvas = page_canvases.get(0).unwrap().clone();
                //             self.render_page(&page_canvas);
                //             self.refresh_screen();
                //         }
                //         BrowserState::PageError { url, error } => {
                //             warn!("Failed to load the page, time to show the error view!");

                //             let placeholder_view = load_from_memory(include_bytes!(
                //                 "../assets/placeholder-error-view.png"
                //             ))
                //             .unwrap();
                //             let placeholder_view = placeholder_view.to_rgba8();
                //             self.render_page(&placeholder_view);
                //             self.refresh_screen();
                //         }
                //         _ => {
                //             unreachable!("Unexpected browser state after render");
                //         }
                //     }
                // }
                AppEvent::Tap { x, y } => {
                    self.handle_tap_on_screen(x, y);
                }
            }
        }

        event_loop_join.join().unwrap();

        Ok(())
    }
}

use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use cgmath::Point2;
use image::RgbaImage;
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent, MultitouchEvent};
use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::channel as std_channel;
use tokio::sync::mpsc::channel as tokio_channel;
use tokio::task::spawn_blocking;

use crate::browser_core::BrowserCore;
use crate::CANVAS_WIDTH;

#[derive(Debug)]
enum AppEvent {
    Navigate(NavigateCommand),
    PreviousPage,
    NextPage,
}

#[derive(Clone, Debug, Deserialize)]
struct NavigateCommand {
    pub url: String,
}

pub struct RemarkableApp {
    browser: BrowserCore,
    framebuffer: Framebuffer,
    current_page_idx: usize,
}

impl RemarkableApp {
    pub fn new() -> Self {
        Self {
            browser: BrowserCore::new(),
            framebuffer: Framebuffer::new(),
            current_page_idx: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let url = "https://andrewjensen.io/pen-plotter-art/";
        info!("The URL argument is: {}", url);

        self.browser.navigate_to(&url).await;
        self.current_page_idx = 0;

        let page_canvas = self.browser.get_pages().get(self.current_page_idx).unwrap();

        for (x, y, pixel) in page_canvas.enumerate_pixels() {
            let pixel_pos = Point2::<u32>::new(x, y);
            self.framebuffer.write_pixel(
                pixel_pos.cast().unwrap(),
                color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
            );
        }

        self.refresh_screen();

        let (app_tx, mut app_rx) = tokio_channel::<AppEvent>(32);
        let app_tx_web = app_tx.clone();

        // Input event loop
        let event_loop_join = spawn_blocking(move || {
            let (input_tx, input_rx) = std_channel::<InputEvent>();

            EvDevContext::new(InputDevice::Multitouch, input_tx.clone()).start();

            info!("Waiting for input events...");
            while let Ok(event) = input_rx.recv() {
                // info!("{:?}", event);

                if let InputEvent::MultitouchEvent {
                    event: multitouch_event,
                } = event
                {
                    if let MultitouchEvent::Press { finger } = multitouch_event {
                        let finger_x = finger.pos.x as u32;
                        if finger_x < (CANVAS_WIDTH / 3) {
                            // Close to the left edge
                            app_tx.blocking_send(AppEvent::PreviousPage).unwrap();
                        } else {
                            app_tx.blocking_send(AppEvent::NextPage).unwrap();
                        }
                    }
                }
            }
        });

        let web_server_join = tokio::spawn(async move {
            info!("Starting web server...");
            let web_server = Router::new()
                .route("/", get(|| async { "Hello from Skelly!" }))
                .route(
                    "/navigate",
                    post(|Json(payload): Json<NavigateCommand>| async move {
                        app_tx_web
                            .send(AppEvent::Navigate(payload.clone()))
                            .await
                            .unwrap();

                        StatusCode::OK
                    }),
                );
            let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
            axum::serve(listener, web_server).await.unwrap();
        });

        info!("Starting app event loop...");
        while let Some(event) = app_rx.recv().await {
            match event {
                AppEvent::Navigate(command) => {
                    info!("Received event: Navigate to {}", command.url);
                    self.browser.navigate_to(&command.url).await;
                    self.current_page_idx = 0;

                    let page_canvas = self.browser.get_pages().get(self.current_page_idx).unwrap();

                    for (x, y, pixel) in page_canvas.enumerate_pixels() {
                        let pixel_pos = Point2::<u32>::new(x, y);
                        self.framebuffer.write_pixel(
                            pixel_pos.cast().unwrap(),
                            color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
                        );
                    }

                    self.refresh_screen();
                }
                AppEvent::PreviousPage => {
                    info!("Received event: Previous page");
                    match self.browser.get_pages().get(self.current_page_idx - 1) {
                        Some(page_canvas) => {
                            self.current_page_idx -= 1;
                            for (x, y, pixel) in page_canvas.enumerate_pixels() {
                                let pixel_pos = Point2::<u32>::new(x, y);
                                self.framebuffer.write_pixel(
                                    pixel_pos.cast().unwrap(),
                                    color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
                                );
                            }
                            self.refresh_screen();
                        }
                        None => {
                            warn!("No more pages to display");
                        }
                    }
                }
                AppEvent::NextPage => {
                    info!("Received event: Next page");
                    match self.browser.get_pages().get(self.current_page_idx + 1) {
                        Some(page_canvas) => {
                            self.current_page_idx += 1;
                            for (x, y, pixel) in page_canvas.enumerate_pixels() {
                                let pixel_pos = Point2::<u32>::new(x, y);
                                self.framebuffer.write_pixel(
                                    pixel_pos.cast().unwrap(),
                                    color::RGB(pixel.0[0], pixel.0[1], pixel.0[2]),
                                );
                            }
                            self.refresh_screen();
                        }
                        None => {
                            warn!("No more pages to display");
                        }
                    }
                }
            }
        }

        event_loop_join.await?;
        web_server_join.await?;

        Ok(())
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

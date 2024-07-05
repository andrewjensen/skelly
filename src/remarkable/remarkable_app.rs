use axum::http::StatusCode;
use axum::routing::{get, post};
use axum::{Json, Router};
use cgmath::Point2;
use image::{load_from_memory, RgbaImage};
use libremarkable::framebuffer::common::*;
use libremarkable::framebuffer::core::Framebuffer;
use libremarkable::framebuffer::{FramebufferIO, FramebufferRefresh};
use libremarkable::input::{ev::EvDevContext, InputDevice, InputEvent, MultitouchEvent};
use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::channel as std_channel;
use tokio::sync::mpsc::channel as tokio_channel;
use tokio::task::spawn_blocking;

use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::Settings;
use crate::CANVAS_WIDTH;

#[derive(Debug)]
enum AppEvent {
    Initialize,
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
    pub fn new(settings: Settings) -> Self {
        Self {
            browser: BrowserCore::new(settings),
            framebuffer: Framebuffer::new(),
            current_page_idx: 0,
        }
    }

    pub async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let (app_tx, mut app_rx) = tokio_channel::<AppEvent>(32);
        let app_tx_web = app_tx.clone();

        app_tx.send(AppEvent::Initialize).await.unwrap();

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
                AppEvent::Initialize => {
                    info!("Time to show the initial view!");

                    let placeholder_view = load_from_memory(include_bytes!(
                        "../../assets/placeholder-initial-view.png"
                    ))
                    .unwrap();
                    let placeholder_view = placeholder_view.to_rgba8();
                    self.render_page(&placeholder_view);
                    self.refresh_screen();
                }
                AppEvent::Navigate(command) => {
                    info!("Received event: Navigate to {}", command.url);

                    let placeholder_loading_view = load_from_memory(include_bytes!(
                        "../../assets/placeholder-loading-view.png"
                    ))
                    .unwrap();
                    let placeholder_loading_view = placeholder_loading_view.to_rgba8();
                    self.render_page(&placeholder_loading_view);
                    self.refresh_screen();

                    self.browser.navigate_to(&command.url).await;

                    match &self.browser.state {
                        BrowserState::ViewingPage { url, page_canvases } => {
                            info!("Page loaded successfully");

                            self.current_page_idx = 0;
                            let page_canvas = page_canvases.get(0).unwrap().clone();
                            self.render_page(&page_canvas);
                            self.refresh_screen();
                        }
                        BrowserState::PageError { url, error } => {
                            warn!("Failed to load the page, time to show the error view!");

                            let placeholder_view = load_from_memory(include_bytes!(
                                "../../assets/placeholder-error-view.png"
                            ))
                            .unwrap();
                            let placeholder_view = placeholder_view.to_rgba8();
                            self.render_page(&placeholder_view);
                            self.refresh_screen();
                        }
                        _ => {
                            unreachable!("Unexpected browser state after navigation");
                        }
                    }
                }
                AppEvent::PreviousPage => {
                    info!("Received event: Previous page");

                    match self.browser.get_pages().get(self.current_page_idx - 1) {
                        Some(page_canvas) => {
                            self.current_page_idx -= 1;
                            self.render_page(&page_canvas.clone());
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
                            self.render_page(&page_canvas.clone());
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

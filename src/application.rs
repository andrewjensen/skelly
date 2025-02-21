#![allow(dead_code)]

use image::{load_from_memory, RgbaImage};
use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::{Receiver, Sender};

use crate::CANVAS_WIDTH;
use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::Settings;

#[derive(Debug)]
pub enum UserInputEvent {
    RequestInitialPaint,
    Tap { x: u32, y: u32 },
    RequestExit,
    PagePrevious,
    PageNext,
    Navigate(NavigateCommand),
    Render(RenderCommand),
}

#[derive(Clone, Debug, Deserialize)]
pub struct NavigateCommand {
    pub url: String,
}

#[derive(Debug)]
pub struct RenderCommand {
    pub html: String,
    // Needed for resolving relative image URLs
    pub page_url: String,
}

#[derive(Debug)]
pub enum OutputEvent {
    RenderFullScreen(RgbaImage),
}

#[allow(dead_code)]
pub struct Application {
    pub browser_core: BrowserCore,
    pub user_input_rx: Receiver<UserInputEvent>,
    pub output_tx: Sender<OutputEvent>,
    pub current_page_idx: usize,
}

impl Application {
    pub fn new(settings: Settings, user_input_rx: Receiver<UserInputEvent>, output_tx: Sender<OutputEvent>) -> Self {
        Self {
            browser_core: BrowserCore::new(settings),
            user_input_rx,
            output_tx,
            current_page_idx: 0,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Application running");

        while let Ok(input_event) = self.user_input_rx.recv() {
            match input_event {
                UserInputEvent::RequestInitialPaint => {
                    info!("Requesting initial paint");

                    let placeholder_view = load_from_memory(include_bytes!(
                        "../assets/placeholder-initial-view.png"
                    ));
                    let placeholder_view = placeholder_view.unwrap().to_rgba8();

                    self.output_tx.send(OutputEvent::RenderFullScreen(placeholder_view))?;
                }
                UserInputEvent::Tap { x, y } => {
                    info!("Tap event: {:?}", (x, y));

                    match self.browser_core.state {
                        BrowserState::ViewingPage {
                            url: _,
                            page_canvases: _,
                        } => {
                            if x < CANVAS_WIDTH / 3 {
                                info!("Tap: Previous page");

                                match self.browser_core.get_pages().get(self.current_page_idx - 1) {
                                    Some(page_canvas) => {
                                        self.current_page_idx -= 1;
                                        self.output_tx.send(OutputEvent::RenderFullScreen(page_canvas.clone()))?;
                                    }
                                    None => {
                                        warn!("No previous page to display, ignoring tap");
                                    }
                                }
                            } else {
                                info!("Tap: Next page");
                                match self.browser_core.get_pages().get(self.current_page_idx + 1) {
                                    Some(page_canvas) => {
                                        self.current_page_idx += 1;
                                        self.output_tx.send(OutputEvent::RenderFullScreen(page_canvas.clone()))?;
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
                UserInputEvent::RequestExit => {
                    info!("Requesting exit");
                    return Ok(());
                }
                UserInputEvent::Navigate(command) => {
                    info!("Received event: Navigate to {}", command.url);

                    let placeholder_view = load_from_memory(include_bytes!(
                        "../assets/placeholder-loading-view.png"
                    ));
                    let placeholder_view = placeholder_view.unwrap().to_rgba8();
                    self.output_tx.send(OutputEvent::RenderFullScreen(placeholder_view))?;

                    self.browser_core.navigate_to(&command.url);

                    match &self.browser_core.state {
                        BrowserState::ViewingPage { url, page_canvases } => {
                            info!("Page loaded successfully");

                            self.current_page_idx = 0;
                            let page_canvas = page_canvases.get(0).unwrap().clone();
                            self.output_tx.send(OutputEvent::RenderFullScreen(page_canvas))?;
                        }
                        BrowserState::PageError { url, error } => {
                            warn!("Failed to load the page, time to show the error view!");

                            let placeholder_view = load_from_memory(include_bytes!(
                                "../assets/placeholder-error-view.png"
                            ));
                            let placeholder_view = placeholder_view.unwrap().to_rgba8();
                            self.output_tx.send(OutputEvent::RenderFullScreen(placeholder_view))?;
                        }
                        _ => {
                            unreachable!("Unexpected browser state after navigation");
                        }
                    }
                }
                UserInputEvent::Render(command) => {
                    info!("Received event: Render HTML {}", command.html);

                    let placeholder_view = load_from_memory(include_bytes!(
                        "../assets/placeholder-loading-view.png"
                    ));
                    let placeholder_view = placeholder_view.unwrap().to_rgba8();
                    self.output_tx.send(OutputEvent::RenderFullScreen(placeholder_view))?;

                    self.browser_core.render(&command.html, &command.page_url);

                    match &self.browser_core.state {
                        BrowserState::ViewingPage { url, page_canvases } => {
                            info!("Page loaded successfully");

                            self.current_page_idx = 0;
                            let page_canvas = page_canvases.get(0).unwrap().clone();
                            self.output_tx.send(OutputEvent::RenderFullScreen(page_canvas))?;
                        }
                        BrowserState::PageError { url, error } => {
                            warn!("Failed to load the page, time to show the error view!");

                            let placeholder_view = load_from_memory(include_bytes!(
                                "../assets/placeholder-error-view.png"
                            ));
                            let placeholder_view = placeholder_view.unwrap().to_rgba8();
                            self.output_tx.send(OutputEvent::RenderFullScreen(placeholder_view))?;
                        }
                        _ => {
                            unreachable!("Unexpected browser state after render");
                        }
                    }
                }
                _ => {
                    warn!("Unhandled UserInputEvent: {:?}", input_event);
                }
            }
        }

        Ok(())
    }
}

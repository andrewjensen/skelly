#![allow(dead_code)]

use cosmic_text::{FontSystem, SwashCache};
use image::{load_from_memory, RgbaImage};
use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::{Receiver, Sender};

use crate::browser_core::{BrowserCore, BrowserState};
use crate::settings::Settings;
use crate::ui::keyboard::{add_keyboard_overlay, KeyboardState};
use crate::ui::topbar::{add_topbar_overlay, TopbarState};
use crate::CANVAS_WIDTH;

#[derive(Debug)]
pub enum UserInputEvent {
    RequestInitialPaint,
    Tap { x: u32, y: u32 },
    RequestExit,
    ViewPreviousPage,
    ViewNextPage,
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

    pub font_system: FontSystem,
    pub swash_cache: SwashCache,

    pub topbar_state: TopbarState,
    pub keyboard_state: KeyboardState,
}

impl Application {
    pub fn new(
        settings: Settings,
        user_input_rx: Receiver<UserInputEvent>,
        output_tx: Sender<OutputEvent>,
    ) -> Self {
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();

        Self {
            browser_core: BrowserCore::new(settings),
            user_input_rx,
            output_tx,
            current_page_idx: 0,
            font_system,
            swash_cache,
            topbar_state: TopbarState::Normal,
            keyboard_state: KeyboardState::Normal,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Application running");

        while let Ok(input_event) = self.user_input_rx.recv() {
            match input_event {
                UserInputEvent::RequestInitialPaint => {
                    info!("Requesting initial paint");

                    let placeholder_view =
                        load_from_memory(include_bytes!("../assets/placeholder-initial-view.png"))
                            .unwrap()
                            .to_rgba8();

                    self.render_screen(placeholder_view);
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
                                self.view_previous_page();
                            } else {
                                info!("Tap: Next page");
                                self.view_next_page();
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

                    let placeholder_view =
                        load_from_memory(include_bytes!("../assets/placeholder-loading-view.png"))
                            .unwrap()
                            .to_rgba8();

                    self.render_screen(placeholder_view);

                    self.browser_core.navigate_to(&command.url);

                    match &self.browser_core.state {
                        BrowserState::ViewingPage {
                            url: _,
                            page_canvases,
                        } => {
                            info!("Page loaded successfully");

                            self.current_page_idx = 0;
                            let page_canvas = page_canvases.get(0).unwrap().clone();
                            self.render_screen(page_canvas);
                        }
                        BrowserState::PageError { url: _, error: _ } => {
                            warn!("Failed to load the page, time to show the error view!");

                            let placeholder_view = load_from_memory(include_bytes!(
                                "../assets/placeholder-error-view.png"
                            ))
                            .unwrap()
                            .to_rgba8();

                            self.render_screen(placeholder_view);
                        }
                        _ => {
                            unreachable!("Unexpected browser state after navigation");
                        }
                    }
                }
                UserInputEvent::Render(command) => {
                    info!("Received event: Render HTML {}", command.html);

                    let placeholder_view =
                        load_from_memory(include_bytes!("../assets/placeholder-loading-view.png"))
                            .unwrap()
                            .to_rgba8();

                    self.render_screen(placeholder_view);

                    self.browser_core.render(&command.html, &command.page_url);

                    match &self.browser_core.state {
                        BrowserState::ViewingPage {
                            url: _,
                            page_canvases,
                        } => {
                            info!("Page loaded successfully");

                            self.current_page_idx = 0;
                            let page_canvas = page_canvases.get(0).unwrap().clone();
                            self.render_screen(page_canvas);
                        }
                        BrowserState::PageError { url: _, error: _ } => {
                            warn!("Failed to load the page, time to show the error view!");

                            let placeholder_view = load_from_memory(include_bytes!(
                                "../assets/placeholder-error-view.png"
                            ))
                            .unwrap()
                            .to_rgba8();

                            self.render_screen(placeholder_view);
                        }
                        _ => {
                            unreachable!("Unexpected browser state after render");
                        }
                    }
                }
                UserInputEvent::ViewPreviousPage => {
                    match self.browser_core.state {
                        BrowserState::ViewingPage { .. } => {
                            self.view_previous_page();
                        }
                        _ => {
                            info!("Ignoring ViewPreviousPage event, not in viewing state");
                        }
                    };
                }
                UserInputEvent::ViewNextPage => {
                    match self.browser_core.state {
                        BrowserState::ViewingPage { .. } => {
                            self.view_next_page();
                        }
                        _ => {
                            info!("Ignoring ViewNextPage event, not in viewing state");
                        }
                    };
                }
            }
        }

        Ok(())
    }

    fn view_next_page(&mut self) {
        match self.browser_core.get_pages().get(self.current_page_idx + 1) {
            Some(page_canvas) => {
                self.current_page_idx += 1;
                self.render_screen(page_canvas.clone());
            }
            None => {
                warn!("No next page to display, ignoring tap");
            }
        }
    }

    fn view_previous_page(&mut self) {
        if self.current_page_idx == 0 {
            warn!("No previous page to display, ignoring tap");
            return;
        }

        match self.browser_core.get_pages().get(self.current_page_idx - 1) {
            Some(page_canvas) => {
                self.current_page_idx -= 1;
                self.render_screen(page_canvas.clone());
            }
            None => {
                warn!("No previous page to display, ignoring tap");
            }
        }
    }

    fn render_screen(&mut self, page_canvas: RgbaImage) {
        let mut canvas_with_ui = page_canvas.clone();

        add_topbar_overlay(
            &mut canvas_with_ui,
            &mut self.font_system,
            &mut self.swash_cache,
            &self.topbar_state,
        );

        add_keyboard_overlay(
            &mut canvas_with_ui,
            &mut self.font_system,
            &mut self.swash_cache,
            &self.keyboard_state,
        );

        self.output_tx
            .send(OutputEvent::RenderFullScreen(canvas_with_ui))
            .unwrap();
    }
}

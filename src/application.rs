#![allow(dead_code)]

use image::{load_from_memory, RgbaImage};
use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::{Receiver, Sender};

use crate::browser_core::BrowserCore;
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

pub enum OutputEvent {
    // TODO: send the screen pixels to the backend
    RenderFullScreen(RgbaImage),
}

#[allow(dead_code)]
pub struct Application {
    pub browser_core: BrowserCore,
    pub user_input_rx: Receiver<UserInputEvent>,
    pub output_tx: Sender<OutputEvent>,
}

impl Application {
    pub fn new(settings: Settings, user_input_rx: Receiver<UserInputEvent>, output_tx: Sender<OutputEvent>) -> Self {
        Self {
            browser_core: BrowserCore::new(settings),
            user_input_rx,
            output_tx,
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
                }
                UserInputEvent::RequestExit => {
                    info!("Requesting exit");
                    return Ok(());
                }
                _ => {
                    warn!("Unhandled UserInputEvent: {:?}", input_event);
                }
            }
        }

        Ok(())
    }
}

#![allow(dead_code)]

use log::{info, warn};
use serde::Deserialize;
use std::sync::mpsc::Receiver;

use crate::browser_core::BrowserCore;
use crate::settings::Settings;

#[derive(Debug)]
pub enum UserInputEvent {
    Tap { x: u32, y: u32 },
    RequestExit,
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
}

#[allow(dead_code)]
pub struct Application {
    pub browser_core: BrowserCore,
    pub input_event_receiver: Receiver<UserInputEvent>,
    // pub output_event_sender: Sender<OutputEvent>,
    // pub output_event_receiver: Receiver<OutputEvent>,
}

impl Application {
    pub fn new(settings: Settings, user_input_rx: Receiver<UserInputEvent>) -> Self {
        // let (output_event_sender, output_event_receiver) = channel::<OutputEvent>(32);

        Self {
            browser_core: BrowserCore::new(settings),
            input_event_receiver: user_input_rx,
            // output_event_sender,
            // output_event_receiver,
        }
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Application running");

        while let Ok(input_event) = self.input_event_receiver.recv() {
            match input_event {
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

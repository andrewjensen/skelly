#![allow(dead_code)]

use log::info;
use std::sync::mpsc::{Receiver, channel};

use crate::browser_core::BrowserCore;
use crate::settings::Settings;
use crate::backend::Backend;

#[derive(Debug)]
pub enum UserInputEvent {
    Tap { x: u32, y: u32 },
    RequestExit,
}

pub enum OutputEvent {
    // TODO: send the screen pixels to the backend
}

#[allow(dead_code)]
pub struct Application {
    pub browser_core: BrowserCore,
    pub input_event_receiver: Option<Receiver<UserInputEvent>>,
    // pub output_event_sender: Sender<OutputEvent>,
    // pub output_event_receiver: Receiver<OutputEvent>,
}

impl Application {
    pub fn new(settings: Settings) -> Self {
        // let (output_event_sender, output_event_receiver) = channel::<OutputEvent>(32);

        Self {
            browser_core: BrowserCore::new(settings),
            input_event_receiver: None,
            // output_event_sender,
            // output_event_receiver,
        }
    }

    pub fn connect_to_backend(&mut self, backend: &mut impl Backend) {
        self.input_event_receiver = Some(backend.get_input_event_receiver());
    }

    pub fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Application running");

        let receiver = self.input_event_receiver.as_mut().unwrap();

        while let Ok(input_event) = receiver.recv() {
            info!("Input event: {:?}", input_event);

            match input_event {
                UserInputEvent::Tap { x, y } => {
                    info!("Tap event: {:?}", (x, y));
                }
                UserInputEvent::RequestExit => {
                    info!("Requesting exit");
                    break;
                }
            }
        }

        Ok(())
    }
}

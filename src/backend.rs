use std::sync::mpsc::{Receiver, channel};

use crate::application::{UserInputEvent, OutputEvent};

pub trait Backend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

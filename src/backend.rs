use tokio::sync::mpsc::{Sender, Receiver};

use crate::application::{UserInputEvent, OutputEvent};

pub trait Backend {
    fn get_input_event_receiver(&mut self) -> Receiver<UserInputEvent>;
    // fn set_output_event_receiver(&mut self, channel: Receiver<OutputEvent>);

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>>;
}

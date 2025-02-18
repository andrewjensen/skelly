use image::Rgb;
use log::info;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, MouseButton, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::keyboard::{Key, NamedKey};
use winit::raw_window_handle::{RawDisplayHandle, RawWindowHandle};
use winit::window::{Window, WindowId};
use tokio::sync::mpsc::{channel, Sender, Receiver};

use crate::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::application::{UserInputEvent, OutputEvent};
use crate::backend::Backend;
use crate::settings::Settings;

pub fn create_desktop_backend(settings: Settings) -> DesktopBackend {
    info!("Time to create a window");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let (input_event_sender, input_event_receiver) = channel::<UserInputEvent>(32);

    let mut desktop_app = DesktopBackend {
        window: None,
        mouse_position: (0.0, 0.0),
        input_event_sender: input_event_sender,
        input_event_receiver_temp: Some(input_event_receiver),
        // output_event_receiver: None,
    };

    let _result = event_loop.run_app(&mut desktop_app);

    desktop_app
}

pub struct DesktopBackend {
    window: Option<Window>,
    mouse_position: (f64, f64),
    input_event_sender: Sender<UserInputEvent>,
    input_event_receiver_temp: Option<Receiver<UserInputEvent>>,
    // output_event_receiver: Option<Receiver<OutputEvent>>,
}

impl Backend for DesktopBackend {
    fn get_input_event_receiver(&mut self) -> Receiver<UserInputEvent> {
        let receiver = self.input_event_receiver_temp.take().unwrap();
        self.input_event_receiver_temp = None;

        receiver
    }

    // fn set_output_event_receiver(&mut self, receiver: Receiver<OutputEvent>) {
    //     self.output_event_receiver = Some(receiver);
    // }

    async fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Desktop backend running");

        Ok(())
    }
}

impl ApplicationHandler for DesktopBackend {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        info!("winit window resumed");

        let window_attributes = Window::default_attributes()
            .with_title("Skelly")
            .with_inner_size(LogicalSize::new(CANVAS_WIDTH / 2, CANVAS_HEIGHT / 2))
            .with_resizable(false);

        let window = event_loop.create_window(window_attributes).unwrap();

        // Store the window first before borrowing it
        self.window = Some(window);

        // Now get a reference to the window
        let window = self.window.as_ref().unwrap();
        let window_rc = Rc::new(window);

        let context = softbuffer::Context::new(window_rc.clone()).unwrap();
        let mut surface = softbuffer::Surface::new(&context, window_rc.clone()).unwrap();

        surface
            .resize(
                NonZeroU32::new(CANVAS_WIDTH).unwrap(),
                NonZeroU32::new(CANVAS_HEIGHT).unwrap(),
            )
            .unwrap();

        let mut buffer = surface.buffer_mut().unwrap();
        let bg = Rgb([255, 255, 255]);

        // Convert to the format softbuffer expects
        let bg_u32 = bg.0[2] as u32 | (bg.0[1] as u32) << 8 | (bg.0[0] as u32) << 16;
        buffer.fill(bg_u32);

        buffer.present().unwrap();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                // Redraw the application.
                //
                // It's preferable for applications that do not render continuously to render in
                // this event rather than in AboutToWait, since rendering in here allows
                // the program to gracefully handle redraws requested by the OS.

                // Draw.

                // Queue a RedrawRequested event.
                //
                // You only need to call this if you've determined that you need to redraw in
                // applications which do not always need to. Applications that redraw continuously
                // can render here instead.
                self.window.as_ref().unwrap().request_redraw();
            }
            WindowEvent::KeyboardInput {
                event,
                device_id: _,
                is_synthetic: _,
            } => {
                // info!("Keyboard input: {:?}", event);

                if event.state == ElementState::Released || event.repeat {
                    // We only care about the initial press
                    return;
                }

                match event.logical_key {
                    Key::Named(NamedKey::ArrowLeft) => {
                        info!("Left arrow key pressed");
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        info!("Right arrow key pressed");
                    }
                    Key::Named(NamedKey::Escape) => {
                        info!("Escape key pressed");
                        event_loop.exit();
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position: position,
            } => {
                self.mouse_position = position.into();
            }
            WindowEvent::MouseInput {
                device_id: _,
                state: state,
                button: button,
            } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let cursor_x: u32 = self.mouse_position.0 as u32;
                    let cursor_y: u32 = self.mouse_position.1 as u32;
                    info!(
                        "Left mouse button pressed at ({}, {})",
                        cursor_x, cursor_y
                    );
                }
            }
            _ => (),
        }
    }
}

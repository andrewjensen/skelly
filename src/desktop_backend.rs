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
use std::sync::mpsc::{channel, Sender, Receiver};

use crate::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::application::{UserInputEvent, OutputEvent};
use crate::backend::Backend;
use crate::settings::Settings;

pub struct DesktopBackend {
    window: Option<Window>,
    mouse_position: (f64, f64),
    input_event_sender: Sender<UserInputEvent>,
    event_loop: Option<EventLoop<()>>,
}

impl DesktopBackend {
    pub fn new(input_event_sender: Sender<UserInputEvent>) -> Self {
        let event_loop = EventLoop::new().unwrap();

        Self {
            window: None,
            mouse_position: (0.0, 0.0),
            input_event_sender,
            event_loop: Some(event_loop),
        }
    }
}

impl Backend for DesktopBackend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Desktop backend running");

        let event_loop = self.event_loop.take().unwrap();
        event_loop.set_control_flow(ControlFlow::Wait);

        let _result = event_loop.run_app(self);

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
                        self.input_event_sender.send(UserInputEvent::PagePrevious).unwrap();
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        info!("Right arrow key pressed");
                        self.input_event_sender.send(UserInputEvent::PageNext).unwrap();
                    }
                    Key::Named(NamedKey::Escape) => {
                        info!("Escape key pressed");
                        event_loop.exit();

                        self.input_event_sender.send(UserInputEvent::RequestExit).unwrap();
                    }
                    _ => {}
                }
            }
            WindowEvent::CursorMoved {
                device_id: _,
                position,
            } => {
                self.mouse_position = position.into();
            }
            WindowEvent::MouseInput {
                device_id: _,
                state,
                button,
            } => {
                if state == ElementState::Pressed && button == MouseButton::Left {
                    let cursor_x: u32 = self.mouse_position.0 as u32;
                    let cursor_y: u32 = self.mouse_position.1 as u32;
                    info!(
                        "Left mouse button pressed at ({}, {})",
                        cursor_x, cursor_y
                    );

                    self.input_event_sender
                        .send(UserInputEvent::Tap {
                            x: cursor_x,
                            y: cursor_y,
                        })
                        .unwrap();
                }
            }
            _ => (),
        }
    }
}

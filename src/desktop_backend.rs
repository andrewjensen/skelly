use image::Rgb;
use log::{info, warn, error};
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
use std::sync::mpsc::{channel, Sender, Receiver, TryRecvError};

use crate::{CANVAS_HEIGHT, CANVAS_WIDTH};
use crate::application::{UserInputEvent, OutputEvent};
use crate::backend::Backend;
use crate::settings::Settings;

pub struct DesktopBackend {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    mouse_position: (f64, f64),
    user_input_tx: Sender<UserInputEvent>,
    output_rx: Receiver<OutputEvent>,
    event_loop: Option<EventLoop<()>>,
}

impl DesktopBackend {
    pub fn new(user_input_tx: Sender<UserInputEvent>, output_rx: Receiver<OutputEvent>) -> Self {
        let event_loop = EventLoop::new().unwrap();

        Self {
            window: None,
            surface: None,
            mouse_position: (0.0, 0.0),
            user_input_tx,
            output_rx,
            event_loop: Some(event_loop),
        }
    }
}

impl Backend for DesktopBackend {
    fn run(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        info!("Desktop backend running");

        let event_loop = self.event_loop.take().unwrap();
        event_loop.set_control_flow(ControlFlow::Poll);

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
        let window_rc = Rc::new(window);

        self.window = Some(window_rc.clone());

        let context = softbuffer::Context::new(window_rc.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window_rc).unwrap();
        self.surface = Some(surface);

        self.surface.as_mut().unwrap()
            .resize(
                NonZeroU32::new(CANVAS_WIDTH).unwrap(),
                NonZeroU32::new(CANVAS_HEIGHT).unwrap(),
            )
            .unwrap();

        let mut buffer = self.surface.as_mut().unwrap().buffer_mut().unwrap();
        let bg = Rgb([255, 255, 255]);

        // Convert to the format softbuffer expects
        let bg_u32 = bg.0[2] as u32 | (bg.0[1] as u32) << 8 | (bg.0[0] as u32) << 16;
        buffer.fill(bg_u32);

        buffer.present().unwrap();

        self.user_input_tx.send(UserInputEvent::RequestInitialPaint).unwrap();
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => {
                info!("The close button was pressed; stopping");
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                match self.output_rx.try_recv() {
                    Ok(output_event) => {
                        match output_event {
                            OutputEvent::RenderFullScreen(image) => {
                                info!("Received output event: RenderFullScreen");

                                let mut buffer = self.surface.as_mut().expect("Surface not initialized").buffer_mut().unwrap();

                                let image_width = image.width() as usize;

                                for (x, y, pixel) in image.enumerate_pixels() {
                                    let red = pixel.0[0] as u32;
                                    let green = pixel.0[1] as u32;
                                    let blue = pixel.0[2] as u32;

                                    let color = blue | (green << 8) | (red << 16);
                                    buffer[y as usize * image_width + x as usize] = color;
                                }

                                buffer.present().unwrap();
                            }
                        }
                    }
                    Err(TryRecvError::Empty) => {
                        // No output event to receive, so we don't need to redraw
                    }
                    Err(TryRecvError::Disconnected) => {
                        info!("Output channel disconnected");
                        event_loop.exit();
                    }
                }

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
                        self.user_input_tx.send(UserInputEvent::ViewPreviousPage).unwrap();
                    }
                    Key::Named(NamedKey::ArrowRight) => {
                        info!("Right arrow key pressed");
                        self.user_input_tx.send(UserInputEvent::ViewNextPage).unwrap();
                    }
                    Key::Named(NamedKey::Escape) => {
                        info!("Escape key pressed");
                        event_loop.exit();

                        self.user_input_tx.send(UserInputEvent::RequestExit).unwrap();
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

                    self.user_input_tx
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

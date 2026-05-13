//! Windows platform backend (winit-based).

use crate::backend::{HostBackend, HostError};
use crate::host_event::{HostEvent, WindowId};
use crate::platform::Platform;
use crate::window::WindowConfig;

/// Windows platform implementation.
pub struct WindowsPlatform {
    next_window_id: u64,
    event_queue: Vec<HostEvent>,
    exit_requested: bool,
}

impl WindowsPlatform {
    /// Create a new Windows platform backend.
    pub fn new() -> Self {
        Self {
            next_window_id: 1,
            event_queue: Vec::new(),
            exit_requested: false,
        }
    }
}

impl HostBackend for WindowsPlatform {
    fn init(&mut self) -> Result<(), HostError> {
        Ok(())
    }

    fn create_window(&mut self, _config: WindowConfig) -> Result<WindowId, HostError> {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        Ok(id)
    }

    fn poll_events(&mut self) -> Vec<HostEvent> {
        std::mem::take(&mut self.event_queue)
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn shutdown(&mut self) {}

    fn set_cursor_style(&mut self, _style: crate::backend::CursorStyle) {}
}

impl Platform for WindowsPlatform {
    fn push_event(&mut self, event: HostEvent) {
        self.event_queue.push(event);
    }

    fn run(&mut self, event_handler: &mut dyn FnMut(HostEvent)) -> Result<(), HostError> {
        use winit::application::ApplicationHandler;
        use winit::event::WindowEvent as WinitWindowEvent;
        use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
        use winit::window::WindowId as WinitWindowId;

        struct App<'a> {
            window: Option<winit::window::Window>,
            handler: &'a mut dyn FnMut(HostEvent),
            exit_requested: bool,
            core_window_id: crate::host_event::WindowId,
            modifiers: winit::keyboard::ModifiersState,
        }

        impl<'a> ApplicationHandler for App<'a> {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window.is_none() {
                    let attrs = winit::window::WindowAttributes::default()
                        .with_title("CORE OS Demo")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280, 720));
                    match event_loop.create_window(attrs) {
                        Ok(w) => self.window = Some(w),
                        Err(e) => {
                            tracing::error!("Failed to create window: {}", e);
                            event_loop.exit();
                        }
                    }
                }
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                _window_id: WinitWindowId,
                event: WinitWindowEvent,
            ) {
                if self.exit_requested {
                    event_loop.exit();
                    return;
                }

                match event {
                    WinitWindowEvent::CloseRequested => {
                        (self.handler)(HostEvent::Close {
                            window: self.core_window_id,
                        });
                        event_loop.exit();
                    }
                    WinitWindowEvent::Resized(size) => {
                        (self.handler)(HostEvent::Resize {
                            window: self.core_window_id,
                            width: size.width,
                            height: size.height,
                        });
                    }
                    WinitWindowEvent::CursorMoved { position, .. } => {
                        (self.handler)(HostEvent::Input(crate::events::InputEvent::MouseMove {
                            x: position.x,
                            y: position.y,
                        }));
                    }
                    WinitWindowEvent::MouseInput { button, state, .. } => {
                        let btn = match button {
                            winit::event::MouseButton::Left => crate::events::MouseButton::Left,
                            winit::event::MouseButton::Right => crate::events::MouseButton::Right,
                            winit::event::MouseButton::Middle => crate::events::MouseButton::Middle,
                            _ => crate::events::MouseButton::X1,
                        };
                        let st = match state {
                            winit::event::ElementState::Pressed => crate::events::KeyState::Pressed,
                            winit::event::ElementState::Released => {
                                crate::events::KeyState::Released
                            }
                        };
                        (self.handler)(HostEvent::Input(crate::events::InputEvent::MouseButton {
                            button: btn,
                            state: st,
                        }));
                    }
                    WinitWindowEvent::ModifiersChanged(mods) => {
                        self.modifiers = mods.state();
                    }
                    WinitWindowEvent::KeyboardInput { event, .. } => {
                        let key = match event.logical_key {
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Escape) => {
                                crate::events::KeyCode::Escape
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Enter) => {
                                crate::events::KeyCode::Enter
                            }
                            winit::keyboard::Key::Named(winit::keyboard::NamedKey::Space) => {
                                crate::events::KeyCode::Space
                            }
                            winit::keyboard::Key::Character(ref s) if s.len() == 1 => {
                                let c = s.chars().next().unwrap();
                                match c {
                                    'a'..='z' => match c {
                                        'a' => crate::events::KeyCode::A,
                                        'b' => crate::events::KeyCode::B,
                                        'c' => crate::events::KeyCode::C,
                                        'd' => crate::events::KeyCode::D,
                                        'e' => crate::events::KeyCode::E,
                                        'f' => crate::events::KeyCode::F,
                                        'g' => crate::events::KeyCode::G,
                                        'h' => crate::events::KeyCode::H,
                                        'i' => crate::events::KeyCode::I,
                                        'j' => crate::events::KeyCode::J,
                                        'k' => crate::events::KeyCode::K,
                                        'l' => crate::events::KeyCode::L,
                                        'm' => crate::events::KeyCode::M,
                                        'n' => crate::events::KeyCode::N,
                                        'o' => crate::events::KeyCode::O,
                                        'p' => crate::events::KeyCode::P,
                                        'q' => crate::events::KeyCode::Q,
                                        'r' => crate::events::KeyCode::R,
                                        's' => crate::events::KeyCode::S,
                                        't' => crate::events::KeyCode::T,
                                        'u' => crate::events::KeyCode::U,
                                        'v' => crate::events::KeyCode::V,
                                        'w' => crate::events::KeyCode::W,
                                        'x' => crate::events::KeyCode::X,
                                        'y' => crate::events::KeyCode::Y,
                                        'z' => crate::events::KeyCode::Z,
                                        _ => crate::events::KeyCode::Unmapped(0),
                                    },
                                    '0'..='9' => match c {
                                        '0' => crate::events::KeyCode::Digit0,
                                        '1' => crate::events::KeyCode::Digit1,
                                        '2' => crate::events::KeyCode::Digit2,
                                        '3' => crate::events::KeyCode::Digit3,
                                        '4' => crate::events::KeyCode::Digit4,
                                        '5' => crate::events::KeyCode::Digit5,
                                        '6' => crate::events::KeyCode::Digit6,
                                        '7' => crate::events::KeyCode::Digit7,
                                        '8' => crate::events::KeyCode::Digit8,
                                        '9' => crate::events::KeyCode::Digit9,
                                        _ => unreachable!(),
                                    },
                                    _ => crate::events::KeyCode::Unmapped(0),
                                }
                            }
                            _ => crate::events::KeyCode::Unmapped(0),
                        };
                        let st = match event.state {
                            winit::event::ElementState::Pressed => crate::events::KeyState::Pressed,
                            winit::event::ElementState::Released => {
                                crate::events::KeyState::Released
                            }
                        };

                        // Panic gesture: Ctrl+Shift+Escape
                        if key == crate::events::KeyCode::Escape
                            && event.state == winit::event::ElementState::Pressed
                            && self.modifiers.control_key()
                            && self.modifiers.shift_key()
                        {
                            (self.handler)(HostEvent::PanicExit);
                            event_loop.exit();
                            return;
                        }

                        (self.handler)(HostEvent::Input(crate::events::InputEvent::Keyboard {
                            key,
                            state: st,
                            scancode: 0,
                        }));
                    }
                    WinitWindowEvent::RedrawRequested => {
                        if let Some(ref window) = self.window {
                            window.request_redraw();
                        }
                    }
                    _ => {}
                }
            }

            fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
                if self.exit_requested {
                    event_loop.exit();
                } else if let Some(ref window) = self.window {
                    window.request_redraw();
                }
            }
        }

        let event_loop =
            EventLoop::new().map_err(|e| HostError::WindowCreationFailed(e.to_string()))?;
        event_loop.set_control_flow(ControlFlow::Poll);

        let mut app = App {
            window: None,
            handler: event_handler,
            exit_requested: false,
            core_window_id: self
                .create_window(WindowConfig::default())
                .unwrap_or(WindowId(0)),
            modifiers: winit::keyboard::ModifiersState::empty(),
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| HostError::WindowCreationFailed(e.to_string()))
    }
}

//! Windows platform backend (winit-based).

use crate::backend::{CursorStyle, HostBackend, HostError};
use crate::events::{InputEvent, KeyCode, KeyState, MouseButton};
use crate::host_event::{HostEvent, WindowId};
use crate::platform::Platform;
use crate::window::WindowConfig;

pub struct WindowsPlatform {
    next_window_id: u64,
    event_queue: Vec<HostEvent>,
    exit_requested: bool,
    pending_config: Option<WindowConfig>,
    cursor_style: CursorStyle,
}

impl WindowsPlatform {
    pub fn new() -> Self {
        Self {
            next_window_id: 1,
            event_queue: Vec::new(),
            exit_requested: false,
            pending_config: None,
            cursor_style: CursorStyle::Default,
        }
    }
}

impl HostBackend for WindowsPlatform {
    fn init(&mut self) -> Result<(), HostError> {
        Ok(())
    }

    fn create_window(&mut self, config: WindowConfig) -> Result<WindowId, HostError> {
        let id = WindowId(self.next_window_id);
        self.next_window_id += 1;
        self.pending_config = Some(config);
        Ok(id)
    }

    fn poll_events(&mut self) -> Vec<HostEvent> {
        std::mem::take(&mut self.event_queue)
    }

    fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    fn shutdown(&mut self) {}

    fn set_cursor_style(&mut self, style: CursorStyle) {
        self.cursor_style = style;
    }
}

impl Platform for WindowsPlatform {
    fn push_event(&mut self, event: HostEvent) {
        self.event_queue.push(event);
    }

    fn run(&mut self, event_handler: &mut dyn FnMut(HostEvent)) -> Result<(), HostError> {
        use winit::application::ApplicationHandler;
        use winit::event::WindowEvent as WEvent;
        use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

        let config = self.pending_config.take().unwrap_or_default();
        let initial_cursor_style = self.cursor_style;

        struct App<'a> {
            window: Option<winit::window::Window>,
            handler: &'a mut dyn FnMut(HostEvent),
            exit_requested: bool,
            core_window_id: WindowId,
            modifiers: winit::keyboard::ModifiersState,
            config: WindowConfig,
            cursor_style: CursorStyle,
        }

        impl<'a> ApplicationHandler for App<'a> {
            fn resumed(&mut self, event_loop: &ActiveEventLoop) {
                if self.window.is_none() {
                    let mut attrs = winit::window::WindowAttributes::default()
                        .with_title(&self.config.title)
                        .with_inner_size(winit::dpi::LogicalSize::new(
                            self.config.width,
                            self.config.height,
                        ));

                    if self.config.fullscreen {
                        attrs = attrs
                            .with_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                    }

                    match event_loop.create_window(attrs) {
                        Ok(w) => {
                            apply_cursor(&w, self.cursor_style);
                            self.window = Some(w);
                        }
                        Err(e) => {
                            tracing::error!("failed to create window: {}", e);
                            event_loop.exit();
                        }
                    }
                }
            }

            fn window_event(
                &mut self,
                event_loop: &ActiveEventLoop,
                _window_id: winit::window::WindowId,
                event: WEvent,
            ) {
                if self.exit_requested {
                    event_loop.exit();
                    return;
                }

                match event {
                    WEvent::CloseRequested => {
                        (self.handler)(HostEvent::Close {
                            window: self.core_window_id,
                        });
                        event_loop.exit();
                    }
                    WEvent::Resized(size) => {
                        (self.handler)(HostEvent::Resize {
                            window: self.core_window_id,
                            width: size.width,
                            height: size.height,
                        });
                    }
                    WEvent::Moved(pos) => {
                        (self.handler)(HostEvent::Move {
                            window: self.core_window_id,
                            x: pos.x,
                            y: pos.y,
                        });
                    }
                    WEvent::Focused(focused) => {
                        (self.handler)(HostEvent::Focus {
                            window: self.core_window_id,
                            focused,
                        });
                    }
                    WEvent::Destroyed => {
                        event_loop.exit();
                    }
                    WEvent::Occluded(_hidden) => {}
                    WEvent::CursorMoved { position, .. } => {
                        (self.handler)(HostEvent::Input(InputEvent::MouseMove {
                            x: position.x,
                            y: position.y,
                        }));
                    }
                    WEvent::MouseInput { button, state, .. } => {
                        let btn = map_mouse_button(button);
                        let st = map_key_state(state);
                        (self.handler)(HostEvent::Input(InputEvent::MouseButton {
                            button: btn,
                            state: st,
                        }));
                    }
                    WEvent::MouseWheel { delta, .. } => {
                        let (dx, dy) = match delta {
                            winit::event::MouseScrollDelta::LineDelta(x, y) => (x, y),
                            winit::event::MouseScrollDelta::PixelDelta(p) => {
                                (p.x as f32 / 20.0, p.y as f32 / 20.0)
                            }
                        };
                        (self.handler)(HostEvent::Input(InputEvent::MouseScroll {
                            delta_x: dx,
                            delta_y: dy,
                        }));
                    }
                    WEvent::ModifiersChanged(mods) => {
                        self.modifiers = mods.state();
                    }
                    WEvent::KeyboardInput { event, .. } => {
                        let key = map_keycode(&event.logical_key);
                        let st = map_key_state(event.state);
                        let scancode = match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(code) => winit_scancode_to_raw(code),
                            winit::keyboard::PhysicalKey::Unidentified(_) => 0,
                        };

                        if key == KeyCode::Escape
                            && event.state == winit::event::ElementState::Pressed
                            && self.modifiers.control_key()
                            && self.modifiers.shift_key()
                        {
                            (self.handler)(HostEvent::PanicExit);
                            event_loop.exit();
                            return;
                        }

                        (self.handler)(HostEvent::Input(InputEvent::Keyboard {
                            key,
                            state: st,
                            scancode,
                        }));
                    }
                    WEvent::Touch(touch) => {
                        use winit::event::TouchPhase;
                        let phase = match touch.phase {
                            TouchPhase::Started => crate::events::TouchPhase::Started,
                            TouchPhase::Moved => crate::events::TouchPhase::Moved,
                            TouchPhase::Ended => crate::events::TouchPhase::Ended,
                            TouchPhase::Cancelled => crate::events::TouchPhase::Cancelled,
                        };
                        (self.handler)(HostEvent::Input(InputEvent::Touch {
                            id: touch.id,
                            phase,
                            x: touch.location.x,
                            y: touch.location.y,
                        }));
                    }
                    WEvent::Ime(_) => {}
                    WEvent::RedrawRequested => {
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

        let core_window_id = WindowId(1);
        let mut app = App {
            window: None,
            handler: event_handler,
            exit_requested: false,
            core_window_id,
            modifiers: winit::keyboard::ModifiersState::empty(),
            config,
            cursor_style: initial_cursor_style,
        };

        event_loop
            .run_app(&mut app)
            .map_err(|e| HostError::WindowCreationFailed(e.to_string()))
    }
}

fn map_key_state(state: winit::event::ElementState) -> KeyState {
    match state {
        winit::event::ElementState::Pressed => KeyState::Pressed,
        winit::event::ElementState::Released => KeyState::Released,
    }
}

fn map_mouse_button(button: winit::event::MouseButton) -> MouseButton {
    match button {
        winit::event::MouseButton::Left => MouseButton::Left,
        winit::event::MouseButton::Right => MouseButton::Right,
        winit::event::MouseButton::Middle => MouseButton::Middle,
        winit::event::MouseButton::Back => MouseButton::X1,
        winit::event::MouseButton::Forward => MouseButton::X2,
        _ => MouseButton::X1,
    }
}

fn map_keycode(key: &winit::keyboard::Key) -> KeyCode {
    use winit::keyboard::Key;

    match key {
        Key::Named(n) => map_named_key(n),
        Key::Character(s) => map_char_key(s),
        Key::Unidentified(_) | Key::Dead(_) => KeyCode::Unmapped(0),
    }
}

fn map_named_key(key: &winit::keyboard::NamedKey) -> KeyCode {
    use winit::keyboard::NamedKey as N;
    match key {
        N::Escape => KeyCode::Escape,
        N::Enter => KeyCode::Enter,
        N::Space => KeyCode::Space,
        N::Tab => KeyCode::Tab,
        N::Backspace => KeyCode::Backspace,
        N::Delete => KeyCode::Delete,
        N::Insert => KeyCode::Insert,
        N::Home => KeyCode::Home,
        N::End => KeyCode::End,
        N::PageUp => KeyCode::PageUp,
        N::PageDown => KeyCode::PageDown,
        N::ArrowLeft => KeyCode::ArrowLeft,
        N::ArrowRight => KeyCode::ArrowRight,
        N::ArrowUp => KeyCode::ArrowUp,
        N::ArrowDown => KeyCode::ArrowDown,
        N::F1 => KeyCode::F1,
        N::F2 => KeyCode::F2,
        N::F3 => KeyCode::F3,
        N::F4 => KeyCode::F4,
        N::F5 => KeyCode::F5,
        N::F6 => KeyCode::F6,
        N::F7 => KeyCode::F7,
        N::F8 => KeyCode::F8,
        N::F9 => KeyCode::F9,
        N::F10 => KeyCode::F10,
        N::F11 => KeyCode::F11,
        N::F12 => KeyCode::F12,
        N::Shift => KeyCode::ShiftLeft,
        N::Control => KeyCode::ControlLeft,
        N::Alt => KeyCode::AltLeft,
        N::Super => KeyCode::MetaLeft,
        N::NumLock => KeyCode::NumLock,
        N::CapsLock | N::ScrollLock | N::PrintScreen | N::Pause => KeyCode::Unmapped(0),
        _ => KeyCode::Unmapped(0),
    }
}

fn map_char_key(s: &str) -> KeyCode {
    let c = match s.chars().next() {
        Some(c) => c,
        None => return KeyCode::Unmapped(0),
    };
    match c.to_ascii_lowercase() {
        'a'..='z' => char_to_letter(c),
        '0'..='9' => char_to_digit(c),
        '-' => KeyCode::Minus,
        '=' => KeyCode::Equal,
        '[' => KeyCode::BracketLeft,
        ']' => KeyCode::BracketRight,
        '\\' => KeyCode::Backslash,
        ';' => KeyCode::Semicolon,
        '\'' => KeyCode::Quote,
        '`' => KeyCode::Backquote,
        ',' => KeyCode::Comma,
        '.' => KeyCode::Period,
        '/' => KeyCode::Slash,
        _ => KeyCode::Unmapped(0),
    }
}

fn char_to_letter(c: char) -> KeyCode {
    match c.to_ascii_lowercase() {
        'a' => KeyCode::A,
        'b' => KeyCode::B,
        'c' => KeyCode::C,
        'd' => KeyCode::D,
        'e' => KeyCode::E,
        'f' => KeyCode::F,
        'g' => KeyCode::G,
        'h' => KeyCode::H,
        'i' => KeyCode::I,
        'j' => KeyCode::J,
        'k' => KeyCode::K,
        'l' => KeyCode::L,
        'm' => KeyCode::M,
        'n' => KeyCode::N,
        'o' => KeyCode::O,
        'p' => KeyCode::P,
        'q' => KeyCode::Q,
        'r' => KeyCode::R,
        's' => KeyCode::S,
        't' => KeyCode::T,
        'u' => KeyCode::U,
        'v' => KeyCode::V,
        'w' => KeyCode::W,
        'x' => KeyCode::X,
        'y' => KeyCode::Y,
        'z' => KeyCode::Z,
        _ => KeyCode::Unmapped(0),
    }
}

fn char_to_digit(c: char) -> KeyCode {
    match c {
        '0' => KeyCode::Digit0,
        '1' => KeyCode::Digit1,
        '2' => KeyCode::Digit2,
        '3' => KeyCode::Digit3,
        '4' => KeyCode::Digit4,
        '5' => KeyCode::Digit5,
        '6' => KeyCode::Digit6,
        '7' => KeyCode::Digit7,
        '8' => KeyCode::Digit8,
        '9' => KeyCode::Digit9,
        _ => KeyCode::Unmapped(0),
    }
}

fn winit_scancode_to_raw(code: winit::keyboard::KeyCode) -> u32 {
    code as u32
}

fn apply_cursor(window: &winit::window::Window, style: CursorStyle) {
    use winit::window::CursorIcon;
    let icon = match style {
        CursorStyle::Default => CursorIcon::Default,
        CursorStyle::Text => CursorIcon::Text,
        CursorStyle::Pointer => CursorIcon::Pointer,
        CursorStyle::Move => CursorIcon::Move,
        CursorStyle::NotAllowed => CursorIcon::NotAllowed,
        CursorStyle::ResizeHorizontal => CursorIcon::EwResize,
        CursorStyle::ResizeVertical => CursorIcon::NsResize,
        CursorStyle::ResizeDiagonal1 => CursorIcon::NeswResize,
        CursorStyle::ResizeDiagonal2 => CursorIcon::NwseResize,
        CursorStyle::Hidden => {
            window.set_cursor_visible(false);
            return;
        }
    };
    window.set_cursor_visible(true);
    window.set_cursor(icon);
}

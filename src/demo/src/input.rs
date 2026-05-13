//! Input mapping: winit events → application commands.

use tracing::info;
use winit::event::{MouseButton, MouseScrollDelta, WindowEvent};
use winit::keyboard::{Key, NamedKey};

use crate::app::TextTarget;

const SCROLL_PIXEL_DIVISOR: f32 = 20.0;
const SCROLL_SENSITIVITY: f32 = 2.0;

/// High-level command produced from raw input.
#[derive(Clone, Debug)]
pub enum Command {
    Exit,
    ToggleCommandBar,
    ToggleHelp,
    Type(char, TextTarget),
    Backspace(TextTarget),
    HistoryUp,
    HistoryDown,
    Execute,
    AddCircle,
    ClearCircles,
    ResizeCursor(f32),
    CursorMoved(f64, f64),
    WindowResized(u32, u32),
}

/// Tracks modifier state and converts winit events into semantic commands.
pub struct InputHandler {
    modifiers: winit::keyboard::ModifiersState,
}

impl Default for InputHandler {
    fn default() -> Self {
        Self {
            modifiers: winit::keyboard::ModifiersState::empty(),
        }
    }
}

impl InputHandler {
    /// Process a window event and emit zero or more commands.
    pub fn handle(&mut self, event: &WindowEvent, command_bar_visible: bool) -> Vec<Command> {
        let mut cmds = Vec::new();
        match event {
            WindowEvent::ModifiersChanged(m) => {
                self.modifiers = m.state();
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Pressed =>
            {
                cmds.extend(self.handle_key(&event.logical_key, event.repeat, command_bar_visible));
            }
            WindowEvent::CursorMoved { position, .. } => {
                cmds.push(Command::CursorMoved(position.x, position.y));
            }
            WindowEvent::MouseInput {
                button,
                state: btn_state,
                ..
            } if *btn_state == winit::event::ElementState::Pressed => match button {
                MouseButton::Left => cmds.push(Command::AddCircle),
                MouseButton::Right => cmds.push(Command::ClearCircles),
                _ => {}
            },
            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    MouseScrollDelta::LineDelta(_, y) => *y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / SCROLL_PIXEL_DIVISOR,
                };
                cmds.push(Command::ResizeCursor(dy * SCROLL_SENSITIVITY));
            }
            WindowEvent::Resized(size) => {
                cmds.push(Command::WindowResized(size.width, size.height));
            }
            WindowEvent::CloseRequested => {
                info!("exit requested via close button");
                cmds.push(Command::Exit);
            }
            _ => {}
        }
        cmds
    }

    fn handle_key(&self, key: &Key, repeat: bool, command_bar_visible: bool) -> Vec<Command> {
        let mut cmds = Vec::new();
        match key {
            Key::Named(NamedKey::F1) => {
                cmds.push(Command::ToggleHelp);
            }
            Key::Named(NamedKey::Escape) => {
                if self.modifiers.control_key() && self.modifiers.shift_key() {
                    info!("panic gesture triggered");
                    cmds.push(Command::Exit);
                } else if command_bar_visible {
                    cmds.push(Command::ToggleCommandBar);
                } else {
                    cmds.push(Command::Exit);
                }
            }
            Key::Named(NamedKey::Backspace) => {
                let target = if command_bar_visible {
                    TextTarget::Command
                } else {
                    TextTarget::Typed
                };
                cmds.push(Command::Backspace(target));
            }
            Key::Named(NamedKey::Enter) if command_bar_visible => {
                cmds.push(Command::Execute);
            }
            Key::Named(NamedKey::ArrowUp) if command_bar_visible => {
                cmds.push(Command::HistoryUp);
            }
            Key::Named(NamedKey::ArrowDown) if command_bar_visible => {
                cmds.push(Command::HistoryDown);
            }
            Key::Named(NamedKey::Space) if self.modifiers.shift_key() => {
                cmds.push(Command::ToggleCommandBar);
            }
            Key::Character(ch) if !repeat => {
                let target = if command_bar_visible {
                    TextTarget::Command
                } else {
                    TextTarget::Typed
                };
                for c in ch.chars() {
                    cmds.push(Command::Type(c, target));
                }
            }
            _ => {}
        }
        cmds
    }
}

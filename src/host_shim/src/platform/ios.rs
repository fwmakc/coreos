//! iOS platform backend (winit-based).

use crate::backend::{CursorStyle, HostBackend, HostError};
use crate::events::KeyCode;
use crate::host_event::{HostEvent, WindowId};
use crate::platform::Platform;
use crate::window::WindowConfig;

use super::winit_impl;

pub struct IosPlatform {
    next_window_id: u64,
    event_queue: Vec<HostEvent>,
    exit_requested: bool,
    pending_config: Option<WindowConfig>,
    cursor_style: CursorStyle,
}

impl IosPlatform {
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

impl HostBackend for IosPlatform {
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

    fn set_cursor_style(&mut self, _style: CursorStyle) {}
}

impl Platform for IosPlatform {
    fn push_event(&mut self, event: HostEvent) {
        self.event_queue.push(event);
    }

    fn run(&mut self, event_handler: &mut dyn FnMut(HostEvent)) -> Result<(), HostError> {
        let config = self.pending_config.take().unwrap_or_default();
        let cursor_style = self.cursor_style;
        winit_impl::run_winit_loop(config, cursor_style, event_handler, |key, state, _mods| {
            if state != winit::event::ElementState::Pressed {
                return false;
            }
            key == KeyCode::Escape
        })
    }
}

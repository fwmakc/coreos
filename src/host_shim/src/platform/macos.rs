//! macOS platform backend.

use crate::backend::{HostBackend, HostError};
use crate::host_event::{HostEvent, WindowId};
use crate::platform::Platform;
use crate::window::WindowConfig;

/// macOS platform implementation.
pub struct MacosPlatform {
    next_window_id: u64,
    event_queue: Vec<HostEvent>,
    exit_requested: bool,
}

impl MacosPlatform {
    /// Create a new macOS platform backend.
    pub fn new() -> Self {
        Self {
            next_window_id: 1,
            event_queue: Vec::new(),
            exit_requested: false,
        }
    }
}

impl HostBackend for MacosPlatform {
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

impl Platform for MacosPlatform {
    fn push_event(&mut self, event: HostEvent) {
        self.event_queue.push(event);
    }

    fn run(&mut self, _event_handler: &mut dyn FnMut(HostEvent)) -> Result<(), HostError> {
        Ok(())
    }
}

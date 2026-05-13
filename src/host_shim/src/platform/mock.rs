//! Mock platform backend for unit testing.

use crate::backend::{HostBackend, HostError};
use crate::host_event::{HostEvent, WindowId};
use crate::platform::Platform;
use crate::window::WindowConfig;

/// In-memory platform for tests.
pub struct MockPlatform {
    next_window_id: u64,
    event_queue: Vec<HostEvent>,
    exit_requested: bool,
    init_called: bool,
    shutdown_called: bool,
}

impl MockPlatform {
    /// Create a new mock platform.
    pub fn new() -> Self {
        Self {
            next_window_id: 1,
            event_queue: Vec::new(),
            exit_requested: false,
            init_called: false,
            shutdown_called: false,
        }
    }

    /// Returns true if `init` was called.
    pub fn init_called(&self) -> bool {
        self.init_called
    }

    /// Returns true if `shutdown` was called.
    pub fn shutdown_called(&self) -> bool {
        self.shutdown_called
    }

    /// Returns true if `request_exit` was called.
    pub fn exit_requested(&self) -> bool {
        self.exit_requested
    }
}

impl Default for MockPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl HostBackend for MockPlatform {
    fn init(&mut self) -> Result<(), HostError> {
        self.init_called = true;
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

    fn shutdown(&mut self) {
        self.shutdown_called = true;
    }

    fn set_cursor_style(&mut self, _style: crate::backend::CursorStyle) {}
}

impl Platform for MockPlatform {
    fn push_event(&mut self, event: HostEvent) {
        self.event_queue.push(event);
    }

    fn run(&mut self, event_handler: &mut dyn FnMut(HostEvent)) -> Result<(), HostError> {
        for ev in self.poll_events() {
            event_handler(ev);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{InputEvent, KeyCode, KeyState, MouseButton};

    #[test]
    fn mock_init_sets_flag() {
        let mut mock = MockPlatform::new();
        assert!(!mock.init_called());
        mock.init().unwrap();
        assert!(mock.init_called());
    }

    #[test]
    fn mock_create_window_returns_incrementing_ids() {
        let mut mock = MockPlatform::new();
        let w1 = mock.create_window(WindowConfig::default()).unwrap();
        let w2 = mock.create_window(WindowConfig::default()).unwrap();
        assert_eq!(w1, WindowId(1));
        assert_eq!(w2, WindowId(2));
    }

    #[test]
    fn mock_poll_events_drains_queue() {
        let mut mock = MockPlatform::new();
        mock.push_event(HostEvent::PanicExit);
        mock.push_event(HostEvent::Close {
            window: WindowId(1),
        });

        let batch1 = mock.poll_events();
        assert_eq!(batch1.len(), 2);
        assert!(matches!(batch1[0], HostEvent::PanicExit));

        let batch2 = mock.poll_events();
        assert!(batch2.is_empty());
    }

    #[test]
    fn mock_request_exit_sets_flag() {
        let mut mock = MockPlatform::new();
        assert!(!mock.exit_requested());
        mock.request_exit();
        assert!(mock.exit_requested());
    }

    #[test]
    fn mock_shutdown_sets_flag() {
        let mut mock = MockPlatform::new();
        assert!(!mock.shutdown_called());
        mock.shutdown();
        assert!(mock.shutdown_called());
    }

    #[test]
    fn mock_full_lifecycle() {
        let mut mock = MockPlatform::new();

        // Init
        mock.init().unwrap();
        assert!(mock.init_called());

        // Create window
        let win = mock.create_window(WindowConfig::default()).unwrap();
        assert_eq!(win, WindowId(1));

        // Simulate input
        mock.push_event(HostEvent::Input(InputEvent::Keyboard {
            key: KeyCode::Escape,
            state: KeyState::Pressed,
            scancode: 1,
        }));
        mock.push_event(HostEvent::Input(InputEvent::MouseButton {
            button: MouseButton::Left,
            state: KeyState::Pressed,
        }));
        mock.push_event(HostEvent::PanicExit);

        // Process events
        let events = mock.poll_events();
        assert_eq!(events.len(), 3);
        assert!(matches!(events[2], HostEvent::PanicExit));

        // Exit
        mock.request_exit();
        assert!(mock.exit_requested());

        // Shutdown
        mock.shutdown();
        assert!(mock.shutdown_called());
    }

    #[test]
    fn mock_run_delivers_panic_exit() {
        let mut mock = MockPlatform::new();
        mock.push_event(HostEvent::PanicExit);
        mock.push_event(HostEvent::Close {
            window: WindowId(1),
        });

        let mut collected = Vec::new();
        mock.run(&mut |ev| collected.push(ev)).unwrap();

        assert_eq!(collected.len(), 2);
        assert!(matches!(collected[0], HostEvent::PanicExit));
        assert!(matches!(collected[1], HostEvent::Close { .. }));
    }

    #[test]
    fn default_platform_creates_instance() {
        let platform = super::super::default_platform();
        // Just verify it doesn't panic; type is platform-specific.
        let _ = platform;
    }
}

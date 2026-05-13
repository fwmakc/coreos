//! Host-level events: window lifecycle, system signals, and wrapped input.

use crate::events::InputEvent;

/// Opaque handle to a host window.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct WindowId(pub u64);

/// Unified event produced by the host platform.
#[derive(Debug, Clone, PartialEq)]
pub enum HostEvent {
    /// Window resized.
    Resize {
        /// Window handle.
        window: WindowId,
        /// New width in physical pixels.
        width: u32,
        /// New height in physical pixels.
        height: u32,
    },
    /// Window moved.
    Move {
        /// Window handle.
        window: WindowId,
        /// New X position on screen.
        x: i32,
        /// New Y position on screen.
        y: i32,
    },
    /// Window close requested.
    Close {
        /// Window handle.
        window: WindowId,
    },
    /// Window focus changed.
    Focus {
        /// Window handle.
        window: WindowId,
        /// Gained or lost focus.
        focused: bool,
    },
    /// Window minimized.
    Minimize {
        /// Window handle.
        window: WindowId,
    },
    /// Window maximized.
    Maximize {
        /// Window handle.
        window: WindowId,
    },
    /// Window restored from minimized/maximized.
    Restore {
        /// Window handle.
        window: WindowId,
    },
    /// System panic gesture triggered (Ctrl+Shift+Esc, etc.).
    PanicExit,
    /// Raw input event.
    Input(InputEvent),
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{KeyCode, KeyState};

    #[test]
    fn window_id_equality() {
        assert_eq!(WindowId(1), WindowId(1));
        assert_ne!(WindowId(1), WindowId(2));
    }

    #[test]
    fn host_event_resize() {
        let ev = HostEvent::Resize {
            window: WindowId(0),
            width: 1920,
            height: 1080,
        };
        match ev {
            HostEvent::Resize { width, height, .. } => {
                assert_eq!(width, 1920);
                assert_eq!(height, 1080);
            }
            _ => panic!("Expected Resize"),
        }
    }

    #[test]
    fn host_event_move() {
        let ev = HostEvent::Move {
            window: WindowId(1),
            x: 100,
            y: 200,
        };
        match ev {
            HostEvent::Move { x, y, .. } => {
                assert_eq!(x, 100);
                assert_eq!(y, 200);
            }
            _ => panic!("Expected Move"),
        }
    }

    #[test]
    fn host_event_close() {
        let ev = HostEvent::Close {
            window: WindowId(2),
        };
        assert!(matches!(ev, HostEvent::Close { .. }));
    }

    #[test]
    fn host_event_focus() {
        let ev = HostEvent::Focus {
            window: WindowId(3),
            focused: true,
        };
        match ev {
            HostEvent::Focus { focused, .. } => assert!(focused),
            _ => panic!("Expected Focus"),
        }
    }

    #[test]
    fn host_event_minimize_maximize_restore() {
        let mini = HostEvent::Minimize {
            window: WindowId(4),
        };
        let maxi = HostEvent::Maximize {
            window: WindowId(4),
        };
        let rest = HostEvent::Restore {
            window: WindowId(4),
        };
        assert!(matches!(mini, HostEvent::Minimize { .. }));
        assert!(matches!(maxi, HostEvent::Maximize { .. }));
        assert!(matches!(rest, HostEvent::Restore { .. }));
    }

    #[test]
    fn host_event_panic_exit() {
        assert!(matches!(HostEvent::PanicExit, HostEvent::PanicExit));
    }

    #[test]
    fn host_event_input_wraps_keyboard() {
        let input = InputEvent::Keyboard {
            key: KeyCode::Escape,
            state: KeyState::Pressed,
            scancode: 1,
        };
        let ev = HostEvent::Input(input.clone());
        assert_eq!(ev, HostEvent::Input(input));
    }

    #[test]
    fn host_event_clone_equality() {
        let ev = HostEvent::Resize {
            window: WindowId(7),
            width: 800,
            height: 600,
        };
        assert_eq!(ev.clone(), ev);
    }
}

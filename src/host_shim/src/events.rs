//! Input events: keyboard, mouse, touch, gestures.

/// Unified input event.
#[derive(Debug, Clone, PartialEq)]
pub enum InputEvent {
    /// Keyboard key press/release.
    Keyboard {
        /// Virtual key code.
        key: KeyCode,
        /// Pressed or released.
        state: KeyState,
        /// Platform-native scancode.
        scancode: u32,
    },
    /// Mouse move.
    MouseMove {
        /// X coordinate in physical pixels.
        x: f64,
        /// Y coordinate in physical pixels.
        y: f64,
    },
    /// Mouse button.
    MouseButton {
        /// Which button.
        button: MouseButton,
        /// Pressed or released.
        state: KeyState,
    },
    /// Touch event (mobile).
    Touch {
        /// Touch identifier.
        id: u64,
        /// Touch phase.
        phase: TouchPhase,
        /// X coordinate.
        x: f64,
        /// Y coordinate.
        y: f64,
    },
}

/// Key state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    /// Key pressed.
    Pressed,
    /// Key released.
    Released,
}

/// Mouse buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    /// Left button.
    Left,
    /// Right button.
    Right,
    /// Middle button.
    Middle,
}

/// Touch phases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchPhase {
    /// Finger touched the surface.
    Started,
    /// Finger moved.
    Moved,
    /// Finger lifted.
    Ended,
    /// Gesture cancelled.
    Cancelled,
}

/// Simplified key code enum (will be expanded in phase 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyCode {
    /// Escape key.
    Escape,
    /// Enter/Return.
    Enter,
    /// Space.
    Space,
    /// Unmapped key (raw scancode).
    Unmapped(u32),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_state_equality() {
        assert_eq!(KeyState::Pressed, KeyState::Pressed);
        assert_ne!(KeyState::Pressed, KeyState::Released);
    }

    #[test]
    fn mouse_button_equality() {
        assert_eq!(MouseButton::Left, MouseButton::Left);
        assert_ne!(MouseButton::Left, MouseButton::Right);
    }

    #[test]
    fn touch_phase_ordering() {
        let phases = vec![
            TouchPhase::Started,
            TouchPhase::Moved,
            TouchPhase::Ended,
            TouchPhase::Cancelled,
        ];
        assert_eq!(phases.len(), 4);
    }

    #[test]
    fn input_event_clone_equality() {
        let ev = InputEvent::Keyboard {
            key: KeyCode::Escape,
            state: KeyState::Pressed,
            scancode: 1,
        };
        let cloned = ev.clone();
        assert_eq!(ev, cloned);
    }

    #[test]
    fn keyboard_event_fields() {
        let ev = InputEvent::Keyboard {
            key: KeyCode::Enter,
            state: KeyState::Released,
            scancode: 28,
        };
        match ev {
            InputEvent::Keyboard { key, state, scancode } => {
                assert_eq!(key, KeyCode::Enter);
                assert_eq!(state, KeyState::Released);
                assert_eq!(scancode, 28);
            }
            _ => panic!("Expected Keyboard event"),
        }
    }

    #[test]
    fn mouse_move_fields() {
        let ev = InputEvent::MouseMove { x: 100.5, y: 200.0 };
        match ev {
            InputEvent::MouseMove { x, y } => {
                assert!((x - 100.5).abs() < f64::EPSILON);
                assert!((y - 200.0).abs() < f64::EPSILON);
            }
            _ => panic!("Expected MouseMove event"),
        }
    }

    #[test]
    fn mouse_button_fields() {
        let ev = InputEvent::MouseButton {
            button: MouseButton::Middle,
            state: KeyState::Pressed,
        };
        match ev {
            InputEvent::MouseButton { button, state } => {
                assert_eq!(button, MouseButton::Middle);
                assert_eq!(state, KeyState::Pressed);
            }
            _ => panic!("Expected MouseButton event"),
        }
    }

    #[test]
    fn touch_event_fields() {
        let ev = InputEvent::Touch {
            id: 42,
            phase: TouchPhase::Started,
            x: 50.0,
            y: 75.0,
        };
        match ev {
            InputEvent::Touch { id, phase, x, y } => {
                assert_eq!(id, 42);
                assert_eq!(phase, TouchPhase::Started);
                assert!((x - 50.0).abs() < f64::EPSILON);
                assert!((y - 75.0).abs() < f64::EPSILON);
            }
            _ => panic!("Expected Touch event"),
        }
    }

    #[test]
    fn keycode_unmapped_roundtrip() {
        let code = KeyCode::Unmapped(999);
        match code {
            KeyCode::Unmapped(v) => assert_eq!(v, 999),
            _ => panic!("Expected Unmapped"),
        }
    }
}

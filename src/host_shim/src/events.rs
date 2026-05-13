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
    /// Extra button 1 (browser back).
    X1,
    /// Extra button 2 (browser forward).
    X2,
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

/// Platform-agnostic key code.
#[allow(missing_docs)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum KeyCode {
    // --- Letters ---
    /// A
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    /// N
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    // --- Digits ---
    /// 0
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    /// 5
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // --- Function keys ---
    /// F1
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // --- Navigation ---
    /// Escape
    Escape,
    /// Enter / Return
    Enter,
    /// Space
    Space,
    /// Tab
    Tab,
    /// Backspace
    Backspace,
    /// Delete
    Delete,
    /// Insert
    Insert,
    /// Home
    Home,
    /// End
    End,
    /// Page Up
    PageUp,
    /// Page Down
    PageDown,

    // --- Arrows ---
    /// Left arrow
    ArrowLeft,
    /// Right arrow
    ArrowRight,
    /// Up arrow
    ArrowUp,
    /// Down arrow
    ArrowDown,

    // --- Modifiers ---
    /// Left Shift
    ShiftLeft,
    /// Right Shift
    ShiftRight,
    /// Left Ctrl
    ControlLeft,
    /// Right Ctrl
    ControlRight,
    /// Left Alt / Option
    AltLeft,
    /// Right Alt / Option
    AltRight,
    /// Meta / Command / Windows
    MetaLeft,
    /// Meta / Command / Windows
    MetaRight,

    // --- Symbols ---
    /// Minus / Underscore
    Minus,
    /// Equal / Plus
    Equal,
    /// Left bracket
    BracketLeft,
    /// Right bracket
    BracketRight,
    /// Backslash / Pipe
    Backslash,
    /// Semicolon / Colon
    Semicolon,
    /// Quote / Double quote
    Quote,
    /// Backtick / Tilde
    Backquote,
    /// Comma / Less-than
    Comma,
    /// Period / Greater-than
    Period,
    /// Slash / Question mark
    Slash,

    // --- Numpad ---
    /// NumLock
    NumLock,
    /// Numpad 0
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    /// Numpad 5
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    /// Numpad add
    NumpadAdd,
    /// Numpad subtract
    NumpadSubtract,
    /// Numpad multiply
    NumpadMultiply,
    /// Numpad divide
    NumpadDivide,
    /// Numpad enter
    NumpadEnter,
    /// Numpad decimal
    NumpadDecimal,

    // --- Unmapped ---
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
        assert_eq!(MouseButton::X1, MouseButton::X1);
        assert_eq!(MouseButton::X2, MouseButton::X2);
    }

    #[test]
    fn touch_phase_ordering() {
        let phases = [
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
            InputEvent::Keyboard {
                key,
                state,
                scancode,
            } => {
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
    fn keycode_full_alphabet() {
        let letters = vec![
            KeyCode::A,
            KeyCode::B,
            KeyCode::C,
            KeyCode::D,
            KeyCode::E,
            KeyCode::F,
            KeyCode::G,
            KeyCode::H,
            KeyCode::I,
            KeyCode::J,
            KeyCode::K,
            KeyCode::L,
            KeyCode::M,
            KeyCode::N,
            KeyCode::O,
            KeyCode::P,
            KeyCode::Q,
            KeyCode::R,
            KeyCode::S,
            KeyCode::T,
            KeyCode::U,
            KeyCode::V,
            KeyCode::W,
            KeyCode::X,
            KeyCode::Y,
            KeyCode::Z,
        ];
        assert_eq!(letters.len(), 26);
    }

    #[test]
    fn keycode_digits() {
        let digits = vec![
            KeyCode::Digit0,
            KeyCode::Digit1,
            KeyCode::Digit2,
            KeyCode::Digit3,
            KeyCode::Digit4,
            KeyCode::Digit5,
            KeyCode::Digit6,
            KeyCode::Digit7,
            KeyCode::Digit8,
            KeyCode::Digit9,
        ];
        assert_eq!(digits.len(), 10);
    }

    #[test]
    fn keycode_function_keys() {
        let fkeys = vec![
            KeyCode::F1,
            KeyCode::F2,
            KeyCode::F3,
            KeyCode::F4,
            KeyCode::F5,
            KeyCode::F6,
            KeyCode::F7,
            KeyCode::F8,
            KeyCode::F9,
            KeyCode::F10,
            KeyCode::F11,
            KeyCode::F12,
        ];
        assert_eq!(fkeys.len(), 12);
    }

    #[test]
    fn keycode_modifiers() {
        let mods = [
            KeyCode::ShiftLeft,
            KeyCode::ShiftRight,
            KeyCode::ControlLeft,
            KeyCode::ControlRight,
            KeyCode::AltLeft,
            KeyCode::AltRight,
            KeyCode::MetaLeft,
            KeyCode::MetaRight,
        ];
        assert_eq!(mods.len(), 8);
    }

    #[test]
    fn keycode_arrows() {
        let arrows = [
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
        ];
        assert_eq!(arrows.len(), 4);
    }

    #[test]
    fn keycode_navigation() {
        let nav = vec![
            KeyCode::Escape,
            KeyCode::Enter,
            KeyCode::Space,
            KeyCode::Tab,
            KeyCode::Backspace,
            KeyCode::Delete,
            KeyCode::Insert,
            KeyCode::Home,
            KeyCode::End,
            KeyCode::PageUp,
            KeyCode::PageDown,
        ];
        assert_eq!(nav.len(), 11);
    }

    #[test]
    fn keycode_unmapped_roundtrip() {
        let code = KeyCode::Unmapped(999);
        match code {
            KeyCode::Unmapped(v) => assert_eq!(v, 999),
            _ => panic!("Expected Unmapped"),
        }
    }

    #[test]
    fn keycode_numpad() {
        let numpad = vec![
            KeyCode::Numpad0,
            KeyCode::Numpad1,
            KeyCode::Numpad2,
            KeyCode::Numpad3,
            KeyCode::Numpad4,
            KeyCode::Numpad5,
            KeyCode::Numpad6,
            KeyCode::Numpad7,
            KeyCode::Numpad8,
            KeyCode::Numpad9,
            KeyCode::NumpadAdd,
            KeyCode::NumpadSubtract,
            KeyCode::NumpadMultiply,
            KeyCode::NumpadDivide,
            KeyCode::NumpadEnter,
            KeyCode::NumpadDecimal,
            KeyCode::NumLock,
        ];
        assert_eq!(numpad.len(), 17);
    }
}

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

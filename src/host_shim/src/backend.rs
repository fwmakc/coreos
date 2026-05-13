//! Platform-specific backend traits.

use crate::host_event::{HostEvent, WindowId};
use crate::window::WindowConfig;

/// Common interface for all host platforms.
pub trait HostBackend {
    /// Initialize the backend.
    fn init(&mut self) -> Result<(), HostError>;
    /// Shutdown and cleanup.
    fn shutdown(&mut self);
    /// Create a window with the given configuration.
    fn create_window(&mut self, config: WindowConfig) -> Result<WindowId, HostError>;
    /// Drain pending host events.
    fn poll_events(&mut self) -> Vec<HostEvent>;
    /// Request graceful exit.
    fn request_exit(&mut self);
    /// Set the cursor style for the active window.
    fn set_cursor_style(&mut self, style: CursorStyle);
}

/// Cursor appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CursorStyle {
    /// Default arrow.
    Default,
    /// Text input I-beam.
    Text,
    /// Pointer / hand.
    Pointer,
    /// Move / grab.
    Move,
    /// Not allowed.
    NotAllowed,
    /// Resize horizontal.
    ResizeHorizontal,
    /// Resize vertical.
    ResizeVertical,
    /// Resize diagonal (top-left to bottom-right).
    ResizeDiagonal1,
    /// Resize diagonal (top-right to bottom-left).
    ResizeDiagonal2,
    /// Hidden cursor.
    Hidden,
}

/// Errors returned by host operations.
#[derive(Debug)]
pub enum HostError {
    /// Window creation failed.
    WindowCreationFailed(String),
    /// Audio subsystem unavailable.
    AudioUnavailable,
    /// Network initialization failed.
    NetworkInitFailed(String),
    /// Platform not supported.
    PlatformNotSupported,
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::WindowCreationFailed(e) => write!(f, "window creation failed: {e}"),
            HostError::AudioUnavailable => write!(f, "audio subsystem unavailable"),
            HostError::NetworkInitFailed(e) => write!(f, "network init failed: {e}"),
            HostError::PlatformNotSupported => write!(f, "platform not supported"),
        }
    }
}

impl std::error::Error for HostError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::{InputEvent, KeyCode, KeyState};
    use crate::platform::mock::MockPlatform;
    use crate::platform::Platform;

    /// Verify that MockPlatform can be used as a dyn HostBackend.
    /// This ensures our trait design allows test doubles.
    #[test]
    fn mock_platform_acts_as_host_backend() {
        let mut mock = MockPlatform::new();

        // init
        HostBackend::init(&mut mock).unwrap();
        assert!(mock.init_called());

        // create_window
        let win = HostBackend::create_window(&mut mock, WindowConfig::default()).unwrap();
        assert_eq!(win, WindowId(1));

        // poll_events (empty initially)
        let evs = HostBackend::poll_events(&mut mock);
        assert!(evs.is_empty());

        // push and poll
        mock.push_event(HostEvent::Input(InputEvent::Keyboard {
            key: KeyCode::Escape,
            state: KeyState::Pressed,
            scancode: 1,
        }));
        let evs = HostBackend::poll_events(&mut mock);
        assert_eq!(evs.len(), 1);

        // request_exit
        HostBackend::request_exit(&mut mock);
        assert!(mock.exit_requested());

        // set_cursor_style (no-op on mock, just ensure it compiles)
        HostBackend::set_cursor_style(&mut mock, CursorStyle::Pointer);

        // shutdown
        HostBackend::shutdown(&mut mock);
        assert!(mock.shutdown_called());
    }

    #[test]
    fn host_error_display_window_creation() {
        let err = HostError::WindowCreationFailed("DPI aware init failed".into());
        let msg = format!("{err}");
        assert!(msg.contains("window creation failed"));
        assert!(msg.contains("DPI aware init failed"));
    }

    #[test]
    fn host_error_display_audio_unavailable() {
        let err = HostError::AudioUnavailable;
        assert_eq!(format!("{err}"), "audio subsystem unavailable");
    }

    #[test]
    fn host_error_display_network_init() {
        let err = HostError::NetworkInitFailed("port 443 in use".into());
        let msg = format!("{err}");
        assert!(msg.contains("network init failed"));
        assert!(msg.contains("port 443 in use"));
    }

    #[test]
    fn host_error_display_platform_not_supported() {
        let err = HostError::PlatformNotSupported;
        assert_eq!(format!("{err}"), "platform not supported");
    }

    #[test]
    fn host_error_implements_std_error() {
        let err: Box<dyn std::error::Error> = Box::new(HostError::AudioUnavailable);
        assert_eq!(err.to_string(), "audio subsystem unavailable");
    }

    #[test]
    fn host_error_debug() {
        let err = HostError::AudioUnavailable;
        let dbg = format!("{err:?}");
        assert!(dbg.contains("AudioUnavailable"));
    }

    #[test]
    fn cursor_style_variants() {
        // Just verify the enum exists and has expected variants.
        let styles = vec![
            CursorStyle::Default,
            CursorStyle::Text,
            CursorStyle::Pointer,
            CursorStyle::Move,
            CursorStyle::NotAllowed,
            CursorStyle::ResizeHorizontal,
            CursorStyle::ResizeVertical,
            CursorStyle::ResizeDiagonal1,
            CursorStyle::ResizeDiagonal2,
            CursorStyle::Hidden,
        ];
        assert_eq!(styles.len(), 10);
    }
}

//! Platform-specific backend traits.

/// Common interface for all host platforms.
pub trait HostBackend {
    /// Initialize the backend.
    fn init(&mut self) -> Result<(), HostError>;
    /// Shutdown and cleanup.
    fn shutdown(&mut self);
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
}

impl std::fmt::Display for HostError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HostError::WindowCreationFailed(e) => write!(f, "window creation failed: {e}"),
            HostError::AudioUnavailable => write!(f, "audio subsystem unavailable"),
            HostError::NetworkInitFailed(e) => write!(f, "network init failed: {e}"),
        }
    }
}

impl std::error::Error for HostError {}

#[cfg(test)]
mod tests {
    use super::*;

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
}

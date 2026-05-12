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

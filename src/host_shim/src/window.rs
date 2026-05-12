//! Window management abstraction.

/// Window configuration.
#[derive(Debug, Clone)]
pub struct WindowConfig {
    /// Window title.
    pub title: String,
    /// Width in logical pixels.
    pub width: u32,
    /// Height in logical pixels.
    pub height: u32,
    /// Enable high-DPI scaling.
    pub high_dpi: bool,
    /// Start in fullscreen.
    pub fullscreen: bool,
}

impl Default for WindowConfig {
    fn default() -> Self {
        Self {
            title: "CORE OS".into(),
            width: 1280,
            height: 720,
            high_dpi: true,
            fullscreen: false,
        }
    }
}

/// Platform-agnostic window handle.
pub struct Window {
    config: WindowConfig,
}

impl Window {
    /// Create a new window with the given configuration.
    pub fn new(config: WindowConfig) -> Self {
        Self { config }
    }

    /// Returns the window configuration.
    pub fn config(&self) -> &WindowConfig {
        &self.config
    }

    /// Request window close.
    pub fn request_close(&mut self) {
        // TODO: implement in phase 1
    }
}

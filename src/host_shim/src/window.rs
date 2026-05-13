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
            title: "Workspace".into(),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn window_config_default() {
        let cfg = WindowConfig::default();
        assert_eq!(cfg.title, "Workspace");
        assert_eq!(cfg.width, 1280);
        assert_eq!(cfg.height, 720);
        assert!(cfg.high_dpi);
        assert!(!cfg.fullscreen);
    }

    #[test]
    fn window_config_custom() {
        let cfg = WindowConfig {
            title: "Test".into(),
            width: 800,
            height: 600,
            high_dpi: false,
            fullscreen: true,
        };
        assert_eq!(cfg.title, "Test");
        assert_eq!(cfg.width, 800);
        assert_eq!(cfg.height, 600);
        assert!(!cfg.high_dpi);
        assert!(cfg.fullscreen);
    }

    #[test]
    fn window_creation() {
        let cfg = WindowConfig::default();
        let win = Window::new(cfg.clone());
        assert_eq!(win.config().title, "Workspace");
        assert_eq!(win.config().width, 1280);
    }

    #[test]
    fn window_request_close_does_not_panic() {
        let cfg = WindowConfig::default();
        let mut win = Window::new(cfg);
        win.request_close();
    }
}

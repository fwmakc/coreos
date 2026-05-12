//! WebView abstraction over platform engines.

/// Supported web engines.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WebEngine {
    /// Chromium Embedded Framework (desktop).
    CEF,
    /// WebKit (macOS, Linux, iOS).
    WebKit,
    /// WebView2 (Windows).
    WebView2,
    /// WKWebView (iOS, macOS).
    WKWebView,
}

/// WebView configuration.
#[derive(Debug, Clone)]
pub struct WebViewConfig {
    /// Which engine to use.
    pub engine: WebEngine,
    /// Enable JavaScript execution.
    pub javascript: bool,
    /// Enable local storage.
    pub local_storage: bool,
}

impl Default for WebViewConfig {
    fn default() -> Self {
        Self {
            engine: WebEngine::CEF,
            javascript: true,
            local_storage: false,
        }
    }
}

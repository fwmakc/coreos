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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn webview_config_default() {
        let cfg = WebViewConfig::default();
        assert_eq!(cfg.engine, WebEngine::CEF);
        assert!(cfg.javascript);
        assert!(!cfg.local_storage);
    }

    #[test]
    fn webview_config_custom() {
        let cfg = WebViewConfig {
            engine: WebEngine::WebKit,
            javascript: false,
            local_storage: true,
        };
        assert_eq!(cfg.engine, WebEngine::WebKit);
        assert!(!cfg.javascript);
        assert!(cfg.local_storage);
    }

    #[test]
    fn web_engine_variants() {
        let engines = [
            WebEngine::CEF,
            WebEngine::WebKit,
            WebEngine::WebView2,
            WebEngine::WKWebView,
        ];
        assert_eq!(engines.len(), 4);
    }

    #[test]
    fn webview_config_clone() {
        let cfg = WebViewConfig::default();
        let cloned = cfg.clone();
        assert_eq!(cfg.engine, cloned.engine);
        assert_eq!(cfg.javascript, cloned.javascript);
        assert_eq!(cfg.local_storage, cloned.local_storage);
    }
}

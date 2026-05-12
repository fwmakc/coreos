//! Island Mode — WebView embedding and legacy application isolation.
//!
//! Provides sandboxed web content rendering via CEF, WebKit, WebView2, or WKWebView.

#![warn(missing_docs)]

pub mod webview;

/// Island mode version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

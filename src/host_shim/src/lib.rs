//! Host Shim — abstraction layer over host operating systems.
//!
//! Provides unified interfaces for window management, input, audio,
//! storage, and network across Windows, macOS, Linux, Android, and iOS.

#![warn(missing_docs)]

pub mod backend;
pub mod events;
pub mod window;

/// Host shim version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

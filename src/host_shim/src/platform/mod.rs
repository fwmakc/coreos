//! Platform abstraction layer.
//!
//! Each subdirectory (`windows`, `linux`, `macos`, etc.) provides a concrete
//! implementation of the `Platform` trait. The top-level module re-exports
//! the active backend based on `#[cfg(target_os)]`.

use crate::backend::HostBackend;
use crate::host_event::HostEvent;

/// Platform-specific backend interface.
///
/// Implementors handle all OS-specific windowing, input, and lifecycle.
/// The rest of the crate (and the rest of the project) talks to the
/// platform only through this trait.
///
/// `Platform` extends `HostBackend` with internal methods (e.g. `push_event`)
/// that are not exposed to upper layers.
pub trait Platform: HostBackend {
    /// Programmatically push an event into the queue (used by tests and mocks).
    fn push_event(&mut self, event: HostEvent);

    /// Run the platform event loop.
    ///
    /// Blocks until `request_exit()` is called or the user closes the last
    /// window. Events are delivered to the provided callback in real time.
    ///
    /// For mock platforms this simply drains the queued events and returns.
    fn run(
        &mut self,
        event_handler: &mut dyn FnMut(HostEvent),
    ) -> Result<(), crate::backend::HostError>;
}

// ------------------------------------------------------------------
// Platform selection
// ------------------------------------------------------------------

#[cfg(target_os = "android")]
mod android;
#[cfg(target_os = "ios")]
mod ios;
#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;
#[cfg(target_os = "windows")]
mod windows;

#[cfg(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos",
    target_os = "android",
    target_os = "ios"
))]
mod winit_impl;

pub mod mock;

/// Create the default platform backend for the current OS.
pub fn default_platform() -> Box<dyn Platform> {
    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsPlatform::new())
    }
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxPlatform::new())
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosPlatform::new())
    }
    #[cfg(target_os = "android")]
    {
        Box::new(android::AndroidPlatform::new())
    }
    #[cfg(target_os = "ios")]
    {
        Box::new(ios::IosPlatform::new())
    }
}

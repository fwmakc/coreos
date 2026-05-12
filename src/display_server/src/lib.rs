//! Display Server — WebGPU rendering and compositing layer.
//!
//! Handles surface creation, swapchain management, 2D primitives,
//! text rendering, and scene graph compositing.

#![warn(missing_docs)]

pub mod renderer;
pub mod surface;

/// Display server version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

//! Micro-Kernel — core runtime bindings for IPC, security, and storage.
//!
//! The TypeScript/Bun implementation lives in `src/micro_kernel/ts/`.
//! This crate provides native FFI bridges where needed.

#![warn(missing_docs)]

pub mod ipc;
pub mod security;

/// Micro-kernel version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

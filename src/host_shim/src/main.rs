//! Minimal demo binary for the host shim layer.
//!
//! Running this will open a native window and print events to stdout.

use w_host_shim::VERSION;

fn main() {
    println!("Workspace Host Shim v{VERSION}");
    println!("Status: foundation in progress — see plan/phase-01-host-shim-windows.md");
    // TODO: init winit event loop (phase 1)
}

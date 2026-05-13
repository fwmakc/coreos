//! Minimal demo binary for the host shim layer.
//!
//! Running this will open a native window and print events to stdout.
//! Usage: `cargo run --bin host_shim_demo`

use w_host_shim::platform::default_platform;
use w_host_shim::window::WindowConfig;
use w_host_shim::VERSION;

fn main() {
    println!("Workspace Host Shim v{VERSION}");

    let mut platform = default_platform();
    platform.init().expect("platform init failed");

    let config = WindowConfig {
        title: "Workspace Host Shim Demo".into(),
        width: 800,
        height: 600,
        ..Default::default()
    };
    let window_id = platform
        .create_window(config)
        .expect("window creation failed");
    println!("created window: {:?}", window_id);

    println!("entering event loop (close window to exit)...");
    let result = platform.run(&mut |event| {
        println!("{:?}", event);
    });

    match result {
        Ok(()) => println!("event loop exited cleanly"),
        Err(e) => eprintln!("event loop error: {}", e),
    }
}

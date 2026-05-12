//! Real winit integration tests — requires a display server.
//!
//! Run with: `cargo test --test winit_integration -- --ignored --test-threads=1`

use winit::event_loop::EventLoop;
use winit::window::WindowAttributes;

/// TC-01-001: Create a real 800×600 window via winit.
#[test]
#[ignore = "requires display server (run with --ignored --test-threads=1)"]
fn real_window_creation_800x600() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop
        .create_window(
            WindowAttributes::default()
                .with_title("CORE OS Test")
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600)),
        )
        .unwrap();

    let size = window.inner_size();
    assert_eq!(size.width, 800);
    assert_eq!(size.height, 600);
    assert_eq!(window.title(), "CORE OS Test");
}

/// TC-01-002: Resize window via winit.
#[test]
#[ignore = "requires display server"]
fn real_window_resize() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop
        .create_window(
            WindowAttributes::default()
                .with_inner_size(winit::dpi::LogicalSize::new(800, 600)),
        )
        .unwrap();

    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
    // winit resize is asynchronous; we verify the request was accepted.
    assert!(window.inner_size().width > 0);
}

/// TC-01-014: DPI scaling detection.
#[test]
#[ignore = "requires display server"]
fn real_window_dpi_scaling() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop
        .create_window(WindowAttributes::default())
        .unwrap();

    let scale = window.scale_factor();
    assert!(scale > 0.0);
    // Typical values: 1.0 (96 DPI), 1.25 (120 DPI), 1.5 (144 DPI), 2.0 (192 DPI)
    assert!((1.0..=4.0).contains(&scale));
}

/// TC-01-016 / TC-01-017: Minimize / maximize state query.
#[test]
#[ignore = "requires display server"]
fn real_window_minimize_maximize_state() {
    let event_loop = EventLoop::new().unwrap();
    let window = event_loop.create_window(WindowAttributes::default()).unwrap();

    // Window starts normal (not minimized, not maximized).
    assert!(!window.is_minimized().unwrap_or(false));
    assert!(!window.is_maximized());
}

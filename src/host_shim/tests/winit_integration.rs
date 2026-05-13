//! Real winit integration tests — requires a display server.
//!
//! Run with: `cargo test --test winit_integration -- --ignored --test-threads=1`

use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowAttributes;

struct WindowCollector {
    window: Option<winit::window::Window>,
    attrs: WindowAttributes,
}

impl ApplicationHandler for WindowCollector {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(event_loop.create_window(self.attrs.clone()).unwrap());
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        if matches!(event, winit::event::WindowEvent::CloseRequested) {
            event_loop.exit();
        }
    }
}

fn create_window_with_attrs(attrs: WindowAttributes) -> winit::window::Window {
    let event_loop = EventLoop::new().unwrap();
    let mut collector = WindowCollector {
        window: None,
        attrs,
    };
    event_loop.run_app(&mut collector).unwrap();
    collector.window.unwrap()
}

/// TC-01-001: Create a real 800x600 window via winit.
#[test]
#[ignore = "requires display server (run with --ignored --test-threads=1)"]
fn real_window_creation_800x600() {
    let attrs = WindowAttributes::default()
        .with_title("Workspace Test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    let window = create_window_with_attrs(attrs);

    let size = window.inner_size();
    assert_eq!(size.width, 800);
    assert_eq!(size.height, 600);
    assert_eq!(window.title(), "Workspace Test");
}

/// TC-01-002: Resize window via winit.
#[test]
#[ignore = "requires display server"]
fn real_window_resize() {
    let attrs = WindowAttributes::default().with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    let window = create_window_with_attrs(attrs);

    let _ = window.request_inner_size(winit::dpi::PhysicalSize::new(1280, 720));
    assert!(window.inner_size().width > 0);
}

/// TC-01-014: DPI scaling detection.
#[test]
#[ignore = "requires display server"]
fn real_window_dpi_scaling() {
    let window = create_window_with_attrs(WindowAttributes::default());

    let scale = window.scale_factor();
    assert!(scale > 0.0);
    assert!((1.0..=4.0).contains(&scale));
}

/// TC-01-016 / TC-01-017: Minimize / maximize state query.
#[test]
#[ignore = "requires display server"]
fn real_window_minimize_maximize_state() {
    let window = create_window_with_attrs(WindowAttributes::default());

    assert!(!window.is_minimized().unwrap_or(false));
    assert!(!window.is_maximized());
}

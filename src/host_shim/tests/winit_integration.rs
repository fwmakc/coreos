//! Real winit integration tests — requires a display server.
//!
//! Run with: `cargo test --test winit_integration -- --ignored --test-threads=1`

use std::sync::{Arc, Mutex};

use winit::application::ApplicationHandler;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::WindowAttributes;

type AssertFn = Box<dyn FnOnce(&winit::window::Window) + Send>;
type SharedAssert = Arc<Mutex<Option<AssertFn>>>;

struct WindowCollector {
    window: Option<winit::window::Window>,
    attrs: WindowAttributes,
    assertions: SharedAssert,
    asserted: bool,
}

impl ApplicationHandler for WindowCollector {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_none() {
            self.window = Some(
                event_loop
                    .create_window(self.attrs.clone())
                    .expect("window creation failed"),
            );
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
            return;
        }
        if let Some(ref window) = self.window {
            if matches!(event, winit::event::WindowEvent::RedrawRequested) && !self.asserted {
                self.asserted = true;
                let f = self.assertions.lock().unwrap().take();
                if let Some(assert_fn) = f {
                    assert_fn(window);
                }
                event_loop.exit();
            }
        }
    }
}

fn run_with_window(
    attrs: WindowAttributes,
    assertions: Box<dyn FnOnce(&winit::window::Window) + Send>,
) {
    let event_loop = EventLoop::new().expect("event loop creation failed");
    let assertions: SharedAssert = Arc::new(Mutex::new(Some(assertions)));

    let mut collector = WindowCollector {
        window: None,
        attrs,
        assertions,
        asserted: false,
    };
    event_loop.run_app(&mut collector).expect("run_app failed");
}

/// TC-01-001: Create a real 800x600 window via winit.
#[test]
#[ignore = "requires display server (run with --ignored --test-threads=1)"]
fn real_window_creation_800x600() {
    let attrs = WindowAttributes::default()
        .with_title("Workspace Test")
        .with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    run_with_window(
        attrs,
        Box::new(|window| {
            let size = window.inner_size();
            assert_eq!(size.width, 800);
            assert_eq!(size.height, 600);
            assert_eq!(window.title(), "Workspace Test");
        }),
    );
}

/// TC-01-002: Window has nonzero dimensions after creation.
#[test]
#[ignore = "requires display server"]
fn real_window_resize() {
    let attrs = WindowAttributes::default().with_inner_size(winit::dpi::LogicalSize::new(800, 600));
    run_with_window(
        attrs,
        Box::new(|window| {
            assert!(window.inner_size().width > 0);
            assert!(window.inner_size().height > 0);
        }),
    );
}

/// TC-01-014: DPI scaling detection.
#[test]
#[ignore = "requires display server"]
fn real_window_dpi_scaling() {
    run_with_window(
        WindowAttributes::default(),
        Box::new(|window| {
            let scale = window.scale_factor();
            assert!(scale > 0.0);
            assert!((1.0..=4.0).contains(&scale));
        }),
    );
}

/// TC-01-016 / TC-01-017: Minimize / maximize state query.
#[test]
#[ignore = "requires display server"]
fn real_window_minimize_maximize_state() {
    run_with_window(
        WindowAttributes::default(),
        Box::new(|window| {
            assert!(!window.is_minimized().unwrap_or(false));
            assert!(!window.is_maximized());
        }),
    );
}

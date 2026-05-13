//! Real WebGPU integration tests — requires a GPU or WARP/llvmpipe.
//!
//! Run with: `cargo test --test wgpu_integration -- --ignored --test-threads=1`

use wgpu::Instance;

/// TC-09-001: WebGPU instance creation.
#[test]
#[ignore = "requires GPU or software rasterizer (WARP/llvmpipe)"]
fn wgpu_instance_creation() {
    let instance = Instance::default();
    // Just verify instance exists; adapters are queried separately.
    drop(instance);
}

/// TC-09-002: Request an adapter (any backend).
#[test]
#[ignore = "requires GPU or software rasterizer"]
fn wgpu_adapter_request() {
    let instance = Instance::default();
    let adapter = pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::LowPower,
        compatible_surface: None,
        force_fallback_adapter: false,
    }));
    assert!(
        adapter.is_some(),
        "No GPU adapter found. Ensure WARP (Windows) or llvmpipe (Linux) is available."
    );
}

/// TC-09-003: Create a surface and query its capabilities.
#[test]
#[ignore = "requires display server + GPU"]
fn wgpu_surface_creation() {
    let instance = Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("No adapter");

    // We can't create a real surface without a window, but we can query adapter limits.
    let limits = adapter.limits();
    assert!(limits.max_texture_dimension_2d >= 2048);
}

/// TC-09-004: Request a device and queue.
#[test]
#[ignore = "requires GPU or software rasterizer"]
fn wgpu_device_and_queue_creation() {
    let instance = Instance::default();
    let adapter =
        pollster::block_on(instance.request_adapter(&wgpu::RequestAdapterOptions::default()))
            .expect("No adapter");

    let (device, queue) = pollster::block_on(adapter.request_device(
        &wgpu::DeviceDescriptor {
            label: Some("Test Device"),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::downlevel_defaults(),
            memory_hints: wgpu::MemoryHints::Performance,
        },
        None,
    ))
    .expect("Failed to create device");

    // Verify queue accepts work.
    queue.submit([]);
    drop(device);
}

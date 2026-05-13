//! Graphics subsystem: wgpu device, surface, and swapchain management.

pub mod shape;
pub mod text;

use tracing::{info, warn};
use wgpu::{SurfaceConfiguration, TextureFormat};
use winit::dpi::PhysicalSize;
use winit::window::Window;

/// Owned GPU context for a single window.
pub struct GraphicsContext {
    window: Window,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: SurfaceConfiguration,
    size: PhysicalSize<u32>,
    surface_format: TextureFormat,
}

impl GraphicsContext {
    /// Initialize wgpu for the given window.
    pub async fn new(window: Window) -> Self {
        let size = window.inner_size();

        let instance = wgpu::Instance::default();
        // SAFETY: The window is a valid winit Window that outlives this
        // GraphicsContext. The raw window handle is only used to create the
        // wgpu surface and is not stored beyond this call. wgpu takes its own
        // reference to the underlying platform window, keeping it alive for
        // as long as the Surface exists (which is owned by GraphicsContext).
        let surface = unsafe {
            instance.create_surface_unsafe(wgpu::SurfaceTargetUnsafe::from_window(&window).unwrap())
        }
        .expect("failed to create surface");

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("no GPU adapter found");

        info!("Selected adapter: {:?}", adapter.get_info());

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Demo Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .expect("failed to create wgpu device");

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);

        let max_texture_size = device.limits().max_texture_dimension_2d;
        let width = size.width.min(max_texture_size).max(1);
        let height = size.height.min(max_texture_size).max(1);

        let config = SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width,
            height,
            present_mode: wgpu::PresentMode::AutoVsync,
            desired_maximum_frame_latency: 2,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
        };

        surface.configure(&device, &config);

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size: PhysicalSize::new(width, height),
            surface_format,
        }
    }

    /// Reconfigure the surface on window resize.
    pub fn resize(&mut self, new_size: PhysicalSize<u32>) {
        if new_size.width == 0 || new_size.height == 0 {
            warn!("resize to zero ignored");
            return;
        }
        let max = self.device.limits().max_texture_dimension_2d;
        let width = new_size.width.min(max).max(1);
        let height = new_size.height.min(max).max(1);
        self.size = PhysicalSize::new(width, height);
        self.config.width = width;
        self.config.height = height;
        self.surface.configure(&self.device, &self.config);
        info!("resized to {}x{}", width, height);
    }

    /// Current drawable size in pixels.
    pub fn size(&self) -> PhysicalSize<u32> {
        self.size
    }

    /// Window inner size (may differ from surface size on HiDPI).
    pub fn window_size_f32(&self) -> (f32, f32) {
        let s = self.window.inner_size();
        (s.width as f32, s.height as f32)
    }

    /// Surface texture format.
    pub fn format(&self) -> TextureFormat {
        self.surface_format
    }

    /// Acquire the next swapchain image.
    pub fn acquire_frame(&self) -> Result<wgpu::SurfaceTexture, wgpu::SurfaceError> {
        self.surface.get_current_texture()
    }

    /// Submit command buffers.
    pub fn submit<I: IntoIterator<Item = wgpu::CommandBuffer>>(&self, commands: I) {
        self.queue.submit(commands);
    }

    /// Create a new command encoder.
    pub fn create_encoder(&self, label: &str) -> wgpu::CommandEncoder {
        self.device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor { label: Some(label) })
    }

    /// Write data to a buffer.
    pub fn write_buffer(&self, buffer: &wgpu::Buffer, offset: wgpu::BufferAddress, data: &[u8]) {
        self.queue.write_buffer(buffer, offset, data);
    }

    pub fn device(&self) -> &wgpu::Device {
        &self.device
    }

    pub fn queue(&self) -> &wgpu::Queue {
        &self.queue
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }
}

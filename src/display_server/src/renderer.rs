//! GPU renderer: primitives, text, and compositor.

/// Render pipeline state.
#[derive(Debug)]
pub struct Renderer {
    /// Target frame budget in milliseconds (60 FPS ≈ 16.67 ms).
    pub frame_budget_ms: f64,
}

impl Default for Renderer {
    fn default() -> Self {
        Self {
            frame_budget_ms: 16.67,
        }
    }
}

impl Renderer {
    /// Create a new renderer.
    pub fn new() -> Self {
        Self::default()
    }

    /// Render a single frame.
    pub fn render_frame(&mut self) {
        // TODO: WebGPU render loop (phase 9–11)
    }
}

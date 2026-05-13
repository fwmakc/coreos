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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_default_frame_budget() {
        let r = Renderer::new();
        assert!((r.frame_budget_ms - 16.67).abs() < 0.01);
    }

    #[test]
    fn renderer_custom_budget() {
        let r = Renderer {
            frame_budget_ms: 8.33,
        };
        assert!((r.frame_budget_ms - 8.33).abs() < 0.01);
    }

    #[test]
    fn renderer_120fps_budget() {
        let r = Renderer {
            frame_budget_ms: 1000.0 / 120.0,
        };
        assert!((r.frame_budget_ms - 8.333).abs() < 0.01);
    }

    #[test]
    fn renderer_render_frame_does_not_panic() {
        let mut r = Renderer::new();
        r.render_frame();
    }
}

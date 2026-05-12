//! Surface and swapchain management.

/// Window surface wrapper.
#[derive(Debug)]
pub struct Surface {
    /// Physical width in pixels.
    pub width: u32,
    /// Physical height in pixels.
    pub height: u32,
}

impl Surface {
    /// Create a new surface with the given dimensions.
    pub fn new(width: u32, height: u32) -> Self {
        Self { width, height }
    }

    /// Resize the surface.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }
}

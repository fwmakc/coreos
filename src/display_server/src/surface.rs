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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_creation() {
        let s = Surface::new(1920, 1080);
        assert_eq!(s.width, 1920);
        assert_eq!(s.height, 1080);
    }

    #[test]
    fn surface_resize() {
        let mut s = Surface::new(800, 600);
        s.resize(1280, 720);
        assert_eq!(s.width, 1280);
        assert_eq!(s.height, 720);
    }

    #[test]
    fn surface_resize_4k() {
        let mut s = Surface::new(800, 600);
        s.resize(3840, 2160);
        assert_eq!(s.width, 3840);
        assert_eq!(s.height, 2160);
    }

    #[test]
    fn surface_resize_to_zero() {
        let mut s = Surface::new(100, 100);
        s.resize(0, 0);
        assert_eq!(s.width, 0);
        assert_eq!(s.height, 0);
    }
}

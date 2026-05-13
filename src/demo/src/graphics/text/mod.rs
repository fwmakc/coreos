//! Text renderer: fontdue rasterisation + single wgpu pipeline.
//!
//! Glyph atlases are cached by (text, font_size). The expensive rasterization
//! and GPU texture upload only runs when text content changes. Vertex positions
//! (which depend on screen position and color) are rebuilt every frame from
//! cached glyph metrics — this is cheap arithmetic.

pub mod atlas;
mod renderer;

use bytemuck::{Pod, Zeroable};

pub use renderer::TextRenderer;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TextVertex {
    pub(crate) position: [f32; 2],
    pub(crate) tex_coords: [f32; 2],
    pub(crate) color: [f32; 4],
}

/// Description of a single text block to render.
pub struct TextEntry<'a> {
    pub text: &'a str,
    pub font_size: f32,
    pub screen_x: f32,
    pub screen_y_baseline: f32,
    pub color: [f32; 4],
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_vertex_size_matches_layout() {
        let expected = std::mem::size_of::<[f32; 2]>()
            + std::mem::size_of::<[f32; 2]>()
            + std::mem::size_of::<[f32; 4]>();
        assert_eq!(std::mem::size_of::<TextVertex>(), expected);
    }

    #[test]
    fn text_vertex_is_32_bytes() {
        assert_eq!(std::mem::size_of::<TextVertex>(), 32);
    }
}

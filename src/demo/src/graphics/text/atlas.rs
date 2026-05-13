//! CPU-side glyph atlas: rasterization, metrics, and vertex construction.
//!
//! This module has zero wgpu dependencies — all GPU resource management
//! lives in [`super::renderer`].

pub(crate) const ATLAS_MIN_SIZE: u32 = 64;
pub(crate) const ATLAS_MAX_SIZE: u32 = 2048;
pub(crate) const GLYPH_PADDING: u32 = 2;

#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct AtlasKey {
    pub(crate) text: String,
    pub(crate) font_size_bits: u32,
}

impl AtlasKey {
    pub(crate) fn new(text: &str, font_size: f32) -> Self {
        Self {
            text: text.to_owned(),
            font_size_bits: font_size.to_bits(),
        }
    }
}

pub(crate) struct GlyphInfo {
    pub(crate) atlas_x: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) xmin: i32,
    pub(crate) ymin: i32,
    pub(crate) advance_width: f32,
}

/// Per-glyph metrics produced by rasterization, consumed by vertex building.
pub(crate) struct AtlasMetrics {
    pub(crate) data: Vec<u8>,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) glyphs: Vec<GlyphInfo>,
}

/// Rasterize glyphs into a horizontal atlas buffer and extract per-glyph metrics.
pub(crate) fn rasterize_atlas(font: &fontdue::Font, text: &str, font_size: f32) -> AtlasMetrics {
    let mut pen_x = 0u32;
    let mut max_height = 0u32;

    for c in text.chars() {
        let (metrics, _) = font.rasterize(c, font_size);
        pen_x += metrics.width as u32 + GLYPH_PADDING;
        max_height = max_height.max(metrics.height as u32);
    }

    let atlas_width = pen_x
        .next_power_of_two()
        .clamp(ATLAS_MIN_SIZE, ATLAS_MAX_SIZE);
    let atlas_height = max_height
        .next_power_of_two()
        .clamp(ATLAS_MIN_SIZE, ATLAS_MAX_SIZE);

    let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
    let mut glyphs = Vec::with_capacity(text.len());
    pen_x = 0;

    for c in text.chars() {
        let (metrics, bitmap) = font.rasterize(c, font_size);
        let w = metrics.width as u32;
        let h = metrics.height as u32;

        for y in 0..h {
            for x in 0..w {
                let atlas_idx = ((y * atlas_width) + pen_x + x) as usize;
                let bitmap_idx = (y * w + x) as usize;
                if atlas_idx < atlas_data.len() && bitmap_idx < bitmap.len() {
                    atlas_data[atlas_idx] = bitmap[bitmap_idx];
                }
            }
        }

        glyphs.push(GlyphInfo {
            atlas_x: pen_x,
            width: w,
            height: h,
            xmin: metrics.xmin,
            ymin: metrics.ymin,
            advance_width: metrics.advance_width,
        });
        pen_x += w + GLYPH_PADDING;
    }

    AtlasMetrics {
        data: atlas_data,
        width: atlas_width,
        height: atlas_height,
        glyphs,
    }
}

/// Build quads from cached glyph metrics + current screen position / color.
pub(crate) fn build_vertices(
    glyphs: &[GlyphInfo],
    atlas_width: u32,
    atlas_height: u32,
    screen_x: f32,
    screen_y_baseline: f32,
    color: [f32; 4],
    screen_to_ndc: &impl Fn(f32, f32) -> [f32; 2],
) -> Vec<super::TextVertex> {
    use super::TextVertex;

    let mut vertices = Vec::new();
    let mut cursor_x = screen_x;

    for gi in glyphs {
        let w_f = gi.width as f32;
        let h_f = gi.height as f32;

        let left = cursor_x + gi.xmin as f32;
        let right = left + w_f;
        let top = screen_y_baseline - (gi.ymin as f32 + h_f);
        let bottom = screen_y_baseline - gi.ymin as f32;

        let [ndc_left, ndc_top] = screen_to_ndc(left, top);
        let [ndc_right, ndc_bottom] = screen_to_ndc(right, bottom);

        let uv_left = gi.atlas_x as f32 / atlas_width as f32;
        let uv_right = (gi.atlas_x + gi.width) as f32 / atlas_width as f32;
        let uv_top = 0.0;
        let uv_bottom = h_f / atlas_height as f32;

        let v = |px, py, u, v| TextVertex {
            position: [px, py],
            tex_coords: [u, v],
            color,
        };
        vertices.extend_from_slice(&[
            v(ndc_left, ndc_bottom, uv_left, uv_bottom),
            v(ndc_right, ndc_bottom, uv_right, uv_bottom),
            v(ndc_left, ndc_top, uv_left, uv_top),
            v(ndc_right, ndc_bottom, uv_right, uv_bottom),
            v(ndc_right, ndc_top, uv_right, uv_top),
            v(ndc_left, ndc_top, uv_left, uv_top),
        ]);

        cursor_x += gi.advance_width;
    }

    vertices
}

//! Text renderer: fontdue rasterisation + single wgpu pipeline.

use tracing::warn;

use super::GraphicsContext;

const MAX_TEXT_VERTICES: usize = 6 * 1024;
const ATLAS_MIN_SIZE: u32 = 64;
const ATLAS_MAX_SIZE: u32 = 2048;
const GLYPH_PADDING: u32 = 2;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct TextVertex {
    position: [f32; 2],
    tex_coords: [f32; 2],
    color: [f32; 4],
}

/// Description of a single text block to render.
pub struct TextEntry<'a> {
    pub text: &'a str,
    pub font_size: f32,
    pub screen_x: f32,
    pub screen_y_baseline: f32,
    pub color: [f32; 4],
}

/// GPU resources for one prepared text block.
pub struct TextDrawCall {
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    texture: wgpu::Texture,
    vertex_offset: u32,
    vertex_count: u32,
}

impl TextDrawCall {
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>, pipeline: &'a wgpu::RenderPipeline) {
        pass.set_pipeline(pipeline);
        pass.set_bind_group(0, &self.bind_group, &[]);
        pass.draw(self.vertex_offset..self.vertex_offset + self.vertex_count, 0..1);
    }
}

/// Renders strings via a dynamically-built R8Unorm glyph atlas.
pub struct TextRenderer {
    font: fontdue::Font,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
}

impl TextRenderer {
    pub fn new(ctx: &GraphicsContext) -> Self {
        let device = ctx.device();
        let format = ctx.format();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/text.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            offset: 0,
                            shader_location: 0,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 2]>() as wgpu::BufferAddress,
                            shader_location: 1,
                            format: wgpu::VertexFormat::Float32x2,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                            shader_location: 2,
                            format: wgpu::VertexFormat::Float32x4,
                        },
                    ],
                }],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Sampler"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: (std::mem::size_of::<TextVertex>() * MAX_TEXT_VERTICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let font = Self::load_system_font();

        Self {
            font,
            pipeline,
            bind_group_layout,
            sampler,
            vertex_buffer,
        }
    }

    #[cfg(target_os = "windows")]
    fn system_font_candidates() -> &'static [&'static str] {
        &[
            r"C:\Windows\Fonts\segoeui.ttf",
            r"C:\Windows\Fonts\arial.ttf",
        ]
    }

    #[cfg(target_os = "linux")]
    fn system_font_candidates() -> &'static [&'static str] {
        &[
            "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
            "/usr/share/fonts/truetype/liberation/LiberationSans-Regular.ttf",
            "/usr/share/fonts/truetype/freefont/FreeSans.ttf",
        ]
    }

    #[cfg(target_os = "macos")]
    fn system_font_candidates() -> &'static [&'static str] {
        &[
            "/System/Library/Fonts/Helvetica.ttc",
            "/Library/Fonts/Arial.ttf",
        ]
    }

    fn load_system_font() -> fontdue::Font {
        let path = Self::system_font_candidates()
            .iter()
            .find(|p| std::path::Path::new(p).exists())
            .expect("no suitable system font found");
        let data = std::fs::read(path).expect("failed to read font file");
        fontdue::Font::from_bytes(data, fontdue::FontSettings::default())
            .expect("invalid font data")
    }

    /// Prepare GPU resources for a slice of text entries.
    ///
    /// Returns draw calls and the total vertex count so the caller can upload the buffer.
    pub fn prepare(
        &self,
        ctx: &GraphicsContext,
        entries: &[TextEntry<'_>],
        screen_to_ndc: impl Fn(f32, f32) -> [f32; 2],
    ) -> (Vec<TextDrawCall>, Vec<TextVertex>) {
        let mut all_vertices = Vec::new();
        let mut draw_calls = Vec::new();

        for entry in entries {
            if entry.text.is_empty() {
                continue;
            }
            let offset = all_vertices.len() as u32;
            if let Some((verts, atlas_data, w, h)) =
                self.layout_text(entry, &screen_to_ndc)
            {
                if all_vertices.len() + verts.len() > MAX_TEXT_VERTICES {
                    warn!(
                        "text vertex buffer full at {} vertices, dropping text entry",
                        all_vertices.len()
                    );
                    continue;
                }
                let count = verts.len() as u32;
                let (texture, bind_group) = self.upload_atlas(ctx, &atlas_data, w, h);
                all_vertices.extend_from_slice(&verts);
                draw_calls.push(TextDrawCall {
                    bind_group,
                    texture,
                    vertex_offset: offset,
                    vertex_count: count,
                });
            }
        }

        (draw_calls, all_vertices)
    }

    fn layout_text(
        &self,
        entry: &TextEntry<'_>,
        screen_to_ndc: &impl Fn(f32, f32) -> [f32; 2],
    ) -> Option<(Vec<TextVertex>, Vec<u8>, u32, u32)> {
        let mut pen_x = 0u32;
        let mut max_height = 0u32;

        for c in entry.text.chars() {
            let (metrics, _) = self.font.rasterize(c, entry.font_size);
            pen_x += metrics.width as u32 + GLYPH_PADDING;
            max_height = max_height.max(metrics.height as u32);
        }

        let atlas_width = pen_x.next_power_of_two().clamp(ATLAS_MIN_SIZE, ATLAS_MAX_SIZE);
        let atlas_height = max_height.next_power_of_two().clamp(ATLAS_MIN_SIZE, ATLAS_MAX_SIZE);

        let mut atlas_data = vec![0u8; (atlas_width * atlas_height) as usize];
        let mut char_info = Vec::new();
        pen_x = 0;

        for c in entry.text.chars() {
            let (metrics, bitmap) = self.font.rasterize(c, entry.font_size);
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

            char_info.push((pen_x, w, h, metrics));
            pen_x += w + GLYPH_PADDING;
        }

        let mut vertices = Vec::new();
        let mut cursor_x = entry.screen_x;

        for (i, _c) in entry.text.chars().enumerate() {
            let (atlas_x, w, h, metrics) = char_info[i];
            let w_f = w as f32;
            let h_f = h as f32;

            let left = cursor_x + metrics.xmin as f32;
            let right = left + w_f;
            let top = entry.screen_y_baseline - (metrics.ymin as f32 + h_f);
            let bottom = entry.screen_y_baseline - metrics.ymin as f32;

            let [ndc_left, ndc_top] = screen_to_ndc(left, top);
            let [ndc_right, ndc_bottom] = screen_to_ndc(right, bottom);

            let uv_left = atlas_x as f32 / atlas_width as f32;
            let uv_right = (atlas_x + w) as f32 / atlas_width as f32;
            let uv_top = 0.0;
            let uv_bottom = h_f / atlas_height as f32;

            let color = entry.color;
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

            cursor_x += metrics.advance_width;
        }

        Some((vertices, atlas_data, atlas_width, atlas_height))
    }

    fn upload_atlas(
        &self,
        ctx: &GraphicsContext,
        atlas_data: &[u8],
        width: u32,
        height: u32,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let device = ctx.device();
        let queue = ctx.queue();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });

        queue.write_texture(
            texture.as_image_copy(),
            atlas_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(width),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        (texture, bind_group)
    }

    /// Upload vertex data to the GPU text buffer.
    pub fn upload(&self, ctx: &GraphicsContext, vertices: &[TextVertex]) {
        if !vertices.is_empty() {
            ctx.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    pub fn set_vertex_buffer<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
    }

    pub fn pipeline(&self) -> &wgpu::RenderPipeline {
        &self.pipeline
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_vertex_size_matches_layout() {
        let expected = std::mem::size_of::<[f32; 2]>()  // position
            + std::mem::size_of::<[f32; 2]>()            // tex_coords
            + std::mem::size_of::<[f32; 4]>();            // color
        assert_eq!(std::mem::size_of::<TextVertex>(), expected);
    }

    #[test]
    fn text_vertex_is_32_bytes() {
        assert_eq!(std::mem::size_of::<TextVertex>(), 32);
    }
}

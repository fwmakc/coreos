//! GPU text renderer: pipeline, atlas cache, draw calls.

use std::collections::HashMap;

use tracing::warn;

use crate::graphics::GraphicsContext;

use super::atlas::{self, AtlasKey, AtlasMetrics};
use super::{TextEntry, TextVertex};

const MAX_TEXT_VERTICES: usize = 6 * 1024;

struct CachedAtlas {
    bind_group: wgpu::BindGroup,
    #[allow(dead_code)]
    texture: wgpu::Texture,
    atlas_width: u32,
    atlas_height: u32,
    glyphs: Vec<atlas::GlyphInfo>,
    frame: u64,
}

struct PendingDrawCall {
    key: AtlasKey,
    vertex_offset: u32,
    vertex_count: u32,
}

/// Renders strings via a dynamically-built R8Unorm glyph atlas.
///
/// Atlases are cached across frames. Call [`prepare()`](Self::prepare) to
/// build vertex data, then [`upload()`](Self::upload) to send it to the GPU,
/// and finally [`render()`](Self::render) inside a render pass.
pub struct TextRenderer {
    font: fontdue::Font,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    sampler: wgpu::Sampler,
    vertex_buffer: wgpu::Buffer,
    cache: HashMap<AtlasKey, CachedAtlas>,
    pending: Vec<PendingDrawCall>,
    frame: u64,
}

impl TextRenderer {
    pub fn new(ctx: &GraphicsContext) -> Self {
        let device = ctx.device();
        let format = ctx.format();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../assets/text.wgsl").into()),
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
            cache: HashMap::new(),
            pending: Vec::new(),
            frame: 0,
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

    fn upload_atlas(
        &self,
        ctx: &GraphicsContext,
        metrics: &AtlasMetrics,
    ) -> (wgpu::Texture, wgpu::BindGroup) {
        let device = ctx.device();
        let queue = ctx.queue();

        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Text Atlas"),
            size: wgpu::Extent3d {
                width: metrics.width,
                height: metrics.height,
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
            &metrics.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(metrics.width),
                rows_per_image: Some(metrics.height),
            },
            wgpu::Extent3d {
                width: metrics.width,
                height: metrics.height,
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

    /// Prepare GPU resources for a slice of text entries.
    ///
    /// Cached atlases are reused when text content has not changed.
    /// Returns vertex data to be uploaded via [`upload()`](Self::upload).
    pub fn prepare(
        &mut self,
        ctx: &GraphicsContext,
        entries: &[TextEntry<'_>],
        screen_to_ndc: impl Fn(f32, f32) -> [f32; 2],
    ) -> Vec<TextVertex> {
        self.frame += 1;
        let current_frame = self.frame;
        self.pending.clear();

        let mut all_vertices = Vec::new();

        for entry in entries {
            if entry.text.is_empty() {
                continue;
            }

            let key = AtlasKey::new(entry.text, entry.font_size);

            let needs_insert = !self.cache.contains_key(&key);
            if needs_insert {
                let metrics = atlas::rasterize_atlas(&self.font, entry.text, entry.font_size);
                let (texture, bind_group) = self.upload_atlas(ctx, &metrics);
                self.cache.insert(
                    key.clone(),
                    CachedAtlas {
                        texture,
                        bind_group,
                        atlas_width: metrics.width,
                        atlas_height: metrics.height,
                        glyphs: metrics.glyphs,
                        frame: 0,
                    },
                );
            }

            let cached = self.cache.get_mut(&key).expect("just inserted");
            cached.frame = current_frame;

            let offset = all_vertices.len() as u32;
            let vertices = atlas::build_vertices(
                &cached.glyphs,
                cached.atlas_width,
                cached.atlas_height,
                entry.screen_x,
                entry.screen_y_baseline,
                entry.color,
                &screen_to_ndc,
            );

            if all_vertices.len() + vertices.len() > MAX_TEXT_VERTICES {
                warn!(
                    "text vertex buffer full at {} vertices, dropping text entry",
                    all_vertices.len()
                );
                continue;
            }

            let count = vertices.len() as u32;
            all_vertices.extend(vertices);
            self.pending.push(PendingDrawCall {
                key,
                vertex_offset: offset,
                vertex_count: count,
            });
        }

        self.cache.retain(|_, v| v.frame == current_frame);

        all_vertices
    }

    /// Upload vertex data to the GPU text buffer.
    pub fn upload(&self, ctx: &GraphicsContext, vertices: &[TextVertex]) {
        if !vertices.is_empty() {
            ctx.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(vertices));
        }
    }

    /// Render all pending text draw calls into the given render pass.
    ///
    /// Must be called after [`prepare()`](Self::prepare) and [`upload()`](Self::upload).
    pub fn render(&self, pass: &mut wgpu::RenderPass<'_>) {
        if self.pending.is_empty() {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        for call in &self.pending {
            let cached = &self.cache[&call.key];
            pass.set_bind_group(0, &cached.bind_group, &[]);
            pass.draw(
                call.vertex_offset..call.vertex_offset + call.vertex_count,
                0..1,
            );
        }
    }
}

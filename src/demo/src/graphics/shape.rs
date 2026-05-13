//! Shape renderer: cursor, circles, and UI panels via a single GPU pipeline.

use tracing::warn;

use super::GraphicsContext;

const MAX_SHAPES: usize = 10_000;
const VERTICES_PER_QUAD: usize = 6;
const MAX_VERTICES: usize = VERTICES_PER_QUAD * MAX_SHAPES;

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ShapeVertex {
    position: [f32; 2],
    center: [f32; 2],
    radius: f32,
    aspect: f32,
    color: [f32; 4],
}

/// A drawable shape instance.
#[derive(Clone, Copy, Debug)]
pub enum Shape {
    /// Filled rectangle (no discard).
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        color: [f32; 4],
    },
    /// Circle drawn with discard in fragment shader.
    Circle {
        x: f32,
        y: f32,
        radius: f32,
        color: [f32; 4],
    },
}

struct QuadBounds {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
    color: [f32; 4],
    center: [f32; 2],
    radius: f32,
    aspect: f32,
}

/// GPU pipeline and buffer for shape quads.
pub struct ShapeRenderer {
    pipeline: wgpu::RenderPipeline,
    vertex_buffer: wgpu::Buffer,
    vertices: Vec<ShapeVertex>,
}

impl ShapeRenderer {
    pub fn new(ctx: &GraphicsContext) -> Self {
        let device = ctx.device();
        let format = ctx.format();

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shape Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../assets/shape.wgsl").into()),
        });

        let layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Shape Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Shape Pipeline"),
            layout: Some(&layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<ShapeVertex>() as wgpu::BufferAddress,
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
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 5]>() as wgpu::BufferAddress,
                            shader_location: 3,
                            format: wgpu::VertexFormat::Float32,
                        },
                        wgpu::VertexAttribute {
                            offset: std::mem::size_of::<[f32; 6]>() as wgpu::BufferAddress,
                            shader_location: 4,
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

        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Shape Vertex Buffer"),
            size: (std::mem::size_of::<ShapeVertex>() * MAX_VERTICES) as wgpu::BufferAddress,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        Self {
            pipeline,
            vertex_buffer,
            vertices: Vec::new(),
        }
    }

    /// Clear the frame shape list.
    pub fn clear(&mut self) {
        self.vertices.clear();
    }

    /// Push a shape to be drawn this frame.
    pub fn push(&mut self, shape: &Shape, screen_to_ndc: impl Fn(f32, f32) -> [f32; 2]) {
        if self.vertices.len() + VERTICES_PER_QUAD > MAX_VERTICES {
            warn!("shape vertex buffer full, dropping shape");
            return;
        }
        match *shape {
            Shape::Rect { x, y, w, h, color } => {
                let left = screen_to_ndc(x, y + h)[0];
                let right = screen_to_ndc(x + w, y + h)[0];
                let top = screen_to_ndc(x, y)[1];
                let bottom = screen_to_ndc(x, y + h)[1];
                self.push_quad(QuadBounds {
                    left,
                    right,
                    top,
                    bottom,
                    color,
                    center: [0.0; 2],
                    radius: 0.0,
                    aspect: 0.0,
                });
            }
            Shape::Circle {
                x,
                y,
                radius,
                color,
            } => {
                let [cx, cy] = screen_to_ndc(x, y);
                let [rx, _] = screen_to_ndc(x + radius, y);
                let [_, ry] = screen_to_ndc(x, y + radius);
                let r_ndc_x = rx - cx;
                let r_ndc_y = cy - ry;
                let aspect = r_ndc_x / r_ndc_y;
                self.push_quad(QuadBounds {
                    left: cx - r_ndc_x,
                    right: cx + r_ndc_x,
                    top: cy + r_ndc_y,
                    bottom: cy - r_ndc_y,
                    color,
                    center: [cx, cy],
                    radius: r_ndc_x,
                    aspect,
                });
            }
        }
    }

    fn push_quad(&mut self, b: QuadBounds) {
        let v = |x, y| ShapeVertex {
            position: [x, y],
            center: b.center,
            radius: b.radius,
            aspect: b.aspect,
            color: b.color,
        };
        self.vertices.extend_from_slice(&[
            v(b.left, b.bottom),
            v(b.right, b.bottom),
            v(b.left, b.top),
            v(b.right, b.bottom),
            v(b.right, b.top),
            v(b.left, b.top),
        ]);
    }

    /// Upload accumulated shapes to the GPU.
    pub fn upload(&self, ctx: &GraphicsContext) {
        if !self.vertices.is_empty() {
            ctx.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&self.vertices));
        }
    }

    /// Record draw commands into the render pass.
    pub fn render<'a>(&'a self, pass: &mut wgpu::RenderPass<'a>) {
        let count = self.vertices.len() as u32;
        if count == 0 {
            return;
        }
        pass.set_pipeline(&self.pipeline);
        pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        pass.draw(0..count, 0..1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shape_vertex_size_matches_layout() {
        let expected = std::mem::size_of::<[f32; 2]>()  // position
            + std::mem::size_of::<[f32; 2]>()            // center
            + std::mem::size_of::<f32>()                  // radius
            + std::mem::size_of::<f32>()                  // _pad
            + std::mem::size_of::<[f32; 4]>(); // color
        assert_eq!(std::mem::size_of::<ShapeVertex>(), expected);
    }

    #[test]
    fn shape_vertex_is_40_bytes() {
        assert_eq!(std::mem::size_of::<ShapeVertex>(), 40);
    }
}

//! Phase 0 — Playable Demo (refactored)
//!
//! Modular interactive prototype: window, wgpu rendering, cursor, clicks, text.
//! Run with: `cargo run --bin demo`

use std::time::Instant;

use tracing::{info, warn};
use winit::application::ApplicationHandler;
use winit::dpi::PhysicalSize;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};

mod app;
mod graphics;
mod input;

use app::{
    AppState, CommandResult, CMD_FONT_SIZE, CMD_PANEL_COLOR, CMD_PANEL_HEIGHT, CURSOR_COLOR,
    FONT_SIZE, HELP_TEXT, TEXT_COLOR,
};

use graphics::shape::{Shape, ShapeRenderer};
use graphics::text::{TextEntry, TextRenderer};
use graphics::GraphicsContext;
use input::{Command, InputHandler};

const BRAND_COLOR: wgpu::Color = wgpu::Color {
    r: 0x0a as f64 / 255.0,
    g: 0x0e as f64 / 255.0,
    b: 0x1a as f64 / 255.0,
    a: 1.0,
};

/// Convert screen pixels to Normalized Device Coordinates.
fn screen_to_ndc(x: f32, y: f32, w: f32, h: f32) -> [f32; 2] {
    [(x / w) * 2.0 - 1.0, 1.0 - (y / h) * 2.0]
}

struct GpuResources {
    ctx: GraphicsContext,
    shapes: ShapeRenderer,
    text_renderer: TextRenderer,
}

struct DemoApp {
    gpu: Option<GpuResources>,
    app: AppState,
    input: InputHandler,
    last_frame_time: Instant,
    frame_count: u32,
}

impl DemoApp {
    fn new() -> Self {
        Self {
            gpu: None,
            app: AppState::default(),
            input: InputHandler::default(),
            last_frame_time: Instant::now(),
            frame_count: 0,
        }
    }

    fn init_gpu(&mut self, window: winit::window::Window) {
        let ctx = pollster::block_on(GraphicsContext::new(window));
        let shapes = ShapeRenderer::new(&ctx);
        let text_renderer = TextRenderer::new(&ctx);
        self.gpu = Some(GpuResources {
            ctx,
            shapes,
            text_renderer,
        });
    }

    fn handle_command(&mut self, cmd: Command, elwt: &ActiveEventLoop) {
        match cmd {
            Command::Exit => {
                info!("exiting gracefully");
                elwt.exit();
            }
            Command::ToggleCommandBar => {
                self.app.toggle_command_bar();
                info!("command bar toggled: {}", self.app.command_bar_visible);
            }
            Command::ToggleHelp => {
                self.app.help_visible = !self.app.help_visible;
                info!("help overlay toggled");
            }
            Command::Type(ch, target) => {
                self.app.type_char(ch, target);
            }
            Command::Backspace(target) => {
                self.app.backspace(target);
            }
            Command::HistoryUp => {
                self.app.history_up();
            }
            Command::HistoryDown => {
                self.app.history_down();
            }
            Command::Execute => {
                if let Some(result) = self.app.execute_command() {
                    match &result {
                        CommandResult::Exit => {
                            info!("command: exit");
                            elwt.exit();
                        }
                        CommandResult::ClearCircles => {
                            self.app.clear_circles();
                            info!("command: circles cleared");
                        }
                        CommandResult::ResizeCursor(size) => {
                            info!("command: cursor resized to {:.0}", size);
                        }
                        CommandResult::ShowHelp => {
                            info!("command: help");
                        }
                        CommandResult::Unknown(cmd) => {
                            info!("unknown command: {}", cmd);
                        }
                    }
                }
            }
            Command::AddCircle => {
                if self.app.add_circle() {
                    info!(
                        "circle added at ({:.0}, {:.0}), total {}",
                        self.app.cursor_pos_x(),
                        self.app.cursor_pos_y(),
                        self.app.circles().len()
                    );
                }
            }
            Command::ClearCircles => {
                self.app.clear_circles();
                info!("circles cleared");
            }
            Command::ResizeCursor(delta) => {
                self.app.resize_cursor(delta);
            }
            Command::CursorMoved(x, y) => {
                self.app.set_cursor_pos(x, y);
            }
            Command::WindowResized(w, h) => {
                if let Some(gpu) = &mut self.gpu {
                    gpu.ctx.resize(PhysicalSize::new(w, h));
                }
            }
        }
    }

    fn update_fps(&mut self) {
        self.frame_count += 1;
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_frame_time).as_secs_f64();
        if elapsed >= 1.0 {
            let fps = self.frame_count as f64 / elapsed;
            if let Some(gpu) = &self.gpu {
                gpu.ctx
                    .window()
                    .set_title(&format!("Workspace Demo — {:.0} FPS", fps));
            }
            self.frame_count = 0;
            self.last_frame_time = now;
        }
    }

    fn build_and_render(&mut self) {
        let gpu = self.gpu.as_mut().expect("gpu not initialized");
        let (w, h) = gpu.ctx.window_size_f32();
        let to_ndc = |x, y| screen_to_ndc(x, y, w, h);

        gpu.shapes.clear();
        build_shapes(&mut gpu.shapes, &self.app, w, h, &to_ndc);
        gpu.shapes.upload(&gpu.ctx);

        let text_entries = build_text_entries(&self.app, h);
        let text_vertices = gpu.text_renderer.prepare(&gpu.ctx, &text_entries, to_ndc);
        gpu.text_renderer.upload(&gpu.ctx, &text_vertices);

        render_frame(&mut gpu.ctx, &gpu.shapes, &gpu.text_renderer);

        gpu.ctx.request_redraw();
    }
}

impl ApplicationHandler for DemoApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.gpu.is_none() {
            let window_attrs = winit::window::WindowAttributes::default()
                .with_title("Workspace Demo")
                .with_inner_size(PhysicalSize::new(1280, 720));
            let window = match event_loop.create_window(window_attrs) {
                Ok(w) => w,
                Err(e) => {
                    tracing::error!("failed to create window: {}", e);
                    event_loop.exit();
                    return;
                }
            };
            self.init_gpu(window);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: winit::window::WindowId,
        event: winit::event::WindowEvent,
    ) {
        let ctx = match &self.gpu {
            Some(g) => g.ctx.window(),
            None => return,
        };
        if window_id != ctx.id() {
            return;
        }

        let commands = self.input.handle(&event, self.app.command_bar_visible);
        for cmd in commands {
            self.handle_command(cmd, event_loop);
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.update_fps();
        self.build_and_render();
    }
}

fn build_shapes(
    shapes: &mut ShapeRenderer,
    app: &AppState,
    w: f32,
    h: f32,
    to_ndc: &dyn Fn(f32, f32) -> [f32; 2],
) {
    let half = app.cursor_size() / 2.0;
    let cx = app.cursor_pos_x() as f32;
    let cy = app.cursor_pos_y() as f32;
    shapes.push(
        &Shape::Rect {
            x: cx - half,
            y: cy - half,
            w: app.cursor_size(),
            h: app.cursor_size(),
            color: CURSOR_COLOR,
        },
        to_ndc,
    );

    for circle in app.circles() {
        shapes.push(
            &Shape::Circle {
                x: circle.x,
                y: circle.y,
                radius: circle.radius,
                color: circle.color,
            },
            to_ndc,
        );
    }

    if app.command_bar_visible {
        shapes.push(
            &Shape::Rect {
                x: 0.0,
                y: h - CMD_PANEL_HEIGHT,
                w,
                h: CMD_PANEL_HEIGHT,
                color: CMD_PANEL_COLOR,
            },
            to_ndc,
        );
    }
}

fn build_text_entries<'a>(app: &'a AppState, h: f32) -> Vec<TextEntry<'a>> {
    let mut entries = Vec::new();

    if !app.typed_text().is_empty() {
        entries.push(TextEntry {
            text: app.typed_text(),
            font_size: FONT_SIZE,
            screen_x: 20.0,
            screen_y_baseline: 60.0,
            color: TEXT_COLOR,
        });
    }

    if app.help_visible {
        entries.push(TextEntry {
            text: HELP_TEXT,
            font_size: CMD_FONT_SIZE,
            screen_x: 20.0,
            screen_y_baseline: h - CMD_PANEL_HEIGHT - 10.0,
            color: [0.6, 0.8, 1.0, 1.0],
        });
    }

    if app.command_bar_visible && !app.command_text().is_empty() {
        entries.push(TextEntry {
            text: app.command_text(),
            font_size: CMD_FONT_SIZE,
            screen_x: 10.0,
            screen_y_baseline: h - (CMD_PANEL_HEIGHT / 2.0) + (CMD_FONT_SIZE / 2.0),
            color: TEXT_COLOR,
        });
    }

    entries
}

fn render_frame(ctx: &mut GraphicsContext, shapes: &ShapeRenderer, text_renderer: &TextRenderer) {
    let output = match ctx.acquire_frame() {
        Ok(f) => f,
        Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
            ctx.resize(ctx.size());
            return;
        }
        Err(e) => {
            warn!("surface error: {}", e);
            return;
        }
    };

    let view = output
        .texture
        .create_view(&wgpu::TextureViewDescriptor::default());

    let mut encoder = ctx.create_encoder("Render Encoder");
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Demo Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(BRAND_COLOR),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        shapes.render(&mut pass);
        text_renderer.render(&mut pass);
    }

    ctx.submit(std::iter::once(encoder.finish()));
    output.present();
}

fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    info!("Workspace Phase 0 — Playable Demo starting...");

    let event_loop = EventLoop::new().unwrap();
    event_loop.set_control_flow(ControlFlow::Wait);

    let mut demo = DemoApp::new();
    event_loop.run_app(&mut demo).expect("event loop failed");

    info!("Workspace Demo exited gracefully");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_to_ndc_center() {
        let [x, y] = screen_to_ndc(640.0, 360.0, 1280.0, 720.0);
        assert!((x - 0.0).abs() < 0.001);
        assert!((y - 0.0).abs() < 0.001);
    }

    #[test]
    fn screen_to_ndc_top_left() {
        let [x, y] = screen_to_ndc(0.0, 0.0, 800.0, 600.0);
        assert!((x - (-1.0)).abs() < 0.001);
        assert!((y - 1.0).abs() < 0.001);
    }

    #[test]
    fn screen_to_ndc_bottom_right() {
        let [x, y] = screen_to_ndc(800.0, 600.0, 800.0, 600.0);
        assert!((x - 1.0).abs() < 0.001);
        assert!((y - (-1.0)).abs() < 0.001);
    }

    #[test]
    fn screen_to_ndc_half() {
        let [x, _y] = screen_to_ndc(200.0, 0.0, 800.0, 600.0);
        assert!((x - (-0.5)).abs() < 0.001);
    }
}

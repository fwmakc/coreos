# Этап 1 — Фундамент (Host Shim + Display Server)

> **Цель:** Система запускается как полноэкранное окно (или оконное), рендерит базовые UI-примитивы через WebGPU, захватывает ввод (клавиатура, мышь, тач). Host Shim абстрагирует хост-ОС.

**После этого этапа:** есть чёрное окно с тестовым UI (прямоугольники, текст). Можно двигать мышью — курсор отрисовывается. Можно нажимать клавиши — они логируются в консоль.

---

## Зависимости

Нет внутренних зависимостей от CORE OS. Внешние зависимости:
- Rust toolchain (stable)
- winit (окно и ввод)
- wgpu (WebGPU)
- raw-window-handle (интеграция winit + wgpu)
- glyphon или аналог (рендеринг текста через WebGPU)
- cpal (аудио)
- nokhwa или аналог (камера, опционально)

---

## Компоненты

### 1.1 Host Shim (Level 0, Rust)

**Host Shim** — адаптер между CORE OS и хост-ОС. Не содержит бизнес-логики, только plumbing.

#### 1.1.1 Окно и ввод

**winit integration:**
```rust
// host_shim/src/window.rs
use winit::event_loop::EventLoop;
use winit::window::WindowBuilder;

pub struct HostWindow {
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    scale_factor: f64,
}

impl HostWindow {
    pub fn new(title: &str, width: u32, height: u32) -> Self;
    pub fn run<F>(&mut self, callback: F) where F: FnMut(HostEvent);
}

pub enum HostEvent {
    Keyboard { key: KeyCode, state: Pressed/Released, modifiers: Modifiers },
    MouseMove { x: f64, y: f64 },
    MouseButton { button: Left/Right/Middle, state: Pressed/Released },
    MouseWheel { delta_x: f64, delta_y: f64 },
    Touch { id: u64, phase: Start/Move/End/Cancel, x: f64, y: f64 },
    Resized { width: u32, height: u32 },
    ScaleFactorChanged { scale_factor: f64 },
    CloseRequested,
}
```

**Требования:**
- Оконный режим: обычное окно с заголовком, ресайз, минимизация
- Полноэкранный режим: borderless fullscreen (не exclusive, чтобы не ломать alt+tab хост-ОС)
- Масштабирование: поддержка DPI (scale factor от winit)
- Мульти-мониторы: `EventLoop` слушает все мониторы, `Window` может перемещаться

#### 1.1.2 GPU (WebGPU surface)

**wgpu integration:**
```rust
// host_shim/src/gpu.rs
pub struct GpuContext {
    instance: wgpu::Instance,
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
}

impl GpuContext {
    pub async fn new(window: &winit::window::Window) -> Self;
    pub fn resize(&mut self, width: u32, height: u32);
    pub fn present(&mut self, encoder: wgpu::CommandEncoder);
    pub fn create_texture(&self, desc: &wgpu::TextureDescriptor) -> wgpu::Texture;
}
```

**Требования:**
- Backend: primary — Vulkan (Linux/Windows), Metal (macOS), DX12 (Windows fallback)
- Surface: `SurfaceConfiguration` с `present_mode: Fifo` (60 FPS)
- Features: `TEXTURE_BINDING_ARRAY` (для batch rendering), `PUSH_CONSTANTS`
- Limits: минимум `downlevel_defaults()`

#### 1.1.3 Аудио

**cpal integration:**
```rust
// host_shim/src/audio.rs
pub struct AudioHost {
    output_stream: cpal::Stream,
    input_stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
}

impl AudioHost {
    pub fn new() -> Result<Self, AudioError>;
    pub fn play(&mut self, buffer: &[f32]);
    pub fn record<F>(&mut self, callback: F) where F: FnMut(&[f32]);
}
```

**Требования:**
- Output: стерео, 48 kHz (конфигурируется)
- Input: моно, 16 kHz (для Whisper)
- Формат: `f32` нормализованный (-1.0 .. 1.0)

#### 1.1.4 Файловая система (VFS bridge)

```rust
// host_shim/src/fs.rs
pub trait VfsBridge {
    fn read(&self, path: &str) -> Result<Vec<u8>, FsError>;
    fn write(&self, path: &str, data: &[u8]) -> Result<(), FsError>;
    fn list(&self, path: &str) -> Result<Vec<DirEntry>, FsError>;
    fn metadata(&self, path: &str) -> Result<Metadata, FsError>;
    fn watch(&self, path: &str, callback: Box<dyn Fn(WatchEvent)>) -> Result<WatchHandle, FsError>;
}
```

**Требования:**
- Путь абсолютный или относительно `~/.core/` (хост-ОС)
- `watch` — native file system watcher (notify crate)
- Права: чтение/запись без root (пользовательская директория)

#### 1.1.5 Сеть

```rust
// host_shim/src/net.rs
pub struct NetworkHost {
    // WireGuard будет на этапе 5, сейчас — базовые сокеты
}

impl NetworkHost {
    pub fn tcp_connect(&self, addr: &str) -> Result<TcpStream, NetError>;
    pub fn udp_socket(&self, bind_addr: &str) -> Result<UdpSocket, NetError>;
    pub fn local_ip(&self) -> Result<String, NetError>;
}
```

**Требования:**
- TCP/UDP сокеты через стандартную библиотеку Rust
- DNS-резолвинг
- Локальный IP для будущего P2P

### 1.2 Display Server (Level 3, Rust + WebGPU)

Display Server рендерит весь UI CORE OS. На этом этапе — только примитивы.

#### 1.2.1 Render Pipeline

```rust
// display_server/src/renderer.rs
pub struct Renderer {
    gpu: GpuContext,
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    texture_atlas: TextureAtlas,
    text_renderer: TextRenderer, // glyphon
}

impl Renderer {
    pub fn new(gpu: GpuContext) -> Self;
    pub fn begin_frame(&mut self) -> Frame;
    pub fn end_frame(&mut self, frame: Frame);
}

pub struct Frame {
    encoder: wgpu::CommandEncoder,
    view: wgpu::TextureView,
}
```

**Vertex format:**
```rust
#[repr(C)]
struct Vertex {
    position: [f32; 2],  // x, y в пикселях
    uv: [f32; 2],        // текстурные координаты
    color: [f32; 4],     // RGBA
}
```

**Uniforms (per draw call):**
```rust
struct DrawUniforms {
    transform: [[f32; 4]; 4], // orthographic projection
    opacity: f32,
    _padding: [f32; 3],
}
```

#### 1.2.2 Примитивы

```rust
// display_server/src/primitives.rs
pub enum Primitive {
    Rectangle {
        x: f32, y: f32, w: f32, h: f32,
        color: Color,
        radius: [f32; 4], // скругление углов
    },
    Text {
        x: f32, y: f32,
        text: String,
        font_size: f32,
        color: Color,
        max_width: Option<f32>,
    },
    Image {
        x: f32, y: f32, w: f32, h: f32,
        texture_id: TextureId,
        opacity: f32,
    },
    Border {
        x: f32, y: f32, w: f32, h: f32,
        color: Color,
        width: f32,
    },
}
```

**Требования:**
- Rectangle: поддержка скругления углов (шейдер с SDF)
- Text: рендеринг через glyphon (wgpu-based текст)
- Image: загрузка PNG/JPEG через `image` crate, текстурный атлас
- Batch rendering: все примитивы одного типа в один draw call

#### 1.2.3 Scene Graph

```rust
// display_server/src/scene.rs
pub struct Scene {
    nodes: Vec<SceneNode>,
    z_index: Vec<usize>, // sorted by z
}

pub struct SceneNode {
    id: NodeId,
    primitives: Vec<Primitive>,
    transform: Transform, // translate, scale, rotate
    clip_rect: Option<Rect>,
    opacity: f32,
}
```

**Требования:**
- Z-index сортировка перед рендерингом
- Clipping: scissor rect в WebGPU
- Opacity: alpha blending ( premultiplied alpha )
- Transform: только translate (для начала), scale и rotate — опционально

### 1.3 Интеграция Host Shim + Display Server

```rust
// src/main.rs (entry point)
#[tokio::main]
async fn main() {
    let mut window = HostWindow::new("CORE OS", 1280, 720);
    let gpu = GpuContext::new(&window.window()).await;
    let mut display = Renderer::new(gpu);
    
    window.run(|event| {
        match event {
            HostEvent::Resized { w, h } => display.resize(w, h),
            HostEvent::Keyboard { key, .. } => println!("Key: {:?}", key),
            HostEvent::MouseMove { x, y } => println!("Mouse: {}, {}", x, y),
            _ => {}
        }
        
        // Тестовая сцена
        let mut frame = display.begin_frame();
        frame.draw_rectangle(100.0, 100.0, 200.0, 100.0, Color::BLUE, [8.0; 4]);
        frame.draw_text(120.0, 150.0, "CORE OS", 24.0, Color::WHITE);
        display.end_frame(frame);
    });
}
```

---

## Шаги реализации

### Шаг 1.1: Настройка проекта Rust (3 дня)

1. Создать workspace `src/rust/` с crates:
   - `host_shim/` — Level 0
   - `display_server/` — Level 3
   - `core_os/` — entry point
2. `Cargo.toml` — зависимости: `winit`, `wgpu`, `tokio`, `glyphon`, `cpal`, `image`, `notify`
3. Настроить CI: `cargo check`, `cargo clippy`, `cargo test`
4. Скрипт сборки: `cargo build --release`

### Шаг 1.2: HostWindow + события (4 дня)

1. Реализовать `HostWindow` с `winit::EventLoop`
2. Обработка всех `HostEvent` (keyboard, mouse, touch, resize, close)
3. DPI scaling: корректное преобразование логических → физических пикселей
4. Тест: логировать все события в консоль

### Шаг 1.3: GpuContext (4 дня)

1. Создание `wgpu::Instance`, `Surface`, `Adapter`, `Device`, `Queue`
2. `SurfaceConfiguration`: формат (BGRA8_UNORM), present mode (Fifo), alpha mode
3. `resize`: пересоздание surface texture
4. `present`: `CommandEncoder` → `Queue::submit` → `Surface::present`
5. Тест: очистка экрана цветом (clear color)

### Шаг 1.4: Render Pipeline + примитивы (7 дней)

1. Шейдеры (WGSL):
   - `primitive.wgsl` — vertex + fragment для прямоугольников (SDF rounded corners)
   - `text.wgsl` — vertex + fragment для текста (glyphon integration)
2. Vertex buffer, index buffer
3. Uniform buffer (ortho projection matrix)
4. Texture atlas (PNG/JPEG → wgpu::Texture)
5. Реализация всех `Primitive` типов
6. Batch rendering (инстансинг или merged geometry)
7. Тест: отрисовка сцены со всеми типами примитивов

### Шаг 1.5: Scene Graph (3 дней)

1. `SceneNode` с transform, clip, opacity
2. Z-index сортировка (стабильная)
3. Clipping через `RenderPass::set_scissor_rect`
4. Alpha blending через `BlendState::ALPHA_BLENDING`
5. Тест: наложение полупрозрачных прямоугольников

### Шаг 1.6: Аудио bridge (2 дня)

1. cpal: перечисление устройств, создание output stream
2. Ring buffer для аудио данных
3. Тест: проигрывание sine wave

### Шаг 1.7: ФС bridge (2 дня)

1. Реализация `VfsBridge` через `std::fs`
2. `notify` для file watcher
3. Тест: чтение/запись файла, watcher callback

### Шаг 1.8: Интеграция и smoke test (3 дня)

1. `core_os/src/main.rs` — запуск EventLoop + Renderer
2. Тестовая сцена: 10 прямоугольников, 5 текстовых label, 1 изображение
3. Интерактив: мышь подсвечивает прямоугольник под курсором
4. Performance test: 60 FPS при 1000 примитивах на 1920x1080

---

## Критерии приёмки

- [ ] Система запускается как окно (1280x720 или fullscreen) на Windows, macOS, Linux
- [ ] Рендеринг 60 FPS (verified через `window.request_redraw` + `Instant::elapsed`)
- [ ] Отрисовка: прямоугольники (со скруглением), текст, изображения, бордеры
- [ ] Ввод: клавиатура (все key codes), мышь (move, click, wheel), тач (опционально)
- [ ] DPI scaling: UI не размывается на HiDPI (2x, 3x)
- [ ] Аудио: sine wave проигрывается без артефактов
- [ ] ФС: чтение/запись + watcher работают
- [ ] Нет memory leaks (проверка через valgrind / leaks на macOS)

---

## Placeholder'ы (будут заменены в следующих этапах)

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Текстовый рендеринг — только латиница | Этап 2 | glyphon с кириллицей |
| Нет анимаций | Этап 2 | Tween engine |
| Нет blur / shadows | Этап 3 | Post-processing effects |
| Аудио — только output | Этап 7 | Input (микрофон) для Whisper |
| Сеть — только TCP/UDP сокеты | Этап 5 | WireGuard + libp2p |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Host Shim | layer-8 (до §1), layer-3 (Host Shim раздел) |
| Display Server | layer-8 §3.3, §3.1, §3.6, §3.7 |
| GPU Context | layer-8 §3.3 (WebGPU окна), layer-1 (UX визуал) |
| Audio Host | layer-8 §3.6 (low-latency audio), §7.1 (Whisper audio) |
| VFS Bridge | layer-8 §VFS, layer-5 (Mirror Engine) |

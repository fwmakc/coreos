# Этап 9 — Display Server: Core

## Цель
Создать ядро Display Server — подсистемы, которая управляет GPU, swapchain, и базовым рендерингом через WebGPU. После этого этапа Workspace умеет рендерить простейшие примитивы (треугольники, прямоугольники) в окно Host Shim с 60 FPS.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `wgpu` (WebGPU native), `raw-window-handle` (интеграция с winit), `pollster` (синхронный рендер-цикл на старте, позже async)
- **Шейдеры:** WGSL (WebGPU Shading Language)
- **Целевые ОС:** Windows (DirectX 12 / Vulkan), macOS (Metal), Linux (Vulkan), Android (Vulkan)

## Зависимости
- **Этап 1–3** — Host Shim: Window (surface handle, swapchain integration).
- **Этап 8** — Host Shim: Network (не критично, но reference по архитектуре).

## Часть системы
**Level 3 — Display Server** [См. layer-8 §3, layer-1 §3, layer-9 §2]

Display Server — это полноценный композитор, аналогичный Wayland compositor или macOS WindowServer, но использующий WebGPU вместо OpenGL/Metal/DirectX. Он рендерит всё: окна приложений, Command Bar, анимации, эффекты.

## Требования

### 7.1 GPU инициализация
- Перечисление адаптеров (GPU) через `wgpu::Instance::enumerate_adapters`.
- Выбор адаптера по приоритету: дискретный GPU > интегрированный > программный (fallback).
- Создание `Device` и `Queue` с лимитами, совместимыми с WebGPU core spec.
- **Fallback:** если WebGPU недоступен — программный рендеринг через `wgpu` с бэкендом `noop` не работает, поэтому fallback — минимальный Vulkan/Metal/DirectX через `angle` или `lavapipe` (software Vulkan).

### 7.2 Swapchain и surface
- Создание surface из `winit::window::Window` через `unsafe { instance.create_surface_unsafe(...) }` (или safe API в новых версиях wgpu).
- Конфигурация surface: формат (Bgra8UnormSrgb), цветовое пространство (sRGB), present mode:
  - `AutoVsync` — по умолчанию (60 FPS, без tearing).
  - `AutoNoVsync` — Game Mode (максимальный FPS).
- Обработка resize: пересоздание surface configuration.
- Обработка потери surface (minimize, отключение монитора) — пауза рендеринга.

### 7.3 Рендер-цикл
- Синхронный цикл: `loop { render_frame(); sleep_until_next_vsync(); }`.
- Интеграция с event loop Host Shim: рендеринг запускается по событию `RedrawRequested` от winit.
- **Frame budget:** 16.67 мс на кадр. Если рендеринг превышает — логируется `FrameOverrun`, и система переходит в режим пропуска кадров (drop to 30 FPS).

### 7.4 Базовый рендер-пайплайн
- Vertex buffer + Index buffer для треугольника.
- Render pipeline с простейшим WGSL-шейдером (vertex: pass-through, fragment: solid color).
- Uniform buffer для MVP-матрицы (model-view-projection).
- **Гамма-коррекция:** рендеринг в linear color space, вывод в sRGB (через surface format).

### 7.5 Resource management
- Текстурный атлас (базовый): выделение прямоугольных областей в большой 2D-текстуре.
- Буферный пул: переиспользование vertex/uniform буферов между кадрами.
- GPU memory budget: 512 МБ по умолчанию, логирование при превышении.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Green triangle | Базовый рендер | Запуск → зелёный треугольник в окне |
| Resize | Изменение размера | Растянуть окно → треугольник масштабируется |
| 60 FPS | VSync | Запуск → лог показывает ~16.67 мс/кадр |
| GPU adapter | Выбор GPU | Лог содержит название выбранного GPU |
| Multi-monitor | Перемещение окна | Перетащить на другой монитор → рендер продолжается |

## Интеграция с будущими этапами
- **Вход:** этап 1–3 (Host Shim) — window handle, resize events, focus events.
- **Выход:** render texture / framebuffer → этап 8 (Display Server 2D) для отрисовки текста и примитивов.
- **Выход:** GPU device/queue → этап 9 (Compositor) для сложных эффектов.
- **Выход:** surface configuration → этап 27 (Game Mode) для direct GPU context.

## Критерии приёмки
- [ ] Компилируется на Windows (DX12/Vulkan), macOS (Metal), Linux (Vulkan).
- [ ] Зелёный треугольник рендерится в окне.
- [ ] Resize не вызывает panic или артефактов.
- [ ] Frame time ~16.67 мс на рекомендуемом железе (2 ядра, integrated GPU).
- [ ] GPU memory budget логирует использование.
- [ ] Fallback на software Vulkan работает ( lavapipe ), пусть и медленно.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Space, проекты (визуальная основа)
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Display Server §3
- [layer-9-hardware-requirements.md](../layers/layer-9-hardware-requirements.md) — Рекомендуемое железо

# Track 1: Core Runtime

## Цель

Рантайм для JS/TS-приложений с нативной скоростью рендеринга через WebGPU. Замена Electron без DOM/CSS.

---

## Архитектура

```
┌─────────────────────────────────────┐
│  JS/TS Application (V8 Isolate)     │
│  import { View, Text } from '@core' │
├─────────────────────────────────────┤
│  Core Runtime (Bun)                 │
│  - Component tree                   │
│  - State management                 │
│  - IPC bridge                       │
├─────────────────────────────────────┤
│  Host Shim (Rust)                   │
│  - WebGPU window                    │
│  - Input (mouse, keyboard)          │
│  - V8 Isolate lifecycle             │
├─────────────────────────────────────┤
│  Host OS (Windows/Linux/macOS)      │
└─────────────────────────────────────┘
```

---

## Компоненты

### 1. Host Shim (Rust)

Ответственность: окно, графика, ввод, управление изолятами.

#### window.rs — Создание окна
- winit для кроссплатформенного окна
- wgpu для WebGPU контекста
- Рендеринг на Surface при каждом кадре
- Target: 60 FPS, VSync

#### input.rs — Обработка ввода
- Мышь: move, click, scroll, drag
- Клавиатура: keydown, keyup, shortcuts
- Тачпад: pinch, swipe (будущее)
- События передаются в Runtime через mpsc-канал

#### isolate.rs — V8 Isolate Manager
- Создание/уничтожение изолятов
- Выполнение JS-кода внутри изолята
- IPC через SharedArrayBuffer
- Resource quotas (память, CPU)
- TerminateExecution() при превышении

#### renderer.rs — Рендерер
- Получает список draw-команд от Runtime
- Отрисовка через WebGPU: прямоугольники, текст, изображения
- Базовые эффекты: тени, скругление, прозрачность
- Clip-области для окон

### 2. Core Runtime (Bun / TypeScript)

Ответственность: компонентная модель, layout, мост между JS-приложениями и Shim.

#### kernel.ts — Ядро
- Запуск Shim как child-process
- Бинарный IPC с Shim (message pack или custom protocol)
- Управление жизненным циклом приложений
- Роутинг событий ввода → целевое приложение

#### component-tree.ts — Дерево компонентов
- Реактивное дерево (подобие VDOM, но без DOM)
- Diff-алгоритм для минимальных обновлений
- Каждый узел = прямоугольник с позицией, размером, стилями
- Flattening в draw-команды для Shim

#### layout.ts — Layout Engine
- Flexbox (упрощенный, subset)
- Основные свойства: flexDirection, gap, padding, margin, alignItems, justifyContent
- Текстовое измерение (ширина/высота строки)
- Yoga-like подход: calculateLayout(root, width, height) → дерево с координатами

#### components/ — Базовые компоненты
- `View` — контейнер (прямоугольник с фоном, границами, скруглением)
- `Text` — текстовая строка (шрифт, размер, цвет, выравнивание)
- `Button` — интерактивный элемент (hover, active, disabled)
- `Image` — изображение (пока только PNG/JPEG из локальных файлов)
- `TextInput` — поле ввода (курсор, выделение, вставка)
- `ScrollView` — прокручиваемая область

#### ipc.ts — Мост Shim ↔ Runtime
- Бинарный протокол (не JSON)
- Типы сообщений:
  - `DrawCommands` — Runtime → Shim (список прямоугольников, текстов, изображений)
  - `InputEvents` — Shim → Runtime (mouse, keyboard)
  - `SystemCalls` — Runtime → Shim (создать окно, прочитать файл)
  - `AppLifecycle` — Runtime → Shim (создать/убить изолят)

---

## Milestones

### Sprint 1 (Месяц 1): "Пустое окно"

**Цель:** Rust создает окно, WebGPU рисует один цвет, обрабатывает ввод.

- [ ] Cargo.toml с зависимостями (winit, wgpu)
- [ ] Создание окна 1280x720
- [ ] WebGPU Surface + render loop
- [ ] Очистка экрана в цвет (проверка что GPU работает)
- [ ] Обработка мыши и клавиатуры → лог в консоль
- [ ] Graceful shutdown (крестик, Escape)

**Критерий успеха:** Окно открывается, рисует цвет, реагирует на мышь. RAM < 10 МБ.

### Sprint 2 (Месяц 2): "Живой JS"

**Цель:** JS-код в V8 Isolate рисует в WebGPU окне через компоненты.

- [ ] V8 bindings через Rust (deno_core или v8-rs)
- [ ] Bun subprocess или embedded runtime
- [ ] IPC протокол Shim ↔ Runtime (бинарный)
- [ ] Компонент View: прямоугольник с фоном
- [ ] Компонент Text: строка текста
- [ ] Flexbox layout (базовый: column, row, gap)
- [ ] Клик по компоненту → событие в JS

**Критерий успеха:** JS-код рисует 3 прямоугольника с текстом, клик обрабатывается. RAM < 50 МБ.

### Sprint 3 (Месяц 3): "Приложение"

**Цель:** Полноценное демо-приложение + бенчмарк.

- [ ] Компоненты Button, TextInput, ScrollView
- [ ] Демо: Notes App (список заметок, создание, редактирование, удаление)
- [ ] Тема (цветовая схема)
- [ ] Бенчмарк: тот же Notes App на Electron vs CORE
- [ ] Замеры: RAM, CPU, startup time, FPS

**Критерий успеха:** Notes App работает. RAM < 100 МБ. Startup < 200 мс. FPS 60.

---

## Technical Decisions (Open)

### V8 Integration
**Вариант A:** deno_core (Rust-крейт, V8 bindings)
**Вариант B:** Bun как subprocess, IPC через stdio/socket
**Вариант C:** Bun embedded через C FFI

### IPC Protocol
**Вариант A:** MessagePack (бинарный, быстрый)
**Вариант B:** FlatBuffers (zero-copy)
**Вариант C:** Custom binary protocol

### Text Rendering
**Вариант A:** cosmic-text (Rust крейт)
**Вариант B:** skia-safe (тяжелый, но полноценный)
**Вариант C:** glyphon (wgpu-native text)

### Font Loading
**Вариант A:** Системные шрифты через font-kit
**Вариант B:** Встроенный 1 шрифт (Inter/Roboto)
**Вариант C:** Загрузка из файлов через абстракцию

---

## Dependencies (Rust)

```toml
[dependencies]
winit = "0.30"         # Window creation
wgpu = "22"            # WebGPU
deno_core = "0.300"    # V8 bindings (или альтернатива)
serde = { version = "1", features = ["derive"] }
rmp-serde = "1"        # MessagePack
cosmic-text = "0.12"   # Text rendering (или альтернатива)
```

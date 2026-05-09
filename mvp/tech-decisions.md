# MVP — Открытые технические решения

Перед началом кода нужно закрыть эти вопросы.

---

## 1. V8 Integration: как запускать JS внутри Rust?

### Вариант A: deno_core (Recommended)
- Rust-крейт, прямые V8 bindings
- JS код выполняется внутри Rust-процесса
- IPC через Rust-ops (быстрее любого протокола)
- Минус: привязка к версии V8 от Deno

### Вариант B: Bun как subprocess
- Bun запускается как отдельный процесс
- IPC через stdio pipe (binary)
- Плюс: полная экосистема Bun (SQLite, HTTP)
- Минус: overhead на IPC, сложнее управлять lifecycle

### Вариант C: Bun embedded через C FFI
- Bun компилируется как shared library
- Rust вызывает через FFI
- Плюс: best of both worlds
- Минус: никто так не делал, высокий риск

### Моя рекомендация
**Начать с Варианта B** (Bun subprocess). Самый быстрый путь к рабочему прототипу. Если IPC станет узким местом — мигрировать на deno_core.

---

## 2. IPC Protocol: Shim ↔ Runtime

### Вариант A: bincode (Rust-native)
- Самый быстрый для Rust↔Rust
- Но нужен парсер на TS-стороне

### Вариант B: MessagePack
- Универсальный, есть библиотеки и для Rust, и для Bun
- Бинарный, компактный
- Чуть медленнее bincode

### Вариант C: FlatBuffers
- Zero-copy read
- Сложнее в настройке
- Оверхед на схему

### Воя рекомендация
**MessagePack** (rmp-serde на Rust, msgpack-lite на TS). Быстрый, бинарный, универсальный.

---

## 3. Text Rendering

### Вариант A: cosmic-text
- Pure Rust
- Шрифты, layout, shaping
- Легковесный
- Минус: нет subpixel rendering

### Вариант B: glyphon
- wgpu-native text rendering
- Интеграция с wgpu из коробки
- Минус: меньше возможностей по layout

### Вариант C: skia-safe
- Полноценный 2D-рендеринг (текст, векторы, изображения)
- Тяжелый (50+ МБ бинарник)
- Избыточный для MVP

### Моя рекомендация
**glyphon** для MVP (нативная интеграция с wgpu). Если не хватит — мигрировать на cosmic-text.

---

## 4. Target Platform (MVP)

### Вариант A: Windows только
- Самая большая аудитория разработчиков
- Проще тестировать
- Минус: API-специфичный код (Win32)

### Вариант B: macOS только
- Основная аудитория JS/TS-разработчиков
- Metal backend для wgpu (стабильнее)

### Вариант C: Кроссплатформа сразу
- winit + wgpu = кроссплатформа по дизайну
- Но debugging сложнее

### Моя рекомендация
**Windows** для MVP (самый быстрый путь к бенчмарку vs Electron). Архитектура — кроссплатформенная через абстракции winit/wgpu.

---

## 5. State Management в Shell

### Вариант A: Простой reactive object
- Proxy-based (как Vue 3 reactive)
- Минимальный overhead
- Компоненты подписываются на изменения

### Вариант B: Event bus
- Pub/Sub: компоненты шлют события, store реагирует
- Проще для отладки
- Минус: нет автоматической подписки

### Вариант C: External state (Zustand-like)
- Store + subscribe pattern
- Проверенный подход

### Моя рекомендация
**Proxy-based reactive** (Вариант A). Минимальный код, нет зависимостей, достаточно для MVP.

---

## 6. Voice Input (Whisper)

### Вариант A: whisper.cpp через WebGPU
- Локальный инференс
- Нет зависимости от интернета
- Сложно интегрировать в MVP за 1 спринт

### Вариант B: Web Speech API (браузерная заглушка)
- Работает из коробки в Chrome
- Но мы не в браузере

### Вариант C: External API (OpenAI Whisper)
- Быстро интегрировать
- Требует интернета
- Для MVP достаточно, потом заменить на локальный

### Моя рекомендация
**Вариант C для MVP** (OpenAI Whisper API). Быстро работает, легко интегрировать. В v2 — переход на локальный whisper.cpp через wgpu.

---

## Decision Log

| # | Вопрос | Решение | Дата |
|---|--------|---------|------|
| 1 | V8 Integration | Bun subprocess (Вариант B) | TBD |
| 2 | IPC Protocol | MessagePack (Вариант B) | TBD |
| 3 | Text Rendering | glyphon (Вариант B) | TBD |
| 4 | Target Platform | Windows (Вариант A) | TBD |
| 5 | State Management | Proxy-based reactive (Вариант A) | TBD |
| 6 | Voice Input | OpenAI Whisper API (Вариант C) | TBD |

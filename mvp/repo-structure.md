# MVP — Структура репозитория

## Monorepo

Один репозиторий, два основных пакета + общие модули.

```
os/
├── AGENTS.md
├── archive/                        # Исходные обсуждения
├── project/                        # Проектная документация
├── mvp/                            # MVP планирование
│
├── src/
│   ├── shim/                       # [Rust] Host Shim
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs             # Точка входа
│   │       ├── window.rs           # winit + wgpu: создание окна
│   │       ├── renderer.rs         # WebGPU: отрисовка draw-команд
│   │       ├── input.rs            # Мышь, клавиатура → события
│   │       ├── ipc.rs              # IPC с Runtime (бинарный протокол)
│   │       └── text.rs             # Рендеринг текста (cosmic-text/glyphon)
│   │
│   ├── runtime/                    # [Bun/TS] Core Runtime
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts            # Точка входа: запуск Shim, инициализация
│   │       ├── ipc.ts              # IPC с Shim (бинарный протокол)
│   │       ├── kernel.ts           # Управление изолятами, lifecycle
│   │       ├── layout/
│   │       │   ├── engine.ts       # Flexbox layout engine
│   │       │   ├── types.ts        # Layout node, style types
│   │       │   └── measure.ts      # Текстовое измерение
│   │       ├── components/
│   │       │   ├── view.ts         # Контейнер
│   │       │   ├── text.ts         # Текст
│   │       │   ├── button.ts       # Кнопка (interactive)
│   │       │   ├── text-input.ts   # Поле ввода
│   │       │   ├── scroll-view.ts  # Прокрутка
│   │       │   └── image.ts        # Изображение
│   │       ├── reconciler.ts       # Diff компонентного дерева → draw-команды
│   │       └── draw.ts             # Типы draw-команд для Shim
│   │
│   ├── shell/                      # [Bun/TS] Core Shell
│   │   ├── package.json
│   │   └── src/
│   │       ├── index.ts            # Точка входа: монтирование Shell
│   │       ├── layout/
│   │       │   ├── top-bar.ts      # Верхняя панель
│   │       │   ├── sidebar.ts      # Боковая панель (проекты + теги)
│   │       │   ├── content.ts      # Основная область
│   │       │   ├── command-bar.ts  # Cmd+K: поиск и команды
│   │       │   └── status-bar.ts   # Нижняя панель (timeline)
│   │       ├── features/
│   │       │   ├── projects.ts     # CRUD проектов
│   │       │   ├── ideas.ts        # CRUD идей
│   │       │   ├── tags.ts         # Управление тегами
│   │       │   ├── search.ts       # Полнотекстовый поиск
│   │       │   ├── timeline.ts     # Лента активности
│   │       │   └── voice.ts        # Whisper → текст → Command Bar
│   │       └── store/
│   │           ├── db.ts           # SQLite через Bun
│   │           ├── schema.ts       # Схема БД
│   │           └── queries.ts      # Запросы
│   │
│   └── apps/                       # [Bun/TS] Встроенные приложения
│       └── notes/
│           ├── package.json
│           └── src/
│               └── index.ts        # Notes App (демо для бенчмарка)
│
├── tests/
│   ├── benchmarks/
│   │   ├── electron-notes/         # Notes App на Electron (для сравнения)
│   │   └── results/                # Результаты бенчмарков
│   └── integration/
│       ├── shim.test.ts            # Тесты IPC, окна, ввода
│       ├── layout.test.ts          # Тесты layout engine
│       └── components.test.ts      # Тесты компонентов
│
├── tools/
│   └── dev.ts                      # Dev server: запуск Shim + Runtime + Shell
│
├── Cargo.toml                      # Root workspace (Rust)
├── package.json                    # Root workspace (Bun)
└── bunfig.toml                     # Bun config
```

---

## Границы модулей

### Shim (Rust) → Runtime (TS)
- Shim НЕ знает про компоненты, layout, проекты
- Shim знает только: draw-команды (прямоугольник, текст, изображение) и input-события
- Протокол: бинарный, через stdio pipe или socket

### Runtime (TS) → Shell (TS)
- Runtime предоставляет компоненты (View, Text, Button) и layout
- Shell использует компоненты для построения интерфейса
- Runtime НЕ знает про проекты, теги, поиск

### Shell (TS) → Apps (TS)
- Shell запускает приложения в изолятах
- Приложения используют @core/components для UI
- Приложения НЕ знают про Shell (только про Runtime API)

---

## Зависимости

### Rust (shim/Cargo.toml)
```toml
[dependencies]
winit = "0.30"         # Window
wgpu = "22"            # WebGPU
serde = { version = "1", features = ["derive"] }
bincode = "1"          # Binary serialization (или rmp-serde)
```

### Bun (runtime/package.json, shell/package.json)
```json
{
  "dependencies": {
    // Runtime: нет внешних зависимостей (Bun встроен)
  }
}
```

### Bun встроенное
- SQLite (bun:sqlite)
- HTTP сервер (Bun.serve)
- TypeScript (нативная поддержка)

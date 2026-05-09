# AGENTS.md — CORE OS Project Configuration

## Project: CORE OS

Распределенная, Web-native операционная система. Работает поверх любой хост-ОС (Windows/Linux/macOS/Android) как "паразит-симбионт" — забирает ресурсы, полностью заменяет пользовательский опыт.

### Documentation Structure

```
os/
├── AGENTS.md                    # Этот файл — конфигурация проекта
├── archive/                     # Исходные обсуждения (brainstorm)
│   ├── core.md                  # Основной мозговой штурм
│   ├── architector.md           # Техническая прожарка архитектором
│   ├── marketolog.md            # Маркетинговая стратегия
│   ├── investor.md              # Питч для инвестора
│   ├── gazprom.md               # Промышленный кейс (Газпром)
│   └── gorynych.md              # Консорциум Яндекс/Сбер/ВК
├── layers/                      # Слои проектирования (сверху вниз)
│   ├── layer-1-user-experience.md          # UX + Space: что видит пользователь
│   ├── layer-2-system-split.md             # Фронт (Shell) и Бэк (Backoffice)
│   ├── layer-3-installation-scenarios.md   # Сценарии установки и эксплуатации
│   ├── layer-7-technical-decomposition.md  # Подсистемы: техническая декомпозиция
│   ├── layer-6-security.md                 # Безопасность: единый кросс-слойный документ
│   ├── layer-4-devices.md                  # Устройства и носители: USB, диски, сеть, P2P, принтеры
│   ├── layer-5-apps.md                     # Модель приложений: 5 уровней интеграции
│   └── layers-review.md                  # Анализ слоёв: противоречия, дублирование, пробелы
├── project/                     # Проектная документация
│   ├── vision.md                # Видение и философия
│   ├── architecture.md          # 5-уровневая архитектура
│   ├── tech-stack.md            # Стек технологий и обоснование
│   ├── security.md              # Модель безопасности
│   ├── multiuser.md             # Мультипользовательность
│   ├── filesystem.md            # Виртуальная файловая система
│   ├── ui-framework.md          # UI-фреймворк (3 уровня)
│   ├── ai-layer.md              # ИИ-слой и Intent API
│   ├── integration-modes.md     # Режимы интеграции с хост-ОС
│   ├── p2p-sync.md              # P2P-сеть, CRDT, синхронизация
│   ├── backoffice.md            # Бэк-офис и Суперюзер
│   ├── stress-tests.md          # Результаты стресс-тестов
│   └── business-model.md        # Бизнес-модель и go-to-market
├── mvp/                         # MVP планирование (3 месяца)
│   ├── README.md                # MVP scope и timeline
│   ├── track1-runtime.md        # Трек 1: Core Runtime
│   ├── track2-shell.md          # Трек 2: Core Shell
│   ├── repo-structure.md        # Структура репозитория
│   └── tech-decisions.md        # Открытые технические решения
└── src/                         # Исходный код (TODO)
```

### Язык общения

- **Всегда на русском** — все ответы, пояснения, обсуждения
- **Запрещены:** китайский, японский (иероглифы), украинский — нигде и никогда
- Документация: русский для проектной документации, английский для кода и коммитов

### Before Committing

1. Все новые документы — в `project/`
2. Формат: Markdown, заголовки `##`, subsections `###`
3. Каждый документ должен быть самодостаточным — читается без остальных
4. Cross-reference: ссылаться на другие документы как `[См. architecture.md](architecture.md)`

### Tech Stack (MVP)

| Layer | Technology | Purpose |
|-------|-----------|---------|
| Host Shim | Rust (winit + wgpu) | Window, WebGPU rendering, input |
| Runtime | Bun (TypeScript) | Component tree, layout, IPC |
| Apps | V8 Isolates (via Bun) | Application sandbox |
| Rendering | WebGPU (wgpu) | Native canvas, 60 FPS |
| Voice | Whisper API (OpenAI) | Voice input → Command Bar |
| Storage | SQLite (Bun built-in) | Projects, ideas, tags |

### Key Architecture Rules (MVP)

- No DOM, no CSS, no Chromium — only WebGPU rendering
- No GC in system modules — TSCLANG with ARC or manual memory management
- No JSON in P2P — only binary deltas
- No central servers — P2P mesh with Merkle Search Trees
- No IPC bridges — SharedArrayBuffer + zero-copy ABI
- User interactivity always wins over sync — priority-based scheduling

### Build Commands (TODO)

```bash
# TODO: заполнить после настройки проекта
# cargo build
# bun run dev
# bun test
```

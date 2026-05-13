# Workspace — Cross-Platform Overlay Runtime

> **Status:** Research & Early Development (Pre-Alpha)  
> **License:** Apache-2.0  
> **Languages:** Rust (systems layer) · TypeScript/Bun (runtime & apps)

---

## English

Workspace is a **host-agnostic overlay runtime** that provides a unified, secure workspace across Windows, macOS, Linux, Android, and iOS. It runs alongside the host operating system, leveraging its hardware drivers while delivering a consistent user experience, offline-first P2P sync, and local AI inference.

### What makes it different
- **No vendor lock-in:** Your workspace, data, and identity move seamlessly between devices.
- **Privacy-first:** All data stays on-device or inside your private mesh. No central cloud required.
- **Legacy compatibility:** Web-native apps and wrapped legacy tools coexist in one environment.
- **Local AI:** Intent-based UI, voice control, and smart scheduling run on-device via ONNX / Ollama.

### Architecture at a glance
```
┌─────────────────────────────────────────────┐
│  User Layer: Command Bar · Window Manager   │
│  Apps: V8 Isolates · Island Mode (WebView)  │
├─────────────────────────────────────────────┤
│  Micro-Kernel (Bun/TS): IPC · Security ·    │
│  VFS · CRDT · P2P Mesh · Backup             │
├─────────────────────────────────────────────┤
│  Display Server (Rust): WebGPU · Compositor │
├─────────────────────────────────────────────┤
│  Host Shim (Rust): winit · wgpu · CPAL ·    │
│  Network · Storage — Windows/macOS/Linux/   │
│  Android/iOS                                │
└─────────────────────────────────────────────┘
```

### Repository structure

```
├── archive/          # Raw brainstorming + historical code snapshots (read-only)
├── layers/           # Design layers (UX → AI → Security → Hardware)
├── plan/             # 37 implementation phases + roadmap
├── src/              # Source code
│   ├── demo/         # Phase 0: playable prototype (wgpu + winit + fontdue)
│   ├── display_server/
│   ├── host_shim/
│   ├── island_mode/
│   └── micro_kernel/
├── tests/            # Cross-phase test specs
└── log/              # Development session logs
```

### Current status
- **Phase 0 (Playable Demo):** ✅ Complete. Window, rendering, cursor, shapes, text, command bar. Runs on Windows 11 (native Vulkan) and WSL2 Ubuntu (llvmpipe via WSLg).
- **Phase 1–11 (Host Shim + Display Server):** Specification complete, implementation in progress.
- **Phase 12+ (Micro-Kernel & Apps):** Architecture frozen, awaiting Rust foundation.

See [`PROJECT_STATUS.md`](PROJECT_STATUS.md) for a detailed progress report.

### Quick start

```bash
# Run the Phase 0 demo (Windows or WSL2 with GUI support)
cd src && cargo run --bin demo

# Run tests
cargo test

# Strict linting (must pass on both Windows and Linux)
cargo clippy -- -D warnings
```

### Quick links
- [Project Status](PROJECT_STATUS.md)
- [Roadmap & 37 Implementation Phases](plan/roadmap.md)
- [Architecture Layers](layers/)
- [Contributing](CONTRIBUTING.md)
- [Security Policy](SECURITY.md)
- [Agent Configuration](AGENTS.md) — coding standards & environment notes

---

## Русский

Workspace — это **кросс-платформенный overlay runtime**, который создаёт единое защищённое рабочее пространство поверх Windows, macOS, Linux, Android и iOS. Система использует драйверы хост-ОС, но предоставляет согласованный пользовательский опыт, P2P-синхронизацию в offline-режиме и локальный ИИ-инференс.

### Ключевые отличия
- **Независимость от платформы:** Ваше рабочее пространство, данные и идентичность бесшовно перемещаются между устройствами.
- **Приватность прежде всего:** Все данные остаются на устройстве или внутри приватного mesh. Центральное облако не требуется.
- **Совместимость с legacy:** Web-native приложения и обёртки над legacy-инструментами сосуществуют в одной среде.
- **Локальный ИИ:** Intent-based UI, голосовое управление и умное планирование работают на устройстве через ONNX / Ollama.

### Структура репозитория

```
├── archive/          # «Сырой» мозговой штурм + исторические снапшоты кода (только для чтения)
├── layers/           # Слои проектирования (UX → AI → Безопасность → Железо)
├── plan/             # 37 фаз реализации + дорожная карта
├── src/              # Исходный код
│   ├── demo/         # Фаза 0: играбельный прототип (wgpu + winit + fontdue)
│   ├── display_server/
│   ├── host_shim/
│   ├── island_mode/
│   └── micro_kernel/
├── tests/            # Тесты сквозь фазы
└── log/              # Логи разработки по сессиям
```

### Текущий статус
- **Фаза 0 (Playable Demo):** ✅ Завершена. Окно, рендеринг, курсор, фигуры, текст, командная строка. Работает на Windows 11 (нативный Vulkan) и WSL2 Ubuntu (llvmpipe через WSLg).
- **Фазы 1–11 (Host Shim + Display Server):** Спецификация завершена, ведётся реализация.
- **Фазы 12+ (Micro-Kernel и приложения):** Архитектура зафиксирована, ожидается завершение Rust-фундамента.

См. [`PROJECT_STATUS.md`](PROJECT_STATUS.md) для детального отчёта о прогрессе.

### Быстрый старт

```bash
# Запустить демо фазы 0 (Windows или WSL2 с GUI)
cd src && cargo run --bin demo

# Запустить тесты
cargo test

# Строгий линтинг (должен проходить и на Windows, и на Linux)
cargo clippy -- -D warnings
```

### Ссылки
- [Статус проекта](PROJECT_STATUS.md)
- [Дорожная карта и 37 этапов](plan/roadmap.md)
- [Архитектурные слои](layers/)
- [Участие в проекте](CONTRIBUTING.md)
- [Политика безопасности](SECURITY.md)
- [Конфигурация для агентов](AGENTS.md) — стандарты кода и заметки по окружению

---

## Tech Stack

| Component        | Language / Runtime | Purpose                              |
|------------------|--------------------|--------------------------------------|
| Host Shim        | Rust               | Window, input, GPU, audio, files, net|
| Display Server   | Rust (wgpu, WGSL)  | WebGPU rendering & compositing       |
| Micro-Kernel     | Bun (TypeScript)   | IPC, SQLite, capability security     |
| App Runtime      | V8 Isolates        | Sandboxed apps with `@core/*` API    |
| Island Mode      | CEF / WebKit       | Web content & legacy app embedding   |
| AI Engine        | Bun + ONNX/Ollama  | ASR, SLM, TTS, embeddings            |
| P2P / Sync       | Bun + Rust         | Mesh networking, CRDT, WireGuard     |
| Storage          | SQLite (Bun)       | Data, indexes, settings, audit log   |

## License

Licensed under the Apache License, Version 2.0.  
See [LICENSE](LICENSE) for details.

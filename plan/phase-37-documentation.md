# Этап 37 — Documentation

## Цель
Создать полную документацию проекта: для пользователей, разработчиков, администраторов. После этого этапа любой может развернуть, использовать и расширять Workspace без дополнительных вопросов.

## Язык и стек
- **Язык:** Markdown, Mermaid (диаграммы), AsciiDoc (API reference, опционально)
- **Инструменты:** `mdbook` или `docusaurus` (docs site), `typedoc` (TypeScript API), `rustdoc` (Rust API)
- **Целевые ОС:** N/A (документация — кроссплатформенная)

## Зависимости
- **Все предыдущие этапы** (1–36).

## Часть системы
**Cross-cutting: Documentation** [См. layer-11 §Developer Reference, layer-10 §Business]

## Требования

### 37.1 User Documentation
- **Getting Started:** установка (все 5 платформ), первый запуск, создание профиля, recovery phrase.
- **Projects:** создание, переключение, теги, Smart Folders, checkpoint.
- **Voice Control:** настройка Whisper, список голосовых команд, push-to-talk, wake word.
- **Security:** уровни безопасности, RBAC, audit, remote wipe, Incognito.
- **Troubleshooting:** FAQ (20+ вопросов), диагностика (logs, safe mode), recovery.
- **Video Tutorials:** 5+ коротких видео (установка, проекты, голос, безопасность, Game Mode).

### 37.2 Developer Documentation
- **Architecture Overview:** 5-уровневая архитектура, data flow diagrams, component diagram.
- **App Model:** 5 уровней интеграции, `core.json` schema, `@core/*` API reference.
- **Intent API:** Intent structure, parser rules, handler registration, Generative UI.
- **P2P Protocol:** WireGuard handshake, CRDT sync protocol, Merkle Search Trees, signaling.
- **Contributing Guide:** code style, commit conventions, PR process, code review checklist.
- **Local Development:** `core-dev` CLI, debugging, dev tools, simulator.

### 37.3 Admin Documentation
- **Installation Guide:** system requirements, deployment scenarios (personal, corporate, bare metal).
- **Backoffice Manual:** каждый раздел с screenshots и step-by-step.
- **Hardcore Manual:** SSH setup, TUI navigation, CLI commands cheat sheet.
- **Security Hardening:** corp mode hardening, TPM setup, audit configuration, backup policies.
- **Migration Guide:** export/import, cross-platform migration, version upgrade.

### 37.4 API Reference
- **Intent API Reference:** все actions, params, examples.
- **App Manifest (`core.json`):** full schema, validation rules, examples for each level.
- **P2P RPC:** protocol buffers / binary format, endpoints, auth.
- **Rust API (`HostBackend`, `DisplayServer`):** rustdoc generated.
- **TypeScript API (`@core/*`):** typedoc generated.

### 37.5 Docs Infrastructure
- **Static Site:** docs site (docs.core.app или GitHub Pages), search, versioning (v1.0, v1.1, latest).
- **Search:** full-text search across all docs (Algolia or local FTS).
- **Versioning:** каждый release — отдельная версия документации. Latest — main branch.
- **I18n:** русский (primary), английский (secondary). Другие языки — community translations.
- **Auto-generation:** API reference генерируется автоматически из кода (rustdoc, typedoc). Architecture diagrams — из Mermaid в Markdown.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Getting Started | Быстрый старт | Новый пользователь разворачивает за 15 мин |
| Architecture | Архитектура | Диаграммы читаемы, data flow понятен |
| API Reference | Полный | Все endpoints задокументированы |
| Search | Поиск | Поиск "Game Mode" → находит все упоминания |
| Versioning | Версии | v1.0 и v1.1 → разные документы |
| I18n | Языки | Русский + английский, переключение |

## Интеграция с будущими этапами
- **Вход:** все предыдущие этапы.
- **Выход:** docs → пользователи, разработчики, администраторы.

## Критерии приёмки
- [ ] User docs: getting started, voice, security, troubleshooting — полнота.
- [ ] Developer docs: architecture, app model, Intent API, P2P — полнота.
- [ ] Admin docs: installation, Backoffice, Hardcore, hardening — полнота.
- [ ] API reference: Intent API, core.json, P2P RPC, @core/* — полнота.
- [ ] Docs site: работает, поиск работает, versioning работает.
- [ ] Новый разработчик разворачивает систему за 1 день по docs.
- [ ] Новый пользователь осваивает базовые функции за 15 минут.

## Ссылки
- [layer-10-business-model.md](../layers/layer-10-business-model.md) — Go-to-market, docs как часть продукта
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — Developer Reference

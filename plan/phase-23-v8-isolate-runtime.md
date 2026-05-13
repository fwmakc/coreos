# Этап 23 — V8 Isolate Runtime

## Цель
Создать runtime для нативных приложений Workspace на базе V8 Isolates (через Bun). После этого этапа приложения level 3–5 могут запускаться в sandbox, иметь доступ к `@core/*` API, и взаимодействовать с системой через capabilities.

## Язык и стек
- **Язык:** TypeScript (runtime), приложения пишутся на TypeScript/JavaScript
- **Runtime:** Bun (V8 Isolates через `Bun.Transpiler` и sandboxed execution)
- **Ключевые зависимости:** встроенный V8 в Bun, кастомный `@core/*` SDK
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: Core (IPC, event loop, SQLite).
- **Этап 13** — Micro-Kernel: Security (capability checks, sandbox policies).
- **Этап 14** — Micro-Kernel: VFS (файловый доступ).
- **Этап 22** — App Registry (установленные приложения, core.json, capabilities).
- **Этап 11** — Display Server: Compositor (текстуры для рендеринга).

## Часть системы
**Level 1 — Micro-Kernel: V8 Isolates** [См. layer-8 §4.1, layer-6 §3–4, layer-11 §App Model]

## Требования

### 20.1 Isolate Lifecycle
- **Создание:** при запуске приложения (из Command Bar, Project Manager, или URL) создаётся новый V8 Isolate.
- **Загрузка кода:** entry point (`core.json:entry`) транспилируется и выполняется в isolate.
- **Контекст:** каждый isolate получает `globalThis.core` — API-объект с методами, разрешёнными capabilities.
- **Состояния:** `created` → `running` → `suspended` (background) → `resumed` → `terminated`.
- **Freeze/Thaw:** при переключении Space или сворачивании приложения isolate приостанавливается (freeze), состояние сериализуется. При возврате — deserialize + resume [См. layer-1 §4.4].

### 20.2 Sandbox
- **Memory limit:** каждый isolate имеет лимит RAM (по умолчанию 128 МБ, настраивается в `core.json`).
- **CPU limit:** `requestAnimationFrame` и таймеры ограничены (max 60 FPS, таймауты > 1 сек агрегируются).
- **API restrictions:**
  - Нет доступа к `process`, `require`, `eval`, `Function` constructor.
  - `fetch` — только если выдано `network:http`. Домены ограничены whitelist.
  - `console.log` — перенаправлен в системный лог (с тегом app-id).
  - `setTimeout`/`setInterval` — ограничены 100 активными таймерами.
- **Filesystem sandbox:** `core.fs.read(path)` — path ограничен `WORKSPACE_ROOT/apps/<app-id>/` + explicitly granted папками.
- **Graphics sandbox:** приложение не имеет прямого доступа к GPU. Оно отправляет `RenderNode` дерево в Display Server (этап 9), который рендерит от имени приложения.

### 20.3 @core/* API
- **@core/fs:** `read`, `write`, `list`, `watch` — доступ к VFS с capability-check.
- **@core/net:** `fetch` (ограниченный), `websocket` — сеть.
- **@core/ui:** `render(tree)` — отправка React-подобного дерева компонентов в Display Server. Display Server рендерит его как texture для окна приложения.
- **@core/graphics:** `createCanvas()`, `draw()` — low-level WebGPU API для level 5 приложений (canvas texture отправляется в Display Server).
- **@core/audio:** `play()`, `capture()` — аудио через Host Shim.
- **@core/notifications:** `send(title, body)` — системные уведомления.
- **@core/intent:** `emit(intent)` — отправка Intent в систему (например, "открой файл").
- **@core/contacts:** `list()`, `search()` — адресная книга (с `contacts:read`).
- **@core/messenger:** `send(peer, message)` — отправка сообщения.
- **@core/mock:** npm-пакет для тестирования приложений вне Workspace (dev dependency).

### 20.4 Permissions UI
- При установке (этап 19) или при первом использовании capability приложение запрашивает право.
- **Диалог:** модальное окно (рендерится Display Server как Overlay Layer):
  - Иконка приложения, название.
  - Список запрашиваемых capabilities с описанием (из `core.json:permissions_ui`).
  - Кнопки: "Разрешить", "Разрешить один раз", "Отклонить".
- **Запоминание:** выбор пользователя сохраняется в `apps.capabilities`.

### 20.5 Warm Recovery
- **Checkpoint:** каждые 5 секунд (или по событию) состояние isolate сериализуется в SQLite.
- **Kill:** если приложение зависло (не отвечает на ping > 5 сек), пользователь может принудительно закрыть его.
- **Restore:** при повторном открытии приложения — восстановление из checkpoint. Потеря данных: максимум 5 секунд работы.
- **Native Process Monitor:** если isolate потребляет > 85% RAM — graceful suspend → checkpoint → kill [См. layer-8 §4.3.2].

### 20.6 Graphics Integration
- Приложение не рендерит напрямую в окно. Оно отправляет `RenderNode` tree через IPC в Display Server.
- **@core/ui:** React-подобный API. Компоненты: `View`, `Text`, `Image`, `Button`, `TextInput`, `ScrollView`.
  - Каждый компонент — описание (props + children), не DOM-узел.
  - Display Server превращает `RenderNode` tree в GPU draw calls (этап 8–9).
- **@core/graphics (level 5):** Raw WebGPU API. Приложение создаёт `RenderTexture`, рисует в неё через WebGPU commands, и отправляет texture handle в Display Server для compositing.

### 20.7 Error Handling
- **Isolate crash:** если isolate падает (unhandled exception, OOM) — система показывает уведомление "Приложение остановлено" с кнопками "Перезапустить" и "Закрыть". Лог ошибки отправляется в Error Reporting (placeholder, этап 27).
- **Graceful degradation:** если приложение не отвечает 2 секунды — Window Manager показывает Static UI Overlay (этап 15).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Launch app | Запуск изолейта | Открыть core.notes → isolate создан, код выполнен |
| @core/fs.read | Чтение файла | Вызов → содержимое файла (sandboxed) |
| @core/ui.render | Отрисовка UI | Отправка tree → Display Server рендерит окно |
| Capability deny | Отказ | Запросить `camera:capture` без права → ошибка |
| Permissions dialog | Диалог | Запросить `fs:write` → модальное окно |
| Checkpoint | Сохранение | Ввести текст → убить → открыть → текст на месте |
| Memory limit | Лимит | Выделить 200 МБ → OOM, isolate killed |

## Интеграция с будущими этапами
- **Вход:** этап 11 (Security) — capability checks, sandbox policies.
- **Вход:** этап 12 (VFS) — файловый доступ.
- **Вход:** этап 19 (App Registry) — core.json, capabilities, installed apps.
- **Вход:** этап 9 (Compositor) — scene graph, window texture compositing.
- **Выход:** `RenderNode` tree → этап 9 (Display Server).
- **Выход:** `Intent` → этап 25 (Intent API).
- **Выход:** errors → этап 27 (Error Reporting).

## Критерии приёмки
- [ ] Запуск приложения: isolate создан, entry point выполнен.
- [ ] `@core/fs.read` возвращает файл из песочницы.
- [ ] `@core/ui.render` отображает окно через Display Server.
- [ ] Capability deny: вызов без права → `CapabilityError`.
- [ ] Permissions dialog: запрос → модальное окно → выбор сохраняется.
- [ ] Checkpoint: kill → reopen → состояние восстановлено (потеря < 5 сек).
- [ ] Memory limit: превышение → graceful OOM, isolate killed.
- [ ] 10 приложений одновременно — стабильность, изоляция.

## Ссылки
- [layer-6-apps.md](../layers/layer-6-apps.md) — 5 уровней, Permissions UI, Warm Recovery
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — V8 Isolates §4.1, Sandbox §4.4
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — @core/* API, App Model

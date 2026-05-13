# Этап 12 — Micro-Kernel: WORKSPACE & IPC

## Цель
Создать ядро системы на TypeScript/Bun: runtime, IPC-мост с Host Shim, SQLite-схему данных, и базовый event loop. После этого этапа Workspace имеет работающий бэкенд, который получает события от Host Shim, хранит данные в SQLite, и может запускать простейшие скрипты.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun (v1.1+)
- **Ключевые зависимости:** `bun:sqlite` (встроенный SQLite), `bun:ffi` (Foreign Function Interface для связи с Rust), `bun:tcp` / `bun:udp` (сеть)
- **База данных:** SQLite (встроен в Bun)
- **Целевые ОС:** Windows, macOS, Linux, Android (Bun доступен на всех)

## Зависимости
- **Этап 1–3** — Host Shim: Window (события ввода).
- **Этап 6** — Host Shim: Audio (event loop интеграция).
- **Этап 8** — Host Shim: Network (транспорт для IPC).
- **Этап 9** — Display Server: WORKSPACE (surface, GPU — для будущей интеграции).

## Часть системы
**Level 1 — Micro-Kernel** [См. layer-8 §4, layer-3 §1, layer-4 §2]

Micro-Kernel — это сердце Workspace. Он управляет всей логикой: проекты, приложения, данные, настройки, безопасность. Он работает внутри Bun runtime и общается с Host Shim (Rust) через zero-copy IPC.

## Требования

### 10.1 IPC Bridge (Bun ↔ Rust)
- **SharedArrayBuffer:** основной канал связи. Rust пишет события в SAB, Bun читает через Atomics.
- **Zero-copy ABI:** данные передаются по ссылке (offset + length в SAB), а не копируются.
- **ARC (Automatic Reference Counting):** память управляется через счётчики ссылок на Rust-стороне. Bun запрашивает `retain`/`release` через FFI.
- **Сообщения от Host Shim → Bun:**
  - `InputEvent` (клавиатура, мышь, тач)
  - `AudioEvent` (capture buffer ready, playback underrun)
  - `StorageEvent` (watcher: create/modify/delete)
  - `NetworkEvent` (packet received, connection established)
  - `PanicExit` (от Host Shim)
- **Сообщения от Bun → Host Shim:**
  - `RenderCommand` (запрос на рендеринг кадра — пока placeholder, полная интеграция в этапе 13)
  - `AudioCommand` (play, stop, set volume)
  - `StorageCommand` (read, write, watch)
  - `NetworkCommand` (connect, send, close)
  - `WindowCommand` (set title, resize, set cursor)

### 10.2 SQLite Schema
- База данных `WORKSPACE.db` в `WORKSPACE_ROOT/`.
- **Таблицы (первая версия):**
  - `profiles` — профили пользователей (id, name, avatar, created_at).
  - `spaces` — контексты жизни (id, name, profile_id, color).
  - `projects` — проекты (id, name, space_id, layout_json, created_at).
  - `settings` — настройки (key, value, profile_id, scope — 'user' или 'system').
  - `audit_log` — журнал действий (id, timestamp, category, action, user_id, result).
- **Индексы:** по `profile_id`, `space_id`, `timestamp`.
- **WAL mode:** Write-Ahead Logging для конкурентного чтения/записи.
- **Шифрование:** SQLCipher (или `PRAGMA key`) для шифрования БД на диске. Ключ — derived from recovery phrase (этап 26).

### 10.3 Event Loop
- Bun запускает `micro-kernel/src/main.ts`.
- Event loop: `while (running) { process_ipc_messages(); process_timers(); process_async_io(); }`.
- **Таймеры:** `setTimeout` / `setInterval` для отложенных задач.
- **Async I/O:** SQLite запросы, сетевые операции, файловые операции.
- **Graceful shutdown:** при получении `PanicExit` от Host Shim — сохранить checkpoint (projects, settings), закрыть SQLite, выйти.

### 10.4 Configuration & Bootstrap
- `WORKSPACE.toml` — конфигурация ядра (пути, логи, уровень безопасности).
- `WORKSPACE_ROOT` определяется автоматически:
  - Windows: `%LOCALAPPDATA%\Workspace\`
  - macOS: `~/Library/Application Support/Workspace/`
  - Linux: `~/.local/share/Workspace/`
  - Android: `/data/data/app.WORKSPACE/files/`
- First-run detection: если `WORKSPACE.db` отсутствует — создать дефолтный профиль, дефолтный Space "Personal", дефолтный Project "Inbox".

### 10.5 Логирование
- Структурированные логи в JSON.
- Уровни: trace, debug, info, warn, error.
- Вывод: stdout + файл `logs/kernel-YYYY-MM-DD.log`.
- Ротация: 100 МБ файл, хранится 7 дней.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| IPC ping | Bun → Rust → Bun | Отправить ping → получить pong за < 1 мс |
| SQLite create | Создание БД | First run → `WORKSPACE.db` создан, таблицы есть |
| Insert profile | Запись в БД | Создать профиль → `SELECT` возвращает его |
| Event loop | Обработка сообщений | 1000 input events/сек → без потерь |
| Panic shutdown | Graceful exit | PanicExit → БД закрыта, лог записан |
| Config load | Чтение WORKSPACE.toml | Создать toml → настройки загружены |

## Интеграция с будущими этапами
- **Вход:** этап 1–6 (Host Shim) — события ввода, аудио, хранилища, сети через IPC.
- **Выход:** SQLite API → этап 11 (Security), 12 (VFS), 13 (Command Bar), 14 (Project Manager).
- **Выход:** event loop → этап 13 (Command Bar) для обработки input.
- **Выход:** IPC → этап 7–9 (Display Server) для render commands.

## Критерии приёмки
- [ ] Bun запускается и подключается к Host Shim через IPC.
- [ ] IPC round-trip (ping) < 1 мс (локально, SharedArrayBuffer).
- [ ] SQLite создаёт таблицы при first run.
- [ ] CRUD операции с профилями работают.
- [ ] 1000 input events/сек обрабатываются без потерь (измеряется счётчиком).
- [ ] PanicExit закрывает БД корректно (проверка через WAL checkpoint).
- [ ] WORKSPACE.toml загружается и применяется.

## Ссылки
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Фронт/Бэк разделение, IPC
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — First run, WORKSPACE_ROOT
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Micro-Kernel §4

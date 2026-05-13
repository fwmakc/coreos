# Этап 22 — App Registry

## Цель
Создать реестр приложений — подсистему установки, обновления, удаления, и управления приложениями. После этого этапа пользователь может устанавливать приложения из локальных файлов и P2P-источников.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (каталог приложений), `@noble/ed25519` (проверка подписей), `bun:ffi` (BLAKE3)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: Core (SQLite, event loop).
- **Этап 13** — Micro-Kernel: Security (capability checks).
- **Этап 14** — Micro-Kernel: VFS (хранение приложений).

## Часть системы
**Level 1 — Бэк: App Registry** [См. layer-6 §2, layer-8 §4.2, layer-11 §App Model]

## Требования

### 19.1 App Catalog
- SQLite таблица `apps`:
  - `app_id` (уникальный идентификатор, например `core.notes`).
  - `name`, `description`, `version`, `author`.
  - `manifest` (JSON, содержимое `core.json`).
  - `level` (1–5, модель интеграции).
  - `capabilities` (JSON-массив запрашиваемых capabilities).
  - `install_path` (CID пакета в VFS).
  - `signature` (Ed25519 подпись пакета).
  - `installed_at`, `updated_at`.
  - `is_system` (boolean, системные приложения не удаляются).

### 19.2 core.json Manifest
- Каждое приложение поставляется с `core.json`:
  ```json
  {
    "id": "core.notes",
    "name": "Notes",
    "version": "1.2.0",
    "level": 5,
    "capabilities": ["fs:read", "fs:write", "notifications:send"],
    "entry": "./index.js",
    "icon": "./icon.png",
    "permissions_ui": {
      "fs:read": "Read your notes",
      "notifications:send": "Send reminders"
    }
  }
  ```
- **Уровни интеграции:**
  - **Level 1:** "Как есть" — веб-приложение в Island Mode, нет манифеста.
  - **Level 2:** "Манифест" — `core.json` + Island Mode.
  - **Level 3:** "Бэк на месте" — `core.json` + V8 Isolate backend + Island Mode frontend.
  - **Level 4:** "@core/*" — `core.json` + V8 Isolate + доступ к `@core/ui`, `@core/fs`, `@core/net`.
  - **Level 5:** "Полный натив" — V8 Isolate + WebGPU + полный `@core/*` API.

### 19.3 Установка
- **Источники:**
  - Локальный файл `.corepkg` (zip-архив с `core.json` + код).
  - P2P seed (CID пакета, загружается через этап 17).
  - Store (placeholder, полный Store — в этапе 27).
- **Процесс установки:**
  1. Загрузка пакета (если P2P/Store).
  2. Проверка BLAKE3 CID.
  3. Проверка Ed25519 подписи (публичный ключ publisher'а в `trusted_keys`).
  4. Распаковка в `WORKSPACE_ROOT/apps/<app-id>/`.
  5. Чтение `core.json`, запрос capabilities у пользователя (placeholder UI — авто-грант на этом этапе, полный Permissions UI — в этапе 20).
  6. Регистрация в `apps`.
  7. Обновление desktop/project icons (через Display Server).

### 19.4 Обновление
- **Проверка обновлений:** периодическая проверка (раз в день) — сравнение версии с source.
- **Delta update:** если source поддерживает — загрузка только изменённых файлов (rsync-подобный алгоритм).
- **Rollback:** если обновление ломает приложение — возможность отката к предыдущей версии (хранится в `apps.versions`).

### 19.5 Удаление
- **Soft delete:** приложение помечается `uninstalled`, файлы не удаляются сраза (на случай отката).
- **Hard delete:** через 30 дней после soft delete — удаление файлов и очистка blob'ов (если на них нет ссылок).
- **Cleanup:** при удалении отзываются все выданные capabilities.

### 19.6 System Apps
- Предустановленные приложения (часть ОС):
  - `core.files` — файловый менеджер.
  - `core.notes` — заметки.
  - `core.terminal` — терминал.
  - `core.settings` — настройки.
  - `core.browser` — браузер (Island Mode).
  - `core.messenger` — мессенджер.
  - `core.contactbook` — контакты.
- System apps имеют `is_system = true` и не удаляются.

### 19.7 core-dev CLI (подготовка)
- Интерфейс для разработчиков:
  - `core-dev init` — создать шаблон приложения.
  - `core-dev run` — запустить приложение в dev-режиме (без подписи).
  - `core-dev build` — сборка пакета `.corepkg`.
  - `core-dev publish` — публикация в P2P / Store.
- На этом этапе — базовые команды `init` и `run`. Полный `build`/`publish` — в этапе 27.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Install local | Установка из файла | `.corepkg` → app появляется в registry |
| Verify signature | Проверка подписи | Подписанный пакет → установлен, подменённый → отказ |
| Request caps | Запрос capabilities | Установить → список capabilities записан |
| Update check | Проверка обновлений | Новая версия → предложение обновить |
| Uninstall | Удаление | Удалить → soft delete, файлы на месте 30 дней |
| System apps | Предустановленные | core.notes установлен по умолчанию |
| core-dev init | Шаблон | Запуск → создаётся папка с `core.json` |

## Интеграция с будущими этапами
- **Вход:** этап 11 (Security) — capability checks.
- **Вход:** этап 12 (VFS) — хранение пакетов, CID.
- **Выход:** `App` record → этап 20 (V8 Isolate Runtime) для запуска.
- **Выход:** `App` record → этап 21 (Island Mode) для level 1–3.
- **Выход:** `core-dev` → этап 27 (Store, polish).
- **Вход:** этап 17 (P2P) — загрузка пакетов из P2P.

## Критерии приёмки
- [ ] Установка `.corepkg` работает, app появляется в registry.
- [ ] Проверка подписи: подписанный пакет устанавливается, неподписанный — отказ (dev mode exception).
- [ ] Capabilities запрошены и записаны в `apps.capabilities`.
- [ ] Обновление: новая версия устанавливается, rollback работает.
- [ ] Удаление: soft delete, hard delete через 30 дней.
- [ ] System apps: предустановлены, не удаляются.
- [ ] `core-dev init` создаёт рабочий шаблон.

## Ссылки
- [layer-6-apps.md](../layers/layer-6-apps.md) — Модель приложений, 5 уровней, core.json
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — App Registry §4.2
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — App Manifest, core-dev CLI

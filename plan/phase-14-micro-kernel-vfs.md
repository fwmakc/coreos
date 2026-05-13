# Этап 14 — Micro-Kernel: Virtual File System

## Цель
Создать виртуальную файловую систему Workspace: единое пространство для всех данных, где файлы адресуются по содержимому (BLAKE3 CID), метаданные хранятся в SQLite (Passport), а тела — в blob-хранилище. После этого этапа Workspace имеет надёжное, дедуплицированное, версионированное хранилище.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `blake3` (хеширование), `bun:sqlite` (метаданные), `bun:ffi` (для BLAKE3 если нет чистого JS)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop, IPC).
- **Этап 13** — Micro-Kernel: Security (capability checks для доступа к файлам).
- **Этап 7** — Host Shim: Storage (чтение/запись файлов хоста).

## Часть системы
**Level 1 — Micro-Kernel: VFS** [См. layer-8 §5, layer-5 §2–3, layer-3 §1.3]

## Требования

### 12.1 Архитектура VFS
- **Passport (метаданные):** хранятся в SQLite. Каждый файл — запись с:
  - `cid` (BLAKE3 хеш содержимого, 32 байта, primary key).
  - `name` (имя файла, Unicode).
  - `tags` (JSON-массив тегов).
  - `created_at`, `modified_at` (HLC — Hybrid Logical Clock).
  - `owner_id`, `permissions`.
  - `versions` (JSON-массив: `{ cid, modified_at, author }`).
- **Body (содержимое):** хранится в `WORKSPACE_ROOT/blobs/`. Путь: `blobs/<first-2-hex>/<rest-hex>`.
- **Дедупликация:** если два файла имеют одинаковый BLAKE3 — хранится один blob. Passport — две записи, указывающие на один CID.

### 12.2 Операции VFS
- `create(path, data, tags?) -> FileRef` — создание файла. Вычисляет BLAKE3, записывает blob, создаёт Passport.
- `read(cid) -> Buffer` — чтение по CID. Проверка capability `fs:read`.
- `update(cid, new_data) -> FileRef` — обновление. Старый CID остаётся в versions. Новый blob записывается.
- `delete(cid)` — soft delete (флаг `deleted` в Passport). Blob не удаляется сразу (lazy GC).
- `list(query?) -> Vec<FileRef>` — список файлов с фильтрацией по тегам, дате, owner.
- `search(text) -> Vec<FileRef>` — полнотекстовый поиск через SQLite FTS5 (индексирует name и extracted text).

### 12.3 Теги вместо папок
- Нет иерархических папок. Только теги и Smart Folders.
- **Тег:** произвольная строка Unicode. Файл может иметь множество тегов.
- **Smart Folder:** сохранённый поисковый запрос (например, "теги: [work, urgent], создано: сегодня"). Реализация — live query в SQLite.
- **Tag graph:** таблица `tag_relations` (tag_a, tag_b, weight) для рекомендаций связанных тегов.

### 12.4 Ленивая загрузка (Lazy Load)
- **Ghost Files:** файлы, у которых Passport есть, но blob отсутствует локально.
- `is_locally_available(cid) -> bool` — проверка наличия blob.
- `request_from_network(cid)` — запрос blob у других устройств через P2P (этап 17). На этом этапе — placeholder, возвращает `NotAvailable`.
- **Cache policy:** LRU для локальных blobs. Макс. размер кэша — 80% от свободного дискового пространства.

### 12.5 Mirror Folders
- **Синхронизация с хост-ФС:** пользователь указывает папку хоста (например, `~/Documents`), и VFS создаёт Mirror Folder.
- **Watcher:** Host Shim (этап 5) отслеживает изменения и уведомляет VFS.
- **Bidirectional sync:** изменения в Mirror Folder отражаются в хост-папке и наоборот. Конфликты — LWW (Last Write Wins) с HLC.
- **Scope:** Mirror Folders работают только в рамках одного устройства (не синхронизируются через P2P).

### 12.6 Версионность
- Каждый `update` создаёт новую версию. Старые версии хранятся в `versions` JSON.
- `restore_version(cid, version_index)` — восстановление старой версии.
- **Garbage Collection:** blob'ы, на которые нет ссылок ни из одного Passport (включая версии), удаляются при фоновом GC.

### 12.7 Integration with Storage Backend
- При записи файла VFS использует `StorageBackend::write` (этап 5) для записи blob на диск хоста.
- При чтении — `StorageBackend::read`.
- **Sandbox:** приложение без `fs:write` не может вызвать `create` или `update`.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Create file | Создание с тегами | Создать "doc.txt" → Passport + blob |
| Deduplication | Два одинаковых файла | Загрузить два одинаковых → один blob |
| Search FTS | Полнотекстовый поиск | Поиск "hello" → находит файл |
| Smart Folder | Live query | Создать Smart Folder с тегом "work" → список обновляется |
| Mirror Folder | Синхронизация | Создать Mirror на ~/Docs → изменение в хосте → видно в VFS |
| Version restore | Версионность | Обновить файл → восстановить v1 → содержимое v1 |
| Ghost file | Lazy load | CID без blob → `is_locally_available` = false |

## Интеграция с будущими этапами
- **Вход:** этап 5 (Storage) — чтение/запись blob на диск.
- **Вход:** этап 11 (Security) — capability checks.
- **Выход:** `FileRef` API → этап 14 (Project Manager) для хранения проектов.
- **Выход:** `FileRef` API → этап 16 (CRDT) для синхронизации.
- **Выход:** CID → этап 17 (P2P) для запроса ghost files.
- **Выход:** search API → этап 13 (Command Bar) для поиска.

## Критерии приёмки
- [ ] Создание, чтение, обновление, удаление файлов работает.
- [ ] Дедупликация: два одинаковых файла → один blob (проверка через `du`).
- [ ] FTS5 поиск находит файлы по имени и содержимому.
- [ ] Smart Folder обновляется при добавлении тега (live query).
- [ ] Mirror Folder синхронизируется с хост-папкой (< 1 сек задержка).
- [ ] Версионность: 3 версии файла, восстановление v1 работает.
- [ ] Ghost file: `is_locally_available` = false, запрос на загрузку — placeholder.
- [ ] Capability: Guest не может удалить чужой файл.

## Ссылки
- [layer-5-devices.md](../layers/layer-5-devices.md) — VFS, дедупликация, Mirror Folders
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — VFS §5
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Теги, Smart Folders, проекты

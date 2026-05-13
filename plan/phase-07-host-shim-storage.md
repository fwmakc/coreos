# Этап 7 — Host Shim: Storage

## Цель
Добавить в Host Shim абстракцию для работы с файловой системой хост-ОС: чтение, запись, наблюдение (watcher), блокировки, и доступ к съёмным носителям (USB). После этого этапа Workspace умеет читать и писать файлы на хосте, отслеживать изменения в папках, и импортировать данные с USB.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `notify` (cross-platform file system watcher), `rusb` или `nusb` (USB access, опционально), `fs4` (file locking)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 1–3** — Host Shim: Windows, macOS, Linux (event loop для интеграции watcher).
- **Этап 6** — Host Shim: Audio (reference по архитектуре Host Shim).

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1.2, layer-5 §3, layer-4 §4]

## Требования

### 5.1 Файловая абстракция
- Определение trait `StorageBackend`:
  - `read(path) -> Vec<u8>` — атомарное чтение.
  - `write(path, data) -> Result` — атомарная запись через временный файл + rename.
  - `delete(path)` — удаление.
  - `exists(path) -> bool`
  - `metadata(path) -> Metadata` (размер, mtime, permissions).
  - `list_dir(path) -> Vec<DirEntry>`
  - `create_dir(path)`
  - `copy(from, to)`, `move(from, to)`
- Все пути — абсолютные или относительные к `WORKSPACE_ROOT` (рабочая директория Workspace на хосте).

### 5.2 Watcher (наблюдение за файлами)
- `watch(path, recursive) -> WatchId` — подписка на изменения.
- События: Create, Modify, Delete, Rename.
- **Особенность:** watcher должен быть интегрирован в event loop Host Shim (через `notify` с бэкендом, соответствующим платформе: ReadDirectoryChangesW, FSEvents, inotify, FAM).
- **Mirror Folders:** watcher используется для Mirror Folders — папок на хост-ФС, которые отображаются в VFS Workspace [См. layer-5 §3.3, layer-8 §5.4].

### 5.3 USB и съёмные носители
- `list_removable() -> Vec<StorageDevice>` — список подключённых USB-накопителей.
- `mount(device) -> MountedPath` — монтирование (или получение точки монтирования).
- **Windows:** `GetLogicalDrives` + `GetDriveType` (DRIVE_REMOVABLE).
- **macOS:** `DiskArbitration` framework (через FFI) или парсинг `/Volumes`.
- **Linux:** `udisks2` D-Bus или парсинг `/media`/`/mnt` + `udev`.
- **Android:** `StorageManager` через JNI.
- **Безопасность:** USB-накопитель проверяется на read-only при импорте (необязательно, но рекомендуется) [См. layer-7 §21.13].

### 5.4 Песочница файлов
- Host Shim ограничивает доступ приложений к ФС хоста через `StorageBackend`.
- По умолчанию приложение видит только свою песочницу (`WORKSPACE_ROOT/apps/<app-id>/`).
- Доступ за пределы песочницы — только через explicit capability и диалог подтверждения пользователя (реализация диалога — в этапе 20, V8 Isolate Runtime).

### 5.5 Дедупликация (подготовка)
- При записи файла Host Shim вычисляет BLAKE3 хеш содержимого.
- Хеш передаётся в Micro-Kernel (этап 12, VFS) для дедупликации.
- Сам Host Shim не хранит дедуплицированные данные — это задача VFS.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Read file | Чтение файла | Запрос → содержимое корректно |
| Write atomically | Атомарная запись | Прервать процесс → файл не повреждён |
| Watcher create | Создание файла | Создать файл → событие Create |
| USB list | Список USB | Вставить флешку → появляется в списке |
| USB read | Чтение с USB | Прочитать файл с USB → содержимое корректно |
| Sandbox | Ограничение | Запросить файл за пределами песочницы → отказ |

## Интеграция с будущими этапами
- **Выход:** `StorageBackend` API → этап 12 (VFS) для виртуализации ФС.
- **Выход:** watcher events → этап 12 (VFS, Mirror Folders).
- **Выход:** USB device events → этап 18 (Backup Engine) для backup на USB.
- **Вход:** этап 12 (VFS) → запросы на чтение/запись через `StorageBackend`.

## Критерии приёмки
- [ ] Компилируется на Windows, macOS, Linux, Android.
- [ ] Чтение и запись файлов работает на всех платформах.
- [ ] Watcher корректно отслеживает Create/Modify/Delete на всех платформах.
- [ ] USB-накопитель обнаруживается в течение 2 секунд после вставки.
- [ ] Чтение с USB работает.
- [ ] Атомарная запись не повреждает файл при аварийном завершении.
- [ ] Sandbox: приложение без capability не читает файлы за пределами песочницы.

## Ссылки
- [layer-5-devices.md](../layers/layer-5-devices.md) — USB, дедупликация, Mirror Folders
- [layer-7-security.md](../layers/layer-7-security.md) — Песочница файлов
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Storage §4.1.2

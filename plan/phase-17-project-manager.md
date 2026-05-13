# Этап 17 — Project Manager

## Цель
Создать систему проектов, тегов и Smart Folders — ядро организации данных в Workspace. После этого этапа пользователь может создавать проекты, назначать им теги, организовывать файлы, и переключаться между контекстами (Spaces).

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (проекты, теги, связи), `bun:ffi` (если нужен low-level access)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop).
- **Этап 14** — Micro-Kernel: VFS (файлы, теги, CID).
- **Этап 15** — Command Bar (создание проектов, навигация).

## Часть системы
**Level 1 — Фронт/Бэк: Project Manager** [См. layer-1 §3, layer-8 §5.2–5.3, layer-3 §1]

## Требования

### 14.1 Spaces (Контексты жизни)
- Space — изолированный контекст: Personal, Work, Family, Gaming.
- Каждый Space имеет:
  - Уникальный `space_id`.
  - Название, цвет (для UI), иконка.
  - Список проектов.
  - Собственные настройки и профиль пользователя.
- **Переключение:** `Ctrl+Shift+~` или через Command Bar. При переключении:
  - Текущие приложения сворачиваются (freeze isolates).
  - Загружаются проекты нового Space.
  - Layout восстанавливается из checkpoint.
- **Анонимный Space:** RAM-only Space без сохранения на диск (Guest). Данные не синхронизируются, не бэкапятся [См. layer-1 §6.2].

### 14.2 Projects (Проекты)
- Проект — рабочий стол с layout (положение окон, открытые файлы, теги).
- Поля проекта:
  - `project_id`, `name`, `space_id`.
  - `layout` (JSON): позиции окон, размеры, z-order, snap state.
  - `apps` (JSON): список открытых приложений с их state.
  - `tags` (массив): теги проекта.
  - `checkpoint` (JSON): последнее сохранённое состояние.
  - `autosave_interval` (сек): по умолчанию 5 сек.
- **Создание:** через Command Bar (`+ project "Name"`) или UI.
- **Удаление:** soft delete (помечается `deleted`, восстанавливается в течение 30 дней).

### 14.3 Tags (Теги)
- Тег — произвольная строка Unicode. Файлы, проекты, заметки, контакты — всё тегируется.
- **Tag Engine:**
  - `add_tag(entity_id, tag)` — добавить тег.
  - `remove_tag(entity_id, tag)` — удалить.
  - `search_by_tags(tags, operator?)` — поиск по тегам. `operator`: AND (все теги) или OR (любой).
  - `get_related_tags(tag)` — теги, часто встречающиеся вместе с данным (из `tag_relations`).
- **Smart Folders:** сохранённый поисковый запрос (теги + фильтры). Обновляется live при изменении данных.
- **Tag autocomplete:** при вводе тега Command Bar предлагает существующие теги.

### 14.4 Layout Engine
- **Grid:** проект разбит на ячейки. Окна snap к краям и углам (как Windows 11 или iPad).
- **Floating:** окна свободно перемещаются и масштабируются.
- **Split:** два окна рядом (50/50, 60/40, 33/33/33).
- **Tabbed:** окна внутри одной вкладки (как VS Code).
- **Сохранение layout:** автосохранение каждые 5 сек, ручное сохранение по `Ctrl+S`.
- **Restore:** при открытии проекта восстанавливается layout и открытые приложения из checkpoint.

### 14.5 Checkpoint & Autosave
- **Checkpoint:** полный снимок состояния проекта (layout, открытые файлы, позиции курсоров).
- **Частота:** каждые 5 секунд или по событию (переключение окна, сохранение файла).
- **Хранение:** checkpoint сохраняется в SQLite (`projects.checkpoint`).
- **Warm Recovery:** если приложение зависло, пользователь может "убить" его без потери данных — восстановление из последнего checkpoint [См. layer-1 §4.4].

### 14.6 Notes (Заметки)
- Заметка — специальный тип файла в VFS (MIME: `text/x-WORKSPACE-note`).
- **Формат:** Markdown-подмножество с расширениями (чекбоксы, теги inline).
- **AI-теги:** placeholder. При сохранении заметки Semantic Kernel (этап 25) предлагает теги. На этом этапе — ручное тегирование.
- **Ссылки:** `[[filename]]` или `[[tag]]` для внутренних ссылок.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Create Space | Новый контекст | Создать "Work" → появляется в списке |
| Switch Space | Ctrl+Shift+~ | Переключить → проекты меняются |
| Create Project | + project | Создать → проект появляется, layout пустой |
| Snap window | К краю | Перетащить к краю → окно занимает 50% |
| Save layout | Autosave | Переместить окно → через 5 сек checkpoint обновлён |
| Restore checkpoint | Warm Recovery | Убить приложение → открыть проект → состояние восстановлено |
| Smart Folder | Live query | Создать по тегу "urgent" → добавить тег файлу → файл появляется |

## Интеграция с будущими этапами
- **Вход:** этап 12 (VFS) — файлы, теги, CID.
- **Вход:** этап 13 (Command Bar) — создание, навигация.
- **Выход:** `Project` → этап 15 (Window Manager) для layout и окон.
- **Выход:** `Checkpoint` → этап 18 (Backup Engine) для включения в backup.
- **Выход:** теги → этап 25 (Intent API) для AI-рекомендаций.

## Критерии приёмки
- [ ] Создание Space и Project работает.
- [ ] Переключение Space меняет список проектов (< 200 мс).
- [ ] Snap to edge работает (4 зоны: лево, право, верх, полноэкранный).
- [ ] Layout autosave каждые 5 сек (проверка через SQLite).
- [ ] Checkpoint восстанавливается после "убийства" приложения.
- [ ] Smart Folder обновляется live (< 100 мс после добавления тега).
- [ ] Tag autocomplete предлагает существующие теги.
- [ ] Guest Space — RAM-only, не сохраняется на диск.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Проекты, теги, Smart Folders, Space
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Оптимистичные мутации, checkpoint
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Project Manager §5.2, Tag Engine §5.3

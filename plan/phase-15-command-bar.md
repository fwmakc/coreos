# Этап 15 — Command Bar Engine

## Цель
Создать Command Bar — центральную точку ввода Workspace. После этого этапа пользователь может набирать текст, видеть подсказки, переключаться между режимами, и взаимодействовать с системой через единую строку ввода.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `fuse.js` или кастомный fuzzy matcher (suggestion engine), `bun:sqlite` (индекс для поиска)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop, IPC).
- **Этап 14** — Micro-Kernel: VFS (поиск файлов, теги).
- **Этап 9–9** — Display Server (рендеринг UI — интеграция через IPC).

## Часть системы
**Level 1 — Фронт: Command Bar** [См. layer-1 §2, layer-8 §1, layer-11 §Command Bar]

Command Bar — это не просто поисковая строка. Это 8 режимов работы, каждый из которых превращает ввод в действие.

## Требования

### 13.1 Input Router
- **Источники ввода:**
  - Клавиатура (физическая и экранная).
  - Мышь / тач (клик по Command Bar, свайп вниз).
  - Голос (placeholder, полная интеграция в этапе 24).
- **Маршрутизация:** Input Router определяет, куда направить ввод:
  - Если фокус на Command Bar — ввод идёт в Command Bar.
  - Если фокус на приложении — ввод идёт в приложение.
  - Если Game Mode — ввод идёт в игру (этап 9, Display Server).
  - **Panic Gesture** (Ctrl+Shift+Esc / тройное касание угла) — всегда обрабатывается Host Shim, минуя Input Router [См. layer-1 §4.2].
- **Горячие клавиши (глобальные):**
  - `Alt+Tab` — переключение окон внутри проекта.
  - `Ctrl+~` — переключение проектов.
  - `Ctrl+Shift+~` — переключение Space.
  - `Win/Super` — открытие Command Bar (в fullscreen).
  - `Ctrl+Space` — открытие Command Bar (глобально).
- **Конфигурация:** все сочетания настраиваются. Приоритет: корп. конфиг Бэка > пользовательский конфиг > умолчания [См. layer-1 §4.2].

### 13.2 Режимы Command Bar
- **Режим 0: Search (поиск)** — по умолчанию. Fuzzy search по файлам, приложениям, контактам, настройкам.
- **Режим 1: Command (команды)** — префикс `>` или `!`. Системные команды: "lock", "restart", "settings", "backup".
- **Режим 2: Create (создание)** — префикс `+`. Создать заметку, проект, контакт, тег.
- **Режим 3: Navigate (навигация)** — префикс `@`. Переход к проекту, Space, файлу, контакту.
- **Режим 4: Ask (вопрос)** — префикс `?`. Вопрос к AI (placeholder, полная интеграция в этапе 25).
- **Режим 5: Calculate (калькулятор)** — префикс `=` или число. Математические выражения: `= 2+2`, `= 100 USD in EUR`.
- **Режим 6: Control (управление)** — префикс `#`. Управление системой: яркость, громкость, Bluetooth, WiFi (через Host Shim IPC).
- **Режим 7: Script (скрипты)** — префикс `$`. Выполнение пользовательских скриптов (JavaScript в sandbox).

### 13.3 Suggestion Engine
- **Fuzzy matching:** алгоритм, который находит совпадения с опечатками и неполным вводом. Например, "ntr" → "notes".
- **Приоритеты:**
  - Частота использования (чем чаще выбирали — тем выше).
  - Текущий контекст (в проекте "Work" приоритет рабочим файлам).
  - Тип совпадения (точное > префикс > fuzzy).
- **Категории suggestions:** Files, Apps, Commands, Contacts, Settings, Recent.
- **Кэш:** индекс suggestions хранится в SQLite FTS5 для мгновенного ответа (< 10 мс).

### 13.4 Рендеринг (через Display Server)
- Command Bar не рендерит сам. Он отправляет Display Server (этап 9) структуру `CommandBarFrame`:
  - Размер и позиция (обычно — верх/центр экрана или bottom sheet на мобильных).
  - Текст ввода (с курсором).
  - Список suggestions (выделенный item, иконки, подписи).
  - Режим (иконка режима слева).
- Display Server рендерит это как Overlay Layer поверх всех окон.
- **Анимации:** появление (opacity + translateY, 150 мс), disappearance (100 мс).

### 13.5 Settings
- `settings.json` в SQLite (таблица `settings` из этапа 10).
- Категории: General, Appearance, Input, Audio, Network, Security, Apps, Sync.
- **На этом этапе:** базовые настройки (тема, язык, масштаб, горячие клавиши).
- **Корпоративный конфиг:** если `allow_gui_admin: false`, настройки Security заблокированы [См. layer-1 §4.2].

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Open Command Bar | Ctrl+Space | Нажать → Command Bar появляется |
| Type "notes" | Fuzzy search | Ввести "nts" → suggestion "Notes" |
| Mode switch | > command | Ввести ">" → режим Command, suggestions команд |
| Alt+Tab | Window switch | Нажать → список окон |
| Settings open | > settings | Ввести ">settings" → открывается Settings |
| Calculator | = 2+2 | Ввести "=2+2" → результат 4 |

## Интеграция с будущими этапами
- **Вход:** этап 10 (IPC) — события клавиатуры/мыши от Host Shim.
- **Вход:** этап 12 (VFS) — поиск файлов и тегов.
- **Выход:** `CommandBarFrame` → этап 9 (Display Server) для рендеринга.
- **Выход:** выбранный Intent → этап 25 (Intent API) для обработки.
- **Выход:** скрипт `$...` → этап 20 (V8 Isolate) для выполнения.

## Критерии приёмки
- [ ] Command Bar открывается по Ctrl+Space (< 100 мс).
- [ ] Fuzzy search находит "notes" по запросу "nts" (< 10 мс).
- [ ] Все 8 режимов переключаются корректно.
- [ ] Alt+Tab показывает список ован (< 50 мс).
- [ ] Калькулятор вычисляет "=2+2" → 4.
- [ ] Settings открываются и сохраняются.
- [ ] Горячие клавиши настраиваются и применяются.
- [ ] Suggestions учитывают частоту использования.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Command Bar, режимы, настройки горячих клавиш
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Command Bar §1, Input Router §1.1
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — Command Bar описание, горячие клавиши

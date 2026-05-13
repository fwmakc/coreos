# Этап 18 — Window Manager

## Цель
Создать Window Manager — подсистему управления окнами: создание, закрытие, фокус, Z-стек, snap, переключение, и интеграция с Display Server. После этого этапа Workspace умеет открывать несколько окон, переключаться между ними, и управлять их расположением.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (состояние окон), `bun:ffi` (для IPC с Display Server)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 11** — Display Server: Compositor (scene graph, damage tracking, effects, chrome rendering).
- **Этап 15** — Command Bar (Alt+Tab, глобальные хоткеи).
- **Этап 17** — Project Manager (layout, snap, checkpoint).

## Часть системы
**Level 1 — Фронт: Window Manager** [См. layer-8 §3, layer-1 §4.2, layer-3 §2]

## Требования

### 15.1 Window Model
- Каждое окно — объект `Window`:
  - `window_id` (uuid).
  - `app_id` (какое приложение внутри).
  - `project_id` (какому проекту принадлежит).
  - `geometry` (x, y, width, height — в логических пикселях).
  - `state` (normal, minimized, maximized, fullscreen).
  - `z_index` (порядок наложения).
  - `focus` (boolean).
  - `chrome` (title bar visible, border radius, opacity).
- **Window lifecycle:**
  - `create(app_id, geometry) -> Window` — открыть окно.
  - `close(window_id)` — закрыть. Приложение получает событие `beforeunload`. Если есть несохранённые данные — приложение может отменить закрытие (модальный диалог — placeholder, на этом этапе принудительное закрытие с autosave).
  - `minimize(window_id)` — свернуть в taskbar/dock.
  - `maximize(window_id)` — развернуть на весь экран (не Game Mode).
  - `fullscreen(window_id)` — fullscreen с собственным chrome (F11).

### 15.2 Z-Stack и Фокус
- **Z-Stack:** список окон, отсортированный по `z_index`. Окно с наибольшим z_index — поверх всех.
- **Фокус:** только одно окно имеет фокус ввода в каждый момент.
- **Click to focus:** клик по окну поднимает его наверх и даёт фокус.
- **Focus follows mouse:** опционально (включается в настройках).
- **Always on top:** флаг для окон, которые остаются поверх (например, плеер).

### 15.3 Snap и Layout
- **Snap zones:** при перетаскивании окна к краю экрана активируется snap:
  - Лево/право — 50% ширины.
  - Верх — полноэкранный (не Game Mode).
  - Углы — 25% (квадрант).
- **Snap assist:** после snap первого окна предлагается snap второго окна в оставшееся пространство.
- **Restore:** двойной клик на title bar или нажатие `Win+↑` после snap — восстановление предыдущего размера.
- **Integration with Project Manager:** layout проекта определяет начальные позиции окон. При открытии проекта Window Manager восстанавливает их.

### 15.4 Window Chrome
- **Title bar:** рендерится Display Server (этап 9), а не приложением.
  - Заголовок (текст от приложения).
  - Кнопки: minimize, maximize/restore, close.
  - Иконка приложения.
- **Resize handles:** 8 зон (4 угла, 4 стороны). Курсор меняется при наведении.
- **Context menu:** правый клик на title bar — меню: minimize, maximize, close, move to project, always on top.

### 15.5 Window Switcher (Alt+Tab)
- **Вызов:** `Alt+Tab` (внутри проекта), `Ctrl+~` (между проектами).
- **UI:** горизонтальная лента окон с превью (thumbnail). Текущее окно выделено.
- **Навигация:** удержание Alt + Tab циклически переключает. Отпускание Alt — активация выбранного окна.
- **Game Mode:** при Alt+Tab в Game Mode отображается Window Switcher поверх shadow framebuffer (этап 9). Выбор Shell → выход из Game Mode.

### 15.6 Static UI Overlay (Graceful Degradation)
- Если CPU load > 90% в течение 2 секунд, Window Manager запрашивает у Display Server отрисовку Static UI Overlay — последнего успешно отрендеренного кадра без анимаций и эффектов.
- Все input events ставятся в Intent Queue (этап 25) и обрабатываются при освобождении CPU.
- **Восстановление:** когда CPU load < 70% в течение 1 секунды — возврат к нормальному рендерингу [См. layer-3 §2.3, layer-8 §3.3.2].

### 15.7 Интеграция с Display Server
- Window Manager не рендерит. Он управляет моделью окон и отправляет Display Server команды:
  - `CreateLayer(window_id, geometry, z_index)`
  - `UpdateLayer(window_id, geometry, opacity)`
  - `DestroyLayer(window_id)`
  - `SetFocus(window_id)`
- Display Server (этап 9) поддерживает эти команды и обновляет scene graph.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Open window | Создание | Открыть приложение → окно появляется |
| Alt+Tab | Переключение | Alt+Tab → фокус переходит, z-index меняется |
| Snap left | К краю | Перетащить к левому краю → 50% ширины |
| Minimize | Свернуть | Клик minimize → окно исчезает, taskbar обновляется |
| Close window | Закрыть | Клик close → окно закрывается, приложение уведомлено |
| Static UI | CPU overload | Загрузить CPU → Static UI Overlay активируется |
| Chrome | Title bar | Окно имеет title bar с кнопками |

## Интеграция с будущими этапами
- **Вход:** этап 9 (Compositor) — scene graph, layers, chrome rendering.
- **Вход:** этап 13 (Command Bar) — Alt+Tab, Ctrl+~.
- **Вход:** этап 14 (Project Manager) — layout, snap, checkpoint.
- **Выход:** `Window` model → этап 20 (V8 Isolate) — приложение получает resize/focus events.
- **Выход:** window focus → этап 24 (Voice) — Zero UI знает активное окно.

## Критерии приёмки
- [ ] Окно открывается, закрывается, минимизируется.
- [ ] Alt+Tab переключает окна (< 50 мs).
- [ ] Snap к краю работает (4 зоны).
- [ ] Resize handles меняют курсор и позволяют изменять размер.
- [ ] Chrome (title bar) рендерится Display Server.
- [ ] Static UI Overlay активируется при CPU > 90%.
- [ ] 20 окон открыты одновременно — система отзывчива.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Окна, Alt+Tab, Panic Gesture, Graceful Degradation
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Static UI Overlay, Graceful Degradation
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Window Manager §3

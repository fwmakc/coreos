# Этап 16 — Command Bar: UI

## Цель
Создать визуальный интерфейс Command Bar — рендеринг строки ввода, suggestions, анимаций, и интеграцию с Display Server. После этого этапа Command Bar отображается на экране как Overlay Layer с полным визуальным оформлением.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `@workspace/ui` (компоненты: View, Text, TextInput, ScrollView, Button), `bun:sqlite` (кэш иконок приложений)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 9–11** — Display Server (Compositor, 2D примитивы, текст, Overlay Layer).
- **Этап 15** — Command Bar Engine (рендер-агностик engine: режимы, suggestions, парсер).

## Часть системы
**Level 3 — Фронт: Command Bar UI** [См. layer-1 §2, layer-8 §1.4, layer-11 §Command Bar]

## Требования

### 16.1 Render Model
- Command Bar UI не рендерит сам. Он отправляет Display Server (этап 11) структуру `CommandBarFrame`:
  - `geometry`: размер и позиция (обычно — верх/центр экрана, 60% ширины; на мобильных — bottom sheet, 100% ширины).
  - `input`: текст ввода (с курсором, выделением, placeholder).
  - `suggestions`: список suggestions (выделенный item, иконки, подписи, shortcut hints).
  - `mode_badge`: иконка текущего режима слева от строки ввода.
  - `status_bar`: индикаторы (caps lock, input language, voice ready).
- Display Server рендерит `CommandBarFrame` как Overlay Layer поверх всех окон с z-index = `MAX_OVERLAY`.

### 16.2 Компоненты UI
- **Input Field:** однострочное поле ввода. Шрифт системный (15–17 pt), цвет — `theme.text.primary`. Placeholder — `theme.text.tertiary`.
- **Suggestion List:** вертикальный ScrollView под input field. Каждый item:
  - Иконка (24×24, из `@workspace/icons` или приложения).
  - Title (основной текст, bold если exact match).
  - Subtitle (дополнительная информация: путь файла, описание команды).
  - Shortcut hint (справа, `Ctrl+Enter`, `Tab` — если есть).
  - Divider между items (1px, `theme.border`).
- **Mode Badge:** круглая кнопка слева от input. Иконки: 🔍 Search, ❯ Command, ✚ Create, @ Navigate, ? Ask, = Calculate, # Control, $ Script.
- **Category Headers:** в списке suggestions разделители по категориям (Files, Apps, Commands, Contacts) с label и счётчиком.

### 16.3 Анимации
- **Появление:** opacity 0 → 1 (100 мс), translateY -20px → 0 (150 мс), easing `ease-out`.
- **Исчезновение:** opacity 1 → 0 (80 мс), translateY 0 → -10px, easing `ease-in`.
- **Suggestions update:** cross-fade старого списка в новый (80 мс). Если список пуст — показывается placeholder "Начните ввод...".
- **Selection change:** highlight перемещается с tween (50 мс), background color transition.
- **Mode switch:** badge иконка cross-fade (100 мс).

### 16.4 Mobile Adaptation
- **Bottom Sheet:** на Android/iOS Command Bar открывается снизу экрана (как системная клавиатура).
  - Свайп вниз — закрытие (threshold 30% высоты).
  - Safe area insets (notch, home indicator) учитываются.
- **Тач-интеракция:** tap по suggestion — выбор. Long press — предпросмотр (quick look).
- **Voice button:** микрофон иконка справа от input (на mobile — prominent, на desktop — subtle).

### 16.5 Accessibility
- **Screen Reader:** все элементы Command Bar имеют accessibility labels. Input — "Command Bar, text field". Suggestion — "Notes, file, double tap to open".
- **Keyboard Navigation:** Tab/Shift+Tab — фокус между input и suggestions. ↑/↓ — навигация по suggestions. Enter — активация. Esc — закрытие.
- **High Contrast:** в high contrast mode — жирные границы фокуса, повышенный контраст текста.

### 16.6 Theming
- Все цвета, шрифты, размеры — через theme tokens (этап 33). Нет хардкода.
- Поддержка dark/light mode переключения без перезагрузки.
- Размеры адаптируются под system font scale (accessibility).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Render frame | Отправка в Display Server | Открыть Command Bar → Overlay Layer появляется |
| Suggestion list | Список | 10 suggestions → рендерятся с иконками |
| Mode badge | Иконка режима | Переключить режим → badge меняется с анимацией |
| Animation open | Появление | Открыть → fade + slide за 150 мс |
| Mobile bottom | Bottom sheet | На Android → открывается снизу, свайп закрывает |
| Keyboard nav | Навигация | Tab → фокус, ↑↓ → selection, Enter → активация |

## Интеграция с будущими этапами
- **Вход:** этап 15 (Engine) — `CommandBarFrame` структура, режим, suggestions.
- **Вход:** этап 11 (Compositor) — Overlay Layer, scene graph, damage tracking.
- **Вход:** этап 33 (Themes) — theme tokens, dark/light mode.
- **Выход:** rendered overlay → пользователь (ввод, выбор).

## Критерии приёмки
- [ ] Command Bar отображается как Overlay Layer поверх всех окон.
- [ ] Suggestions рендерятся с иконками, title, subtitle (< 16 мс GPU time).
- [ ] Mode badge меняется при переключении режима.
- [ ] Анимация появления: 150 мс, 60 FPS, без рывков.
- [ ] Mobile bottom sheet: свайп вниз закрывает, safe area учтён.
- [ ] Keyboard navigation: Tab, ↑↓, Enter, Esc работают.
- [ ] Screen reader: все элементы labeled, focus announced.
- [ ] High Contrast: фокус виден, контраст соответствует WCAG AA.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Command Bar, режимы
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Display Server §3, Overlay §3.3.2
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — Command Bar описание

# Этап 33 — Accessibility & Themes

## Цель
Создать систему доступности (Accessibility) и тем оформления (Themes). После этого этапа Workspace доступна пользователям с ограниченными возможностями и поддерживает кастомизацию визуального стиля.

## Язык и стек
- **Язык:** TypeScript (theme engine, settings), Rust (post-processing shaders, screen reader API)
- **Runtime:** Bun, native Rust
- **Ключевые зависимости:** `wgpu` (post-processing shaders), `accesskit` (screen reader integration, опционально)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 11** — Display Server: Compositor (post-processing stack, shader pipeline).
- **Этап 16** — Command Bar UI (компоненты должны поддерживать accessibility).
- **Этап 23** — V8 Isolate Runtime (`@workspace/ui` components accessibility props).

## Часть системы
**Level 3 — Фронт: Accessibility & Themes** [См. layer-1 §4.7, layer-8 §4.14, layer-9]

## Требования

### 33.1 Accessibility
- **High Contrast:** пост-обработка в Display Server (инверсия или повышение контраста через shader). Не просто инверсия цветов, а smart contrast (текст остаётся читаемым).
- **Large Text:** глобальный масштаб шрифта (1.0x, 1.25x, 1.5x, 2.0x). Все UI-компоненты масштабируются пропорционально.
- **Screen Reader API:** Display Server предоставляет структурированный доступ к UI:
  - `AccessibilityTree` для каждого окна (role, label, value, state, bounding box).
  - Экспорт через platform-specific API:
    - Windows: MSAA/UI Automation.
    - macOS: NSAccessibility.
    - Linux: AT-SPI2.
    - Android: AccessibilityNodeInfo.
    - iOS: UIAccessibility.
- **Reduced Motion:** отключение всех анимаций (tween duration = 0, переходы — мгновенные). Уважение `prefers-reduced-motion`.
- **Color Blindness:** цветовые фильтры через post-processing shader (Deuteranopia, Protanopia, Tritanopia). Simulation mode для разработчиков.
- **Keyboard Navigation:** Tab/Shift+Tab (фокус), Enter/Space (активация), Esc (закрытие/отмена), Arrow keys (навигация в списках). Все интерактивные элементы reachable через keyboard.
- **Focus Indicators:** видимый фокус (outline 2px, цвет `theme.focus`), не зависящий от hover.

### 33.2 Themes
- **Theme Engine:** JSON-файл с design tokens:
  - Colors: `background`, `surface`, `text.primary`, `text.secondary`, `text.tertiary`, `accent`, `error`, `warning`, `success`, `border`, `focus`.
  - Typography: font family, font sizes (xs, sm, base, lg, xl, 2xl), line heights, weights.
  - Spacing: 4px grid (0, 4, 8, 12, 16, 24, 32, 48, 64).
  - Border radius: none, sm, base, lg, xl, full.
  - Shadows: none, sm, base, lg, xl (для elevation).
- **Встроенные темы:**
  - **Light:** белый фон, тёмный текст, синий accent.
  - **Dark:** тёмный фон, светлый текст, голубой accent.
  - **High Contrast:** чёрный/белый, без оттенков серого, жёлтый accent.
- **Кастомные темы:** пользователь может создать свою тему через Settings (color picker для 5 основных цветов, остальное — auto-generated через color theory).
- **System Theme Sync:** автоматическое переключение Light/Dark при изменении системной темы хост-ОС.
- **Hot Reload:** изменение темы применяется мгновенно (< 100 мс) без перезагрузки приложений. Shader recompilation + uniform update.

### 33.3 Migration Tools
- **Export:** `workspace-cli backup --export` или GUI wizard. Выбор scope: all data, projects only, settings only.
- **Import:** `workspace-cli restore --import`. Валидация формата версии, миграция schema при необходимости.
- **Cross-platform:** backup с Windows восстанавливается на macOS/Android (blob'ы + SQLite).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| High contrast | Контраст | Включить → текст читаем, UI distinguishable |
| Large text | Масштаб | 2.0x → все элементы масштабированы, без clipping |
| Screen reader | Чтение | Tab → фокус, screen reader announces label |
| Reduced motion | Без анимаций | Включить → анимации мгновенные |
| Color blind | Фильтры | Deuteranopia → цвета скорректированы |
| Theme switch | Переключение | Light → Dark → мгновенно, без перезагрузки |
| Custom theme | Кастом | Выбрать accent → тема сгенерирована |
| Migration | Перенос | Экспорт → импорт → данные на месте |

## Интеграция с будущими этапами
- **Вход:** этап 11 (Compositor) — post-processing shaders, uniform updates.
- **Вход:** этап 16 (Command Bar UI) — компоненты поддерживают accessibility.
- **Выход:** theme tokens → этап 23 (`@workspace/ui` components).
- **Выход:** accessibility tree → platform APIs.

## Критерии приёмки
- [ ] High Contrast: WCAG AAA контраст для текста.
- [ ] Large Text: 2.0x, нет clipping или overflow.
- [ ] Screen Reader: все интерактивные элементы labeled и focusable.
- [ ] Reduced Motion: анимации = 0 мс.
- [ ] Color Blindness: 3 фильтра, simulation mode.
- [ ] Theme switch: < 100 мс, без перезагрузки.
- [ ] Custom theme: 5 цветов → полная тема.
- [ ] Migration: export → import → идентичность данных.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Accessibility, Themes
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Accessibility §4.14

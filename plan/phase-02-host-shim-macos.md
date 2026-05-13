# Этап 2 — Host Shim: macOS

## Цель
Портировать Host Shim на macOS. После завершения этого этапа Workspace компилируется и запускается на macOS с идентичным поведением окна и ввода.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `winit` (окно + события), `cocoa` / `objc` crates (Native APIs при необходимости), `WORKSPACE-graphics` (DPI, дисплеи)
- **Целевая ОС:** macOS 12+ (Intel, Apple Silicon)

## Зависимости
- **Этап 1** — Host Shim: Windows. Использует trait `HostBackend`, определённый в этапе 1.

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1, layer-4 §3.1]

## Требования

### 2.1 Оконная подсистема
- Реализация `HostBackend` для macOS через `winit`.
- Поддержка оконного режима и полноэкранного (borderless fullscreen + native macOS Full Screen через зелёную кнопку).
- DPI scaling через `NSScreen::backingScaleFactor`.
- Title bar styling: Workspace рендерит собственный chrome, поэтому нативный title bar скрывается или минимизируется.
- **Важно:** macOS Full Screen (зелёная кнопка) должен переключать в borderless fullscreen режим WORKSPACE, а не создавать отдельное Space macOS.

### 2.2 Ввод с клавиатуры
- KeyDown / KeyUp / KeyRepeat через `winit`.
- Модификаторы: Shift, Ctrl, Option (Alt), Command (Meta).
- **Особенность macOS:** Cmd+Tab переключает приложения macOS, а не окна внутри WORKSPACE. В полноэкранном режиме WORKSPACE: Cmd+Tab = Command Bar, Option+Tab = переключение окон внутри WORKSPACE [См. layer-1 §4.2].
- **Panic Gesture:** тройное касание угла или Ctrl+Shift+Esc (Cmd+Esc на macOS) → `PanicExit`.

### 2.3 Ввод с мыши и трекпада
- Magic Mouse и трекпад: multi-touch gestures через `winit`.
- Три пальца — свайп влево/вправо (back/forward).
- Force Touch — дополнительное давление как отдельное событие (опционально).

### 2.4 Абстракции, переиспользуемые с Windows
- Все trait `HostBackend` остаются без изменений.
- Реализация `MacOSBackend` предоставляет те же методы, что и `WindowsBackend`.
- Платформенно-специфичный код инкапсулирован в `src/platform/macos/`.

### 2.5 Логирование
- Логи пишутся в `~/Library/Application Support/Workspace/logs/`.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Окно на macOS | Создание окна | Запуск на Mac → окно появляется |
| KeyDown 'A' | Раскладка QWERTY | Нажать A → KeyDown(A) |
| Cmd+Tab | Command Bar в fullscreen | Перейти в fullscreen, нажать Cmd+Tab → Command Bar открывается |
| Panic Gesture | Тройное касание угла | Коснуться угла 3 раза → PanicExit |
| Retina scaling | 2x/3x DPI | Переместить на Retina → масштаб корректный |

## Интеграция с будущими этапами
- Идентична этапу 1. Выход: `HostEvent` stream → этап 7 (Display Server), этап 10 (Micro-Kernel IPC).

## Критерии приёмки
- [ ] Компилируется на macOS Intel и Apple Silicon.
- [ ] Окно открывается, закрывается, изменяет размер.
- [ ] Все события клавиатуры и мыши попадают в лог.
- [ ] Cmd+Tab в fullscreen открывает Command Bar (проверяется через лог).
- [ ] Panic Gesture работает.
- [ ] Retina scaling корректен.
- [ ] Логи пишутся в `~/Library/Application Support/Workspace/logs/`.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Окна и приложения
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — Выход в хост-ОС
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Host Shim §4.1

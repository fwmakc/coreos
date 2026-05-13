# Этап 3 — Host Shim: Linux

## Цель
Портировать Host Shim на Linux. После завершения этого этапа Workspace компилируется и запускается на Linux (X11 и Wayland) с идентичным поведением окна и ввода.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `winit` (окно + события, с бэкендами x11 и wayland), `smithay-client-toolkit` (SCTK, при необходимости для Wayland-специфики)
- **Целевая ОС:** Linux (X11, Wayland), включая Android (NativeActivity — базовая поддержка)

## Зависимости
- **Этап 1** — Host Shim: Windows (trait `HostBackend`).
- **Этап 2** — Host Shim: macOS (reference реализация).

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1, layer-4 §3.1]

## Требования

### 3.1 Оконная подсистема
- Реализация `HostBackend` для Linux.
- `winit` автоматически выбирает бэкенд: Wayland (приоритет) → X11 (fallback).
- Поддержка оконного режима и полноэкранного (borderless).
- DPI scaling через `wl_output::scale` (Wayland) или `Xft.dpi` (X11).
- Title bar: Workspace рендерит собственный chrome. На Wayland используется `xdg-decoration` для запроса CSD (client-side decorations).

### 3.2 Ввод с клавиатуры
- KeyDown / KeyUp / KeyRepeat через `winit`.
- Модификаторы: Shift, Ctrl, Alt, Super (Meta).
- **Особенность Linux:** Super+Tab = Command Bar в полноэкранном режиме. Alt+Tab = переключение окон внутри CORE [См. layer-1 §4.2].
- **Panic Gesture:** тройное касание угла или Ctrl+Shift+Esc → `PanicExit`.

### 3.3 Ввод с мыши и тачпада
- События мыши и тачпада через `winit`.
- Wayland: `wl_pointer`, `wl_touch`.
- X11: `XI2` (X Input Extension 2) для multi-touch.
- Горизонтальный скролл (two-finger swipe).

### 3.4 Android (опционально, baseline)
- Базовая поддержка Android через `winit` с бэкендом `android_native_activity`.
- События: тач (multi-touch), кнопки «назад», «домой», «недавние».
- **Важно:** кнопка «домой» и свайп снизу вверх — Panic Gesture (выход в хост-ОС) [См. layer-4 §5.3].
- Экранный клавиатурный ввод (IME) через `SoftInput`.

### 3.5 Абстракции
- Все trait `HostBackend` остаются без изменений.
- Платформенно-специфичный код в `src/platform/linux/` и `src/platform/android/`.

### 3.6 Логирование
- Логи пишутся в `~/.local/share/Workspace/logs/`.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Окно на Wayland | Создание окна | Запуск на Wayland → окно появляется |
| Окно на X11 | Fallback | `WAYLAND_DISPLAY=""` → работает на X11 |
| KeyDown 'A' | Раскладка | Нажать A → KeyDown(A) |
| Panic Gesture | Ctrl+Shift+Esc | Нажать → PanicExit |
| Touch events | Android | Коснуться экрана → TouchDown с координатами |

## Интеграция с будущими этапами
- Идентична этапам 1–2. Выход: `HostEvent` stream → этап 7 (Display Server), этап 10 (Micro-Kernel IPC).

## Критерии приёмки
- [ ] Компилируется на Linux x64 и ARM64.
- [ ] Работает на Wayland (Sway, GNOME, KDE).
- [ ] Работает на X11 (i3, GNOME Xorg).
- [ ] Все события клавиатуры и мыши попадают в лог.
- [ ] Panic Gesture работает.
- [ ] Android baseline: компилируется, окно открывается, тач работает.
- [ ] Логи пишутся в `~/.local/share/Workspace/logs/`.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Окна и приложения
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — Выход в хост-ОС, Android
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Host Shim §4.1

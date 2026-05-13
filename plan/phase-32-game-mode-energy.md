# Этап 32 — Game Mode & Energy Manager

## Цель
Создать системные режимы: Game Mode (прямой доступ к GPU для приложений) и Energy Manager (управление энергопотреблением). После этого этапа пользователь может играть в игры с максимальной производительностью, а система автоматически экономит батарею.

## Язык и стек
- **Язык:** TypeScript (API, политики), Rust (Game Mode direct surface, Energy monitoring)
- **Runtime:** Bun, native Rust
- **Ключевые зависимости:** `wgpu` (direct surface), platform-specific battery APIs (Windows: `GetSystemPowerStatus`, macOS: `IOPowerSources`, Linux: `UPower`, Android: `BatteryManager`, iOS: `UIDevice.batteryLevel`)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 11** — Display Server: Compositor (direct surface, shadow framebuffer, input exclusivity).
- **Этап 18** — Window Manager (fullscreen, window states).
- **Этап 23** — V8 Isolate Runtime (Game Mode API для приложений).

## Часть системы
**Level 0/1 — System Modes** [См. layer-8 §3.6, §4.10, layer-1 §4.5]

## Требования

### 32.1 Game Mode
- **API:** `WORKSPACE.game.requestMode()` — запрос перехода в Game Mode (только level 4–5 приложения).
- **Direct Surface:** Display Server создаёт dedicated surface без compositor. Приложение рендерит напрямую в swapchain.
- **Input Exclusivity:** все вводы (клавиатура, мышь, геймпад) направляются напрямую в приложение.
- **Shadow Framebuffer:** перед входом в Game Mode — capture текущего framebuffer в GPU texture. При выходе (Panic Gesture, Alt+Tab) — мгновенное отображение shadow.
- **Переключение:**
  - Shell → Game Mode: < 33 мс (1–2 кадра).
  - Game Mode → Shell (Panic): мгновенно.
  - Game Mode → Shell (Alt+Tab): shadow framebuffer + window switcher.
- **Политики:** пользователь может заблокировать Game Mode для приложения (чёрный список).
- **Energy в Game Mode:** pause background sync, pause backup, minimum notifications.
- **Performance overlay:** опциональный FPS counter, GPU load %, temperature (для power users).

### 32.2 Energy Manager
- **Battery monitoring:** кроссплатформенное API для уровня заряда, статуса (charging/discharging), estimated time.
- **Политики по уровню заряда:**
  - **100–50% (Normal):** полная производительность, 60 FPS, все эффекты.
  - **50–20% (Power Save):** 30 FPS, отключение blur/shadows, pause background sync, увеличение P2P announce interval.
  - **20–10% (Critical):** сохранение checkpoint'ов, закрытие background apps, остановка P2P, минимальная яркость, только emergency notifications.
  - **10–0% (Emergency):** graceful shutdown с сохранением всех данных.
- **Plug detection:** при подключении зарядки — мгновенный возврат к Normal.
- **TTS feedback:** "Включён режим энергосбережения" (этап 26).

### 32.3 Native Process Monitor
- **Memory watchdog:** если нативный процесс (Game Mode) > 85% RAM — graceful suspend → checkpoint → kill.
- **GPU fence:** если GPU fence не сигнализирует 2+ секунды — TDR (Timeout Detection Recovery), принудительное закрытие.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Game Mode enter | Вход | Запрос → direct surface, input exclusivity < 33 мс |
| Game Mode exit | Panic | Ctrl+Shift+Esc → мгновенный возврат в Shell |
| Alt+Tab in Game | Shadow | Alt+Tab → window switcher поверх frozen игры |
| Energy normal | Полный | 100% battery → 60 FPS, все эффекты |
| Energy power save | Экономия | 30% battery → 30 FPS, blur отключён |
| Energy critical | Критический | 15% battery → checkpoint, background killed |
| GPU fence TDR | Зависание | GPU hang 2 сек → принудительное закрытие |

## Интеграция с будущими этапами
- **Вход:** этап 11 (Compositor) — direct surface, shadow framebuffer.
- **Вход:** этап 18 (Window Manager) — fullscreen states.
- **Выход:** Game Mode API → этап 23 (V8 Isolate, level 4–5 apps).
- **Выход:** TTS → этап 26 (Voice Pipeline).

## Критерии приёмки
- [ ] Game Mode: вход < 33 мс, выход (Panic) мгновенно.
- [ ] Alt+Tab в Game Mode: shadow framebuffer + window switcher.
- [ ] Energy: 4 режима, автопереключение.
- [ ] Plug detection: подключение зарядки → Normal.
- [ ] GPU fence TDR: принудительное закрытие при зависании.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Game Mode, Energy
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Game Mode §3.6, Energy §4.10

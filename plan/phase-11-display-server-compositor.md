# Этап 11 — Display Server: Compositor & Game Mode

## Цель
Создать полноценный композитор окон (scene graph, damage tracking, effects) и реализовать Game Mode — режим прямого доступа к GPU для приложений. После этого этапа Workspace умеет отображать несколько окон с наложением, анимациями, blur, и переключаться в эксклюзивный игровой режим.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `wgpu` (compute shaders для blur/effects), `guillotiere` (2D rectangle packing для слоёв), `rustc-hash` (быстрые хеш-таблицы для damage tracking)
- **Шейдеры:** WGSL (blur, blend modes, HDR tone mapping)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 9** — Display Server: Core (swapchain, surface, base pipeline).
- **Этап 10** — Display Server: 2D (примитивы, текст, текстуры).

## Часть системы
**Level 3 — Display Server** [См. layer-8 §3.5–3.7, layer-1 §4.5, layer-3 §2.1]

## Требования

### 9.1 Scene Graph
- Древовидная структура слоёв: каждый узел — `Layer` с transform (translate, scale, opacity), clip region, и content (texture или поддерево).
- **Window Layer:** содержит содержимое окна приложения (текстура, полученная от V8 Isolate или Island Mode).
- **Chrome Layer:** рамка окна, заголовок, кнопки (close, minimize, maximize) — рендерятся Display Server, а не приложением.
- **Overlay Layer:** Command Bar, уведомления, tooltips, Static UI Overlay (при CPU starvation).
- **Compositor traverses** scene graph в порядке z-index, рендерит каждый слой в framebuffer через post-processing stack.

### 9.2 Damage Tracking
- **Invalidation regions:** каждый слой сообщает композитору, какая область изменилась.
- **Clip and merge:** пересечение damage regions с clip-регионами родителей. Merge смежных прямоугольников для минимизации draw calls.
- **Full-screen damage:** первый кадр, resize, эффект размытия (blur затрагивает соседние пиксели), и переключение окон.
- **Оптимизация:** если damage < 30% экрана — рендерить только изменённые области через scissor rects.

### 9.3 Effects
- **Blur (backdrop blur):** Gaussian blur фона под полупрозрачным слоем. Реализация через dual Kawase blur в compute shader.
- **Shadows (drop shadow):** для окон и модальных диалогов. Gaussian blur + offset.
- **Opacity transitions:** tween opacity между кадрами (0 → 1 за 150 мс для появления окна).
- **Brightness/Contrast:** для Accessibility (high contrast mode, этап 27).

### 9.4 Game Mode
- **Direct Surface:** когда приложение запрашивает Game Mode (через API этапа 27), Display Server создаёт dedicated surface, который напрямую рендерит приложение в swapchain без композитора.
- **Input Exclusivity:** все события ввода (клавиатура, мышь, геймпад) направляются напрямую в приложение, минуя Command Bar и Window Manager.
- **Shadow Framebuffer:** перед входом в Game Mode композитор захватывает текущий framebuffer в GPU-текстуру (shadow framebuffer). При переключении обратно (Alt+Tab, Panic Gesture) — shadow framebuffer отображается мгновенно, пока приложение не возобновило рендеринг [См. layer-1 §4.5, layer-8 §3.6].
- **Переключение:**
  - Shell → Game Mode: 1–2 кадра (16–33 мс).
  - Game Mode → Shell (Panic Gesture): мгновенно (shadow framebuffer).
  - Game Mode → Shell (Alt+Tab): shadow framebuffer + window switcher overlay.
  - Обратно в игру: < 100 мс (resume checkpoint isolate).
- **Panic Gesture в Game Mode:** всегда работает, вызывает немедленный exit из Game Mode через Host Shim (уровень железа).

### 9.5 VSync и Frame Pacing
- **Adaptive VSync:** если frame time < 16.67 мс — ждать VSync. Если > 16.67 мс — отображать сразу (чтобы не усугублять задержку).
- **Frame timer:** точный подсчёт `presentation_time` для анимаций.
- **GPU fence:** отслеживание завершения GPU-работы. Если fence не сигнализирует в течение 2 секунд — TDR (timeout detection and recovery), приложение принудительно закрывается [См. layer-8 §4.3.2].

### 9.6 Accessibility Overlay
- **High Contrast:** инверсия цветов или повышение контраста через post-processing shader.
- **Magnifier:** увеличение области под курсором через отдельный render pass с upscale.
- **Screen Reader hooks:** Display Server предоставляет API для получения текстовых элементов в текущем layout (bounding box + text content) для screen reader (этап 27).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Two windows | Два окна наложены | Запуск → два окна видны, z-order корректен |
| Damage tracking | Частичный redraw | Изменить текст в одном окне → только оно перерисовано |
| Blur effect | Backdrop blur | Открыть полупрозрачное окно → фон размыт |
| Window open animation | Opacity tween | Открыть окно → плавное появление 150 мс |
| Game Mode enter | Direct surface | Запрос Game Mode → fullscreen, input захвачен |
| Game Mode exit | Panic Gesture | Ctrl+Shift+Esc → мгновенный возврат в Shell |
| Alt+Tab in Game | Shadow framebuffer | Alt+Tab → window switcher поверх frozen игры |

## Интеграция с будущими этапами
- **Вход:** этап 7 (Core) — swapchain, surface.
- **Вход:** этап 8 (2D) — примитивы, текст, текстуры для chrome и overlay.
- **Вход:** этап 15 (Window Manager) — запросы на открытие/закрытие/перемещение окон.
- **Вход:** этап 20 (V8 Isolate) / 21 (Island Mode) — текстуры содержимого окон.
- **Вход:** этап 27 (Game Mode API, Energy, Accessibility) — запросы на вход/выход в Game Mode, high contrast, magnifier.
- **Выход:** `CompositorFrame` → swapchain present.
- **Выход:** Game Mode direct surface → GPU swapchain.

## Критерии приёмки
- [ ] Два окна рендерятся с корректным z-order и наложением.
- [ ] Damage tracking снижает GPU load при частичных изменениях (измеряется через GPU timer).
- [ ] Blur эффект визуально корректен (Gaussian blur radius 8px).
- [ ] Анимация открытия окна: 150 мс, 60 FPS, без рывков.
- [ ] Game Mode: вход < 33 мс, выход (Panic Gesture) мгновенно.
- [ ] Game Mode Alt+Tab: shadow framebuffer отображается, window switcher работает.
- [ ] GPU fence TDR: принудительное закрытие приложения при зависании GPU > 2 сек.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Game Mode, окна, эффекты
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Graceful Degradation, Static UI Overlay
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Display Server §3.5–3.7, Game Mode §3.6

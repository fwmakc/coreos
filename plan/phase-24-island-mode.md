# Этап 24 — Island Mode

## Цель
Создать Island Mode — sandbox для веб-контента (HTML/CSS/JS) и legacy-приложений на базе встроенного Chromium (CEF) или аналога. После этого этапа пользователь может открывать веб-сайты и веб-приложения в изолированном окне с ограниченным доступом к системе.

## Язык и стек
- **Язык:** TypeScript (интеграция), C++ (CEF/Chromium embedding)
- **Runtime:** Bun + CEF (Chromium Embedded Framework) или альтернатива (Webview2 на Windows, WebKit на macOS, WebKitGTK на Linux)
- **Ключевые зависимости:** CEF binary distribution, `cef-rs` (Rust bindings) или нативные биндинги к платформенным webview
- **Целевые ОС:** Windows (WebView2/CEF), macOS (WebKit), Linux (WebKitGTK/CEF), Android (WebView)

## Зависимости
- **Этап 11** — Display Server: Compositor (текстура для embedding webview).
- **Этап 13** — Micro-Kernel: Security (capability checks для web-контента).
- **Этап 22** — App Registry (workspace.json для level 1–2 приложений).
- **Этап 23** — V8 Isolate Runtime (shared sandbox infrastructure).

## Часть системы
**Level 2 — Island Mode** [См. layer-8 §3.2, layer-6 §2.1, layer-11 §App Model]

## Требования

### 21.1 WebView Embedding
- **Архитектура:** Island Mode — это не отдельное окно браузера, а текстура внутри окна Workspace. WebView рендерит свое содержимое в off-screen surface, который передаётся Display Server (этап 9) как `WindowLayer` texture.
- **Интеграция с Display Server:**
  - WebView предоставляет `SharedTexture` (DMA-BUF на Linux, IOSurface на macOS, DXGI shared handle на Windows).
  - Display Server композитит эту текстуру как обычное окно приложения (с chrome, shadows, blur).
- **Input forwarding:** клавиатура и мышь, попадающие в область окна Island Mode, пересылаются WebView через нативный API.

### 21.2 Platform WebView Strategy
- **Windows:** WebView2 (Edge Chromium) — рекомендуется. Fallback на CEF если WebView2 недоступен.
- **macOS:** WKWebView (WebKit) через `objc` FFI.
- **Linux:** WebKitGTK (WPE WebKit) через GObject FFI. Fallback на CEF.
- **Android:** `android.webkit.WebView` через JNI.
- **Унификация:** все платформы предоставляют одинаковый интерфейс `IslandEngine` с методами:
  - `create(url, size) -> IslandId`
  - `navigate(island_id, url)`
  - `go_back()`, `go_forward()`
  - `reload()`
  - `execute_js(island_id, script) -> Result`
  - `capture_screenshot(island_id) -> Image`
  - `set_zoom(zoom)`
  - `destroy(island_id)`

### 21.3 Sandbox
- **Process isolation:** WebView запускается в отдельном процессе (renderer process) от основного процесса Workspace.
- **Network sandbox:** WebView имеет доступ только к `http/https`. Доступ к `file://` запрещён (за исключением sandboxed app bundle).
- **Capability bridge:** WebView не имеет прямого доступа к `@workspace/*` API. Взаимодействие через `postMessage` bridge:
  - WebView: `window.parent.postMessage({ type: 'WORKSPACE:fs:read', path: '...' }, '*')`
  - Workspace: проверка capability → выполнение → `postMessage` обратно с результатом.
- **Cookie isolation:** cookies WebView не пересекаются с системой и между разными Island Mode окнами (если не настроено иначе).

### 21.4 workspace.json Integration (Level 1–2)
- **Level 1:** "Как есть" — просто URL. Нет манифеста, нет `@WORKSPACE` доступа.
- **Level 2:** "Манифест" — `workspace.json` + URL. Приложение может запрашивать capabilities через `postMessage` bridge. Установка = создание ярлыка с URL и манифестом.

### 21.5 DevTools
- **Remote DevTools:** для разработки — подключение Chrome DevTools к WebView через remote debugging port (опционально, включается флагом `--workspace-dev`).
- **Inspector overlay:** нажатие F12 в dev-режиме открывает DevTools в отдельном окне.

### 21.6 Error Handling
- **Navigation error:** если URL недоступен — отображается страница ошибки с кнопкой "Повторить" и "Назад".
- **SSL error:** самоподписанный сертификат — предупреждение (можно продолжить для localhost).
- **Crash:** если renderer process падает — перезапуск с предложением восстановить вкладку.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Open URL | Открыть сайт | Открыть example.com → страница загружена |
| Input forward | Ввод | Нажать клавишу внутри Island → WebView получает |
| Scroll | Скролл | Скролл колесом → страница скроллится |
| postMessage bridge | @WORKSPACE API | Вызвать `WORKSPACE:fs:read` → capability check → результат |
| Screenshot | Скриншот | Запросить capture → PNG получен |
| DevTools | Отладка | F12 → DevTools подключены |
| Process isolation | Процесс | WebView renderer — отдельный PID |

## Интеграция с будущими этапами
- **Вход:** этап 9 (Compositor) — shared texture compositing.
- **Вход:** этап 11 (Security) — capability checks для bridge.
- **Вход:** этап 19 (App Registry) — level 1–2 apps, workspace.json.
- **Вход:** этап 20 (V8 Isolate) — shared sandbox infrastructure, permissions UI.
- **Выход:** `postMessage` → этап 20 (V8 Isolate API bridge).
- **Выход:** screenshot → этап 24 (Voice, Zero UI: "отправь скриншот").

## Критерии приёмки
- [ ] Открытие URL: страница загружается, рендерится в окне WORKSPACE.
- [ ] Input forwarding: клавиатура и мышь работают внутри WebView.
- [ ] Scroll: smooth scroll через тачпад/колесо.
- [ ] postMessage bridge: запрос `WORKSPACE:fs:read` с правом — успех, без права — отказ.
- [ ] Screenshot: PNG корректного размера.
- [ ] DevTools: подключение работает в dev-режиме.
- [ ] Process isolation: renderer process — отдельный PID (проверка через task manager).
- [ ] 5 вкладок одновременно — стабильность.

## Ссылки
- [layer-6-apps.md](../layers/layer-6-apps.md) — Island Mode, Level 1–2
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Island Mode §3.2
- [layer-11-developer-reference.md](../layers/layer-11-developer-reference.md) — App Model, 5 уровней

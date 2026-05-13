# Тесты: Этап 24 — Island Mode

> CEF/WebKit embedding, cookie isolation, storage isolation, GPU acceleration, file upload, level 1/2 apps. Все тесты на реальных веб-страницах.

---

### TC-24-001: CEF embed — open page
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `island.open("https://example.com")`.
**Ожидаемый результат:**
- CEF surface создан. HTML рендерится.
**Автоматизация:** автоматический.

### TC-24-002: CEF embed — navigate
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Open example.com. 2. Navigate to example.org.
**Ожидаемый результат:**
- Новая страница загружена.
**Автоматизация:** автоматический.

### TC-24-003: CEF embed — reload
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Reload.
**Ожидаемый результат:**
- Страница перезагружена.
**Автоматизация:** автоматический.

### TC-24-004: CEF — JavaScript execution
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Execute `document.title`.
**Ожидаемый результат:**
- Title возвращён.
**Автоматизация:** автоматический.

### TC-24-005: Cookie isolation — separate islands
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Island A: login. 2. Island B: open same site.
**Ожидаемый результат:**
- B: не залогинен.
**Автоматизация:** автоматический.

### TC-24-006: Cookie isolation — same island
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Island A: login. 2. Island A: navigate. 3. Check login.
**Ожидаемый результат:**
- Залогинен.
**Автоматизация:** автоматический.

### TC-24-007: Storage isolation — localStorage
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Island A: `localStorage.setItem("key", "A")`.
2. Island B: `localStorage.getItem("key")`.
**Ожидаемый результат:**
- B: `null`.
**Автоматизация:** автоматический.

### TC-24-008: Storage isolation — IndexedDB
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Island A: write to IndexedDB. 2. Island B: read.
**Ожидаемый результат:**
- B: empty.
**Автоматизация:** автоматический.

### TC-24-009: Storage isolation — sessionStorage
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Island A: `sessionStorage.setItem("key", "A")`.
**Ожидаемый результат:**
- Persisted per island session.
**Автоматизация:** автоматический.

### TC-24-010: GPU acceleration — WebGL
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Open WebGL demo.
**Ожидаемый результат:**
- FPS > 30. GPU accelerated.
**Автоматизация:** автоматический.

### TC-24-011: GPU acceleration — Canvas 2D
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Canvas 2D animation.
**Ожидаемый результат:**
- Smooth. GPU composited.
**Автоматизация:** автоматический.

### TC-24-012: File upload — select file
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `<input type="file">`. 2. Select file.
**Ожидаемый результат:**
- File доступен в Island. Нет доступа за пределы.
**Автоматизация:** автоматический.

### TC-24-013: File upload — drag & drop
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Drag file на Island.
**Ожидаемый результат:**
- File доступен.
**Автоматизация:** автоматический.

### TC-24-014: Level 1 app — simple HTML
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Open simple HTML (level 1).
**Ожидаемый результат:**
- Работает. Нет permission requests.
**Автоматизация:** автоматический.

### TC-24-015: Level 2 app — SPA with workspace.json
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Open SPA. 2. Request `fs:read`.
**Ожидаемый результат:**
- Permission dialog. После allow — доступ.
**Автоматизация:** автоматический.

### TC-24-016: Level 2 app — permission denied
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Request `fs:read`. 2. Deny.
**Ожидаемый результат:**
- `CapabilityError`. App continues.
**Автоматизация:** автоматический.

### TC-24-017: Download — file
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Trigger download.
**Ожидаемый результат:**
- File saved to VFS. User notified.
**Автоматизация:** автоматический.

### TC-24-018: Print — dialog
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. `window.print()`.
**Ожидаемый результат:**
- Print dialog. Или PDF to VFS.
**Автоматизация:** автоматический.

### TC-24-019: WebKit fallback — macOS/iOS
**Тип:** E2E | **Платформа:** macOS/iOS | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. На macOS: WebKit (не CEF).
**Ожидаемый результат:**
- WKWebView создан. Рендерится.
**Автоматизация:** автоматический.

### TC-24-020: WebKit — iOS specific
**Тип:** E2E | **Платформа:** iOS | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Open page on iPhone.
**Ожидаемый результат:**
- Touch events работают. Scroll smooth.
**Автоматизация:** ручной.

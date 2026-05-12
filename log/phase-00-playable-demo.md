# Phase 0 — Playable Demo / Доказательство концепции

> **Начало:** 2026-05-12  
> **Цель:** Осязаемый артефакт, который можно запустить, поклацать мышкой и клавиатурой, и увидеть реакцию системы.  
> **Срок:** 3–4 недели  
> **Статус:** 🚧 In Progress

---

## 2026-05-12 — Старт Phase 0

### Контекст
Проект находится на стадии Foundation. Rust-workspace инициализирован, базовые тесты написаны (120 passed), но интерактивности нет. Решено ускорить выход к первому осязаемому результату, введя Phase 0 — минимальный playable demo поверх готовых winit + wgpu.

### Что сделано
- [x] Создана документация Phase 0 (`plan/phase-00-playable-demo.md`)
- [x] Создана система development log (`log/README.md`, этот файл)
- [x] Обновлен `PROJECT_STATUS.md` — текущий фокус переключен на demo
- [x] Обновлен `CHANGELOG.md` — запись о начале Phase 0
- [x] Обновлен `plan/roadmap.md` и `plan/README.md` — Phase 0 внесена в план

### Результаты тестов (до начала работы над демо)
```bash
cd src && cargo test --all-targets
# test result: ok. 120 passed; 0 failed; 8 ignored; 0 measured; 0 filtered out
```

### Архитектура демо
```
┌─────────────────────────────────────────────┐
│  winit Window (800×600)                     │
│  ┌─────────────────────────────────────┐    │
│  │  wgpu Surface + Swapchain           │    │
│  │  ┌─────────────────────────────┐    │    │
│  │  │  Background (brand color)   │    │    │
│  │  │  ┌─────┐                    │    │    │
│  │  │  │Cursor │ ← follows mouse   │    │    │
│  │  │  └─────┘                    │    │    │
│  │  │  Click → spawn circle      │    │    │
│  │  │  Keyboard → print glyph    │    │    │
│  │  └─────────────────────────────┘    │    │
│  │  ┌─────────────────────────────┐    │    │
│  │  │  Command Bar (bottom panel) │    │    │
│  │  │  [_____________________]    │    │    │
│  │  └─────────────────────────────┘    │    │
│  └─────────────────────────────────────┘    │
└─────────────────────────────────────────────┘
```

### Компоненты

| Компонент | Технология | Статус |
|-----------|-----------|--------|
| Window + Event Loop | winit 0.30 | ✅ Тесты есть (ignored на headless) |
| GPU Surface | wgpu 22 | ✅ Тесты есть (ignored без GPU) |
| Background clear | wgpu render pass | 🚧 Не реализовано |
| Cursor (quad sprite) | wgpu + vertex buffer | 🚧 Не реализовано |
| Click → circle | wgpu + instancing | 🚧 Не реализовано |
| Text rendering | cosmic-text + wgpu | 🚧 Не реализовано |
| Command Bar panel | wgpu + rect rendering | 🚧 Не реализовано |

### Блокеры
- Нет реального рендер-пайплайна wgpu (только тесты инициализации)
- Нет интеграции winit event loop с wgpu surface (resize, redraw)

### Следующий шаг
- [ ] Создать `demo/` crate в workspace
- [ ] Реализовать wgpu render loop: clear → present (60 FPS)
- [ ] Подключить winit events (WindowEvent::CursorMoved, MouseInput, KeyboardInput)
- [ ] Нарисовать курсор-квадрат, следующий за мышью

---

*Последнее обновление: 2026-05-12*

# CORE OS — MVP Plan

## Scope

Два параллельных трека, 3 месяца, одна точка схода.

### Трек 1: Core Runtime (убийца Electron)
Rust + Bun + WebGPU. Рантайм для запуска JS/TS-приложений с нативной скоростью.

### Трек 2: Core Shell (Project-based UI)
Интерфейс поверх Runtime. Поисковая строка, проекты, теги, голосовой ввод.

### Точка схода (конец месяца 3)
Runtime рисует Shell через WebGPU. Shell запускает 1-2 приложения. Бенчмарк vs Electron.

---

## Out of Scope (не входит в MVP)

- P2P, CRDT, Mesh, синхронизация
- TSCLANG
- eBPF, XDP
- Мультипользовательность (пока 1 юзер)
- Бэк-офис, железки
- Мобильные платформы (пока desktop только)
- Island Mode (Chromium)
- 3 уровня UI-свободы (пока только Level 3 — Core Design System)

---

## Timeline

| Месяц | Трек 1 (Runtime) | Трек 2 (Shell) |
|-------|-------------------|----------------|
| 1 | Пустое WebGPU окно, event loop, ввод | Макеты, дизайн-система, прототип на заглушках |
| 2 | V8 Isolate Manager, IPC, базовые компоненты | Shell на WebGPU, навигация, анимации |
| 3 | Layout Engine, демо-приложение, бенчмарк | Проекты, теги, поиск, голосовой ввод |

---

## Key Metrics for MVP

| Метрика | Цель |
|---------|------|
| RAM idle | < 20 МБ |
| RAM с 1 приложением | < 100 МБ |
| FPS | 60+ |
| Запуск приложения | < 200 мс |
| Размер бинарника | < 15 МБ |

## Documents

- [track1-runtime.md](track1-runtime.md) — Runtime: архитектура, компоненты, milestones
- [track2-shell.md](track2-shell.md) — Shell: архитектура, компоненты, milestones
- [repo-structure.md](repo-structure.md) — Структура репозитория
- [tech-decisions.md](tech-decisions.md) — Открытые технические вопросы

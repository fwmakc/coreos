# Тесты: Этап 37 — Documentation

> User docs, developer docs, admin docs, API reference, cross-references, localization, code comments. Все тесты на реальных документах.

---

### TC-37-001: User docs — completeness
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Проверить: все features описаны.
**Ожидаемый результат:**
- Нет orphan features. Every UI element documented.
**Автоматизация:** ручной (audit checklist).

### TC-37-002: User docs — quick start < 5 min
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Follow quick start on fresh VM.
**Ожидаемый результат:**
- CORE запущен < 5 мин.
**Автоматизация:** ручной.

### TC-37-003: Developer docs — API reference generated
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `bun run docs:generate`.
**Ожидаемый результат:**
- 100% `@workspace/*` APIs documented.
**Автоматизация:** автоматический.

### TC-37-004: Developer docs — API examples
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Check each API has example.
**Ожидаемый результат:**
- Every public API: usage example.
**Автоматизация:** автоматический.

### TC-37-005: Admin docs — setup guide
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Follow setup guide on fresh VM.
**Ожидаемый результат:**
- Backoffice запущен. All admin features work.
**Автоматизация:** ручной.

### TC-37-006: Admin docs — SSH setup
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Follow SSH setup.
**Ожидаемый результат:**
- Key auth works. No password.
**Автоматизация:** ручной.

### TC-37-007: Cross-references — no broken links
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `mdbook-linkcheck`.
**Ожидаемый результат:**
- Нет broken links.
**Автоматизация:** автоматический.

### TC-37-008: Cross-references — layer→plan→tests
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Check: every plan phase references tests.
**Ожидаемый результат:**
- Bidirectional links.
**Автоматизация:** автоматический.

### TC-37-009: Localization — completeness
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Check all strings localized (ru, en, zh, ja, ar, hi, es, de, fr, pt).
**Ожидаемый результат:**
- Нет untranslated strings.
**Автоматизация:** автоматический (i18n linter).

### TC-37-010: Localization — fallback
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Unknown locale "xx".
**Ожидаемый результат:**
- Fallback to en.
**Автоматизация:** автоматический.

### TC-37-011: Code comments — Rust docs
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `cargo doc`. 2. Check warnings.
**Ожидаемый результат:**
- 100% pub items documented. No warnings.
**Автоматизация:** автоматический.

### TC-37-012: Code comments — TypeScript docs
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `typedoc`. 2. Check coverage.
**Ожидаемый результат:**
- 100% exports documented.
**Автоматизация:** автоматический.

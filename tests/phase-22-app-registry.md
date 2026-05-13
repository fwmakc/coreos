# Тесты: Этап 22 — App Registry

> Install/update/remove, `workspace.json` validation, Ed25519 signatures, dependency resolution, sandbox. Все тесты на реальных приложениях.

---

### TC-22-001: Install — basic
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `core install ./test-app/`.
**Ожидаемый результат:**
- App зарегистрирована. `apps/` и SQLite.
**Автоматизация:** автоматический.

### TC-22-002: Install — `workspace.json` valid
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Valid `workspace.json`: name, version, entry, permissions.
**Ожидаемый результат:**
- Успешно.
**Автоматизация:** автоматический.

### TC-22-003: Install — `workspace.json` missing name
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `workspace.json` без `name`.
**Ожидаемый результат:**
- `ValidationError: missing field 'name'`.
**Автоматизация:** автоматический.

### TC-22-004: Install — `workspace.json` invalid version
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. version = "not-a-version".
**Ожидаемый результат:**
- `ValidationError: invalid semver`.
**Автоматизация:** автоматический.

### TC-22-005: Install — `workspace.json` invalid permissions
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. permissions = ["invalid:capability"].
**Ожидаемый результат:**
- `ValidationError: unknown capability`.
**Автоматизация:** автоматический.

### TC-22-006: Signature — valid Ed25519
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. App с подписью. 2. Verify.
**Ожидаемый результат:**
- Valid. App trusted.
**Автоматизация:** автоматический.

### TC-22-007: Signature — tampered code
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Изменить 1 байт в коде. 2. Verify.
**Ожидаемый результат:**
- `SignatureInvalid`. App не запускается.
**Автоматизация:** автоматический.

### TC-22-008: Signature — missing
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Без подписи.
**Ожидаемый результат:**
- `SignatureMissing`. Untrusted (если policy strict).
**Автоматизация:** автоматический.

### TC-22-009: Update — v1→v2
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Install v1.0. 2. Update v1.1.
**Ожидаемый результат:**
- v1.1 установлена. User data сохранена.
**Автоматизация:** автоматический.

### TC-22-010: Update — downgrade
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. v2.0. 2. Install v1.0.
**Ожидаемый результат:**
- Warning. Или denied (по политике).
**Автоматизация:** автоматический.

### TC-22-011: Update — breaking changes
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. v2.0 (breaking). 2. Auto-migrate.
**Ожидаемый результат:**
- Data migrated. Или prompt.
**Автоматизация:** автоматический.

### TC-22-012: Uninstall — basic
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `core uninstall test-app`.
**Ожидаемый результат:**
- App удалена из `apps/`. SQLite обновлён.
**Автоматизация:** автоматический.

### TC-22-013: Uninstall — keep data
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Uninstall. 2. Выбрать "Keep data".
**Ожидаемый результат:**
- App удалена. Data в `~/.core/apps/test-app/`.
**Автоматизация:** автоматический.

### TC-22-014: Uninstall — delete data
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Uninstall. 2. "Delete data".
**Ожидаемый результат:**
- Data удалена.
**Автоматизация:** автоматический.

### TC-22-015: Dependency — auto-install
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. A зависит от B. 2. Install A.
**Ожидаемый результат:**
- B установлена автоматически.
**Автоматизация:** автоматический.

### TC-22-016: Dependency — missing
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. A зависит от B (недоступна).
**Ожидаемый результат:**
- `DependencyError: App B not found`.
**Автоматизация:** автоматический.

### TC-22-017: Dependency — version conflict
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. A требует B^1.0. 2. C требует B^2.0.
**Ожидаемый результат:**
- Conflict resolution. Или error.
**Автоматизация:** автоматический.

### TC-22-018: Dependency — circular
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. A→B→A.
**Ожидаемый результат:**
- `CircularDependencyError`.
**Автоматизация:** автоматический.

### TC-22-019: List installed
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `core list`.
**Ожидаемый результат:**
- Список apps. Name, version, size.
**Автоматизация:** автоматический.

### TC-22-020: Search registry
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. `core search calculator`.
**Ожидаемый результат:**
- Результаты. Name, description, author.
**Автоматизация:** автоматический.

# Тесты: Этап 31 — Admin UI

> Workspace.Backoffice GUI, Workspace.Hardcore TUI/CLI, SSH admin, device management, health dashboard. Все тесты на реальных интерфейсах.

---

### TC-31-001: Backoffice — login
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Open Backoffice (port 8443). 2. Login.
**Ожидаемый результат:**
- Dashboard loaded.
**Автоматизация:** автоматический (playwright).

### TC-31-002: Backoffice — dashboard
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Check dashboard.
**Ожидаемый результат:**
- Users, devices, audit log, health visible.
**Автоматизация:** автоматический.

### TC-31-003: Backoffice — user list
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Navigate to Users.
**Ожидаемый результат:**
- User list. Name, role, last active.
**Автоматизация:** автоматический.

### TC-31-004: Backoffice — create user
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Create user "testuser". 2. Role: Member.
**Ожидаемый результат:**
- User created. Invitation sent.
**Автоматизация:** автоматический.

### TC-31-005: Backoffice — delete user
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Delete user. 2. Confirm.
**Ожидаемый результат:**
- User deleted. Data preserved (policy).
**Автоматизация:** автоматический.

### TC-31-006: Backoffice — audit log view
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Navigate to Audit.
**Ожидаемый результат:**
- Events listed. Filterable.
**Автоматизация:** автоматический.

### TC-31-007: Backoffice — audit export
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Export to CSV.
**Ожидаемый результат:**
- CSV downloaded.
**Автоматизация:** автоматический.

### TC-31-008: Backoffice — device list
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Navigate to Devices.
**Ожидаемый результат:**
- Devices: name, OS, last sync, status.
**Автоматизация:** автоматический.

### TC-31-009: Backoffice — remote wipe device
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Select device. 2. Remote wipe. 3. Confirm.
**Ожидаемый результат:**
- Device wiped. Audit: `REMOTE_WIPE`.
**Автоматизация:** автоматический.

### TC-31-010: Backoffice — health dashboard
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Health page.
**Ожидаемый результат:**
- CPU, RAM, disk, network. Alerts if threshold.
**Автоматизация:** автоматический.

### TC-31-011: Backoffice — alert threshold
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. CPU > 90%.
**Ожидаемый результат:**
- Alert. Red indicator.
**Автоматизация:** автоматический.

### TC-31-012: Hardcore TUI — status
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `ssh admin@device`. 2. `core-admin status`.
**Ожидаемый результат:**
- ASCII dashboard. CPU, RAM, uptime.
**Автоматизация:** автоматический (expect).

### TC-31-013: Hardcore TUI — user list
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `core-admin users`.
**Ожидаемый результат:**
- User table.
**Автоматизация:** автоматический.

### TC-31-014: Hardcore TUI — create user
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `core-admin user add testuser`.
**Ожидаемый результат:**
- User created.
**Автоматизация:** автоматический.

### TC-31-015: SSH — key auth
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `ssh -i key admin@device`.
**Ожидаемый результат:**
- Без пароля. Root shell.
**Автоматизация:** автоматический.

### TC-31-016: SSH — password disabled
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. `ssh admin@device` (без key).
**Ожидаемый результат:**
- `Permission denied (publickey)`.
**Автоматизация:** автоматический.

### TC-31-017: SSH — session timeout
**Тип:** Integration | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Idle 15 мин.
**Ожидаемый результат:**
- Connection closed.
**Автоматизация:** автоматический.

### TC-31-018: Role-based admin access
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P0
**Шаги:**
1. Member пытается открыть Backoffice.
**Ожидаемый результат:**
- `AccessDenied`.
**Автоматизация:** автоматический.

### TC-31-019: Backoffice — mobile responsive
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Open on mobile viewport.
**Ожидаемый результат:**
- Layout adapts. Scrollable.
**Автоматизация:** автоматический.

### TC-31-020: Backoffice — dark mode
**Тип:** E2E | **Платформа:** All | **Данные:** Реальные | **Приоритет:** P1
**Шаги:**
1. Toggle dark mode.
**Ожидаемый результат:**
- Theme switches.
**Автоматизация:** автоматический.

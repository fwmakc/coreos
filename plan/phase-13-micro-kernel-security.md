# Этап 13 — Micro-Kernel: Security Engine

## Цель
Встроить в Micro-Kernel систему безопасности на основе capabilities: каждый API-вызов проверяется на наличие права, приложения работают в sandbox, пользователи имеют роли. После этого этапа Workspace защищает данные от несанкционированного доступа на уровне ядра.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (хранение ролей и capability-грантов), `tweetnacl` или `@noble/ed25519` (криптография), `bun:ffi` (для вызова Rust-crypto если нужно)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite schema, event loop, IPC).

## Часть системы
**Level 1 — Micro-Kernel: Capability Security** [См. layer-7 §22, layer-8 §4.4, layer-6 §3]

## Требования

### 11.1 Capability Model
- **Capability:** строка вида `resource:action`, например `fs:read`, `network:http`, `graphics:render`.
- **Context object:** каждый API-вызов внутри isolate получает `context` с набором capabilities, выданных при установке приложения.
- **Проверка:** перед выполнением любого системного действия Micro-Kernel вызывает `checkCapability(context, 'resource:action')`.
- **Отказ:** если capability отсутствует — `CapabilityError` с человекочитаемым сообщением и предложением запросить право.

### 11.2 Роли (RBAC)
- **Встроенные роли:**
  - `Owner` — все capabilities.
  - `Member` — `fs:read`, `fs:write`, `network:http`, `graphics:render`, `contacts:read`, `notifications:send`.
  - `Guest` — `fs:read` (только собственные файлы), `network:http`, `graphics:render`.
- **Кастомные роли:** Owner может создавать роли с произвольным набором capabilities.
- **Наследование:** роль в проекте ограничивает роль в Space. Роль группы добавляется к роли пользователя.
- **Хранение:** таблица `roles` в SQLite.

### 11.3 Sandbox (базовый)
- **V8 Isolate sandbox:** каждое приложение запускается в отдельном V8 Isolate (этап 20). На этом этапе — подготовка sandbox-политик:
  - Какие `global` объекты доступны (console, fetch — только если `network:http`).
  - Доступ к `process`, `require`, `eval` — запрещён по умолчанию.
  - Таймауты: `setTimeout`/`setInterval` ограничены (max 100 активных таймеров).
- **Filesystem sandbox:** приложение видит только `WORKSPACE_ROOT/apps/<app-id>/` и explicitly granted папки.
- **Network sandbox:** приложение может делать HTTP-запросы только к разрешённым доменам (whitelist).

### 11.4 Permissions UI (подготовка)
- **Диалог запроса прав:** когда приложение запрашивает отсутствующее capability, Micro-Kernel генерирует событие `PermissionRequest`.
- **На этом этапе:** обработчик `PermissionRequest` записывает запрос в лог и отвечает авто-отказом (placeholder). Полный UI диалога — в этапе 20 (V8 Isolate Runtime).
- **Формат запроса:** `{ app_id, capability, reason, urgency }`.

### 11.5 Audit Logging
- Каждый `checkCapability` логируется в `audit_log` (таблица из этапа 10).
- Запись: `{ timestamp, category: 'security', action: 'capability.check', resource, user_id, result }`.
- **13 категорий аудита:** Auth, Roles, Projects, Files, Notes, Tags, Messenger, Search, Apps, Browser, Profiles, System, MultiBack [См. layer-7 §22.2].

### 11.6 Guest Mode
- **Guest Profile:** временный профиль, данные которого не сохраняются на диск (RAM-only SQLite).
- **Guest capabilities:** только `fs:read` (временные файлы), `network:http`, `graphics:render`.
- **Выход из Guest:** удаление всех данных, zeroize памяти.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Capability check | Проверка права | Запросить `fs:write` с контекстом Guest → отказ |
| Role assignment | Назначение роли | Назначить Member → проверка `fs:write` → успех |
| Sandbox FS | Ограничение ФС | Guest читает файл за пределами песочницы → отказ |
| Permission request | Запрос права | Приложение запрашивает `camera:capture` → лог + авто-отказ |
| Audit log | Журнал | 10 проверок → 10 записей в audit_log |
| Guest profile | RAM-only | Создать Guest → SQLite в памяти, после выхода — пусто |

## Интеграция с будущими этапами
- **Вход:** этап 10 — SQLite, event loop.
- **Выход:** `checkCapability()` → этап 12 (VFS) для проверки доступа к файлам.
- **Выход:** `checkCapability()` → этап 20 (V8 Isolate Runtime) для sandbox.
- **Выход:** audit log → этап 26 (RBAC + Audit full implementation).
- **Вход:** этап 20 — запросы на capabilities от приложений.

## Критерии приёмки
- [ ] Guest не может писать файлы (отказ с логом).
- [ ] Member может писать в свою песочницу.
- [ ] Owner может всё.
- [ ] Кастомная роль с 3 capabilities работает корректно.
- [ ] Каждый отказ логируется в audit_log.
- [ ] Guest profile — RAM-only, после перезапуска данные отсутствуют.
- [ ] 1000 capability checks/сек — latency < 0.1 мс.

## Ссылки
- [layer-7-security.md](../layers/layer-7-security.md) — RBAC, capabilities, sandbox, audit
- [layer-6-apps.md](../layers/layer-6-apps.md) — Permissions UI, 5 уровней приложений
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Capability Security §4.4

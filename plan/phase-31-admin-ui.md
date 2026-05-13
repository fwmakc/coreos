# Этап 31 — Admin UI (Backoffice & Hardcore)

## Цель
Создать интерфейсы администрирования Workspace: графический (Core.Backoffice) и текстовый (Core.Hardcore TUI/CLI). После этого этапа Owner и администраторы могут управлять пользователями, ролями, аудитом, бэкапом и AI-настройками.

## Язык и стек
- **Язык:** TypeScript (Backoffice GUI, Hardcore CLI), Rust (Hardcore TUI)
- **Runtime:** Bun (GUI/CLI), native Rust (TUI)
- **Ключевые зависимости:** `ratatui` (TUI), `clap` (CLI parser), `@core/ui` (Backoffice components)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 11** — Display Server: Compositor (Overlay Layer, chrome, post-processing).
- **Этап 18** — Window Manager (window states, fullscreen).
- **Этап 30** — Security Core (RBAC, Audit, Key Manager, remote wipe).
- **Все предыдущие этапы** (1–30).

## Часть системы
**Level 1 — Бэк/Фронт: Admin UI** [См. layer-3 §3, layer-8 §16]

## Требования

### 31.1 Core.Backoffice (GUI)
- **Назначение:** графический интерфейс администрирования для Owner.
- **Доступ:** через Command Bar (`> backoffice`) или ярлык.
- **Разделы:**
  - **Users:** список, создание, удаление, назначение ролей, блокировка.
  - **Spaces:** управление Spaces, перенос проектов между ними, архивация.
  - **Apps:** установленные приложения, обновления, permissions, blacklist.
  - **Security:** RBAC редактор (drag-drop capabilities), audit log viewer (фильтры по дате/категории/пользователю, export CSV/JSON), Key Manager status (TPM presence, key health).
  - **Backup:** запуск backup (full/incremental), restore wizard, просмотр истории backup'ов.
  - **AI:** настройки SLM (выбор модели), Cloud Bridge (on/off, API key), voice models.
  - **Support:** remote support session (relay-сервер + WireGuard tunnel для техподдержки).
- **Рендеринг:** через `@core/ui` (этап 23) → Display Server.
- **Корпоративный режим:** если `allow_gui_admin: false` — Core.Backoffice не устанавливается, единственный способ — Core.Hardcore [См. layer-3 §3.2].

### 31.2 Core.Hardcore (TUI)
- **SSH server:** `russh`, порт 2222 (не конфликтует с системным 22).
- **Аутентификация:** Ed25519 public key или recovery phrase (no passwords).
- **TUI:** `ratatui` с экранами:
  - Main menu (Users, Security, Backup, AI, System, Exit).
  - Users (таблица: Name, Role, Last Login, Actions).
  - Security (audit log с пагинацией, фильтры).
  - Backup (список, запуск, статус прогресса).
  - System (CPU%, RAM%, GPU%, network throughput, uptime).
- **Navigation:** arrow keys, Enter, Esc (back), q (quit).

### 31.3 Core.Hardcore (CLI)
- `core-cli user add --name "Иван" --role developer`
- `core-cli user list`
- `core-cli user revoke --name "Иван"`
- `core-cli role create --name "developer" --capabilities "fs.read,fs.write"`
- `core-cli role assign --user "Иван" --role "developer" --resource "project-alpha"`
- `core-cli backup --full --target usb`
- `core-cli restore --target usb --date 2025-01-15`
- `core-cli audit query --category "auth" --from "2025-01-01" --to "2025-01-31"`
- `core-cli settings set --key "security.level" --value "enhanced"`
- `core-cli ai model set --asr "whisper-medium" --nlu "llama-3.1-8b"`
- `core-cli remote-wipe --device-id "..."` (требует recovery phrase + confirmation).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Backoffice GUI | Админка | Открыть → список пользователей, audit log |
| Hardcore SSH | Удалённый доступ | SSH localhost:2222 → core-cli работает |
| Hardcore TUI | Интерфейс | Запустить → меню, навигация стрелками |
| Audit export | Экспорт | Экспорт audit → CSV содержит все записи |
| Remote wipe | Удаление | `core-cli remote-wipe` → устройство zeroize'd |

## Интеграция с будущими этапами
- **Вход:** этап 11 (Compositor) — Overlay Layer для Backoffice.
- **Вход:** этап 30 (Security) — RBAC, Audit, Key Manager.
- **Выход:** управление всеми предыдущими этапами.

## Критерии приёмки
- [ ] Backoffice: все 7 разделов работают, рендерятся через `@core/ui`.
- [ ] Hardcore SSH: подключение, аутентификация, команды.
- [ ] Hardcore TUI: навигация, выполнение команд.
- [ ] Audit export: JSON/CSV корректны.
- [ ] Remote wipe: устройство zeroize'd и rebooted.

## Ссылки
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Core.Backoffice, Core.Hardcore
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Admin §16

# Этап 30 — Security WORKSPACE

## Цель
Встроить в систему полноценную безопасность: RBAC, аудит (13 категорий), Key Manager (Ed25519, TPM/Secure Enclave), Session Management (TTL, auto-lock, remote wipe), и Secure Transaction API. После этого этапа Workspace защищена на всех уровнях: аутентификация, авторизация, шифрование, аудит.

## Язык и стек
- **Язык:** TypeScript (логика), Rust (криптография через FFI если needed)
- **Runtime:** Bun
- **Ключевые зависимости:** `@noble/ed25519`, `@noble/hashes` (BLAKE3, SHA-256), `bip39` (recovery phrase), `bun:sqlite` (audit log, keys)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop).
- **Этап 13** — Micro-Kernel: Security (capability model, base RBAC).
- **Этап 14** — Micro-Kernel: VFS (файлы для audit export).
- **Этап 21** — Backup Engine (recovery phrase, encryption).

## Часть системы
**Level 1 — Бэк: Security Infrastructure** [См. layer-7 §22, layer-8 §4.4, §8, §15, layer-3 §3]

## Требования

### 26.1 Key Manager
- **Recovery Phrase (BIP-39):** 24 слова, генерируются при first run. От них derivates master key.
- **Device Key:** Ed25519 keypair, генерируется при первом запуске, привязан к устройству.
- **Profile Key:** HKDF(master_key + profile_id) — для шифрования данных профиля.
- **Key Storage:**
  - **Windows:** TPM 2.0 или Credential Guard (software fallback если TPM недоступен).
  - **macOS:** Secure Enclave (Keychain).
  - **Linux:** TPM 2.0 (tss2) или software keyring (Keyring / KWallet).
  - **Android:** Android Keystore.
- **Signing:** Ed25519 подпись для всех критичных операций (backup manifest, app packages, CRDT ops).
- **Encryption:** XChaCha20-Poly1305 для шифрования blob'ов и SQLite.

### 26.2 RBAC (Full)
- **Роли:** Owner, Member, Guest + кастомные.
- **Наследование:** project < space < group. Роль группы добавляется к роли пользователя.
- **Resources:** файлы, проекты, Spaces, приложения, настройки.
- **Actions:** create, read, update, delete, execute, share, admin.
- **Audit:** каждое изменение роли логируется в `audit_log`.

### 26.3 Audit (13 категорий)
- **Категории:** Auth, Roles, Projects, Files, Notes, Tags, Messenger, Search, Apps, Browser, Profiles, System, MultiBack.
- **Запись:** каждое действие в категории логируется:
  - `timestamp`, `category`, `user_id`, `action`, `resource`, `result` (success/denied/error), `details` (JSON), `ip_address`, `device_id`.
- **Хранение:** SQLite `audit_log` (append-only WAL).
- **Query:** фильтрация по категории, пользователю, дате, результату.
- **Export:** JSON, CSV. Автоматическая ротация (max 1 ГБ, архивация старых).
- **Integrity:** audit log подписывается Ed25519. Подделка лога невозможна без приватного ключа.

### 26.4 Session Management
- **Session creation:** при входе (биометрия, PIN, или recovery phrase) создаётся session token.
- **TTL:** 30 минут по умолчанию. Refresh — при активности.
- **Auto-lock:** через 5 минут бездействия (настраивается).
- **Biometry:**
  - Windows Hello ( fingerprint / face).
  - macOS Touch ID / Face ID.
  - Android fingerprint / face.
  - Linux: placeholder (biometry не стандартизирована).
- **Remote Wipe:**
  - Owner может отправить команду `remote_wipe(device_id)` через P2P Mesh.
  - Устройство получает команду → zeroize всех ключей → удаление WORKSPACE.db → reboot.
  - **Confirmation:** wipe требует подтверждение через второй фактор (recovery phrase) на устройстве-инициаторе.

### 26.5 Secure Transaction API
- **Сценарий:** подпись документа или платёж.
- **WYSIWYS (What You See Is What You Sign):** Display Server рендерит overlay, который нельзя подделать приложением. Пользователь видит точное содержимое, которое будет подписано.
- **Подпись:** приватный ключ подписывает хеш содержимого + timestamp. Подпись хранится в audit log.
- **Enclave:** на устройствах с Secure Enclave / TPM — подпись выполняется внутри enclave, приватный ключ не покидает защищённую зону.

### 26.6 Incognito Mode
- **Анонимный профиль:** RAM-only, не сохраняется, не синхронизируется.
- **Network:** отдельный WireGuard ключ, трафик не смешивается с обычным профилем.
- **Cleanup:** при выходе из Incognito — zeroize RAM, удаление временных файлов.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Recovery phrase | Генерация | First run → 24 слова, derivation работает |
| Key storage | TPM/Secure Enclave | Ключ создан, не извлекаем (software fallback) |
| Role check | RBAC | Guest не может читать чужой файл |
| Audit log | Журнал | 10 действий → 10 записей, подписаны |
| Session TTL | Время жизни | Ждать 30 мин → сессия expired |
| Auto-lock | Блокировка | Ждать 5 мин → экран заблокирован |
| Remote wipe | Удаление | Отправить команду → устройство zeroize'd |
| Secure sign | Подпись | WYSIWYS overlay → подпись + audit запись |
| Incognito | Аноним | Открыть Incognito → RAM-only, no sync |

## Интеграция с будущими этапами
- **Вход:** этап 10 (WORKSPACE) — SQLite, event loop.
- **Вход:** этап 11 (Security base) — capability model.
- **Вход:** этап 18 (Backup) — recovery phrase, encryption.
- **Выход:** `checkRole()` → этап 12 (VFS), 14 (Project Manager), 19 (App Registry).
- **Выход:** audit log → этап 27 (Workspace.Backoffice) для просмотра.
- **Выход:** Key Manager → этап 17 (P2P) для WireGuard keys.

## Критерии приёмки
- [ ] Recovery phrase: BIP-39, 24 слова, master key derivation корректен.
- [ ] Key storage: device key создан, signature работает, ключ не извлекается (software fallback — хранится в SQLite с шифрованием).
- [ ] RBAC: 3 роли + кастомные, наследование работает.
- [ ] Audit: все 13 категорий логируются, export JSON/CSV работает.
- [ ] Session: TTL, auto-lock, remote wipe.
- [ ] Biometry: Windows Hello / Touch ID / Android fingerprint разблокирует.
- [ ] Remote wipe: устройство zeroize'd и rebooted.
- [ ] Secure sign: WYSIWYS overlay, подпись валидна, запись в audit.
- [ ] Incognito: RAM-only, no disk write, no sync.

## Ссылки
- [layer-7-security.md](../layers/layer-7-security.md) — RBAC, Audit, Key Manager, Session, Incognito
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Security §4.4, §8, §15
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Администрирование, Secure Transaction

# Этап 21 — Backup Engine

## Цель
Создать подсистему резервного копирования 3-2-1: локальные бэкапы на USB, облачные бэкапы на S3-совместимые хранилища, и P2P-бэкапы на доверенные устройства. После этого этапа пользователь может восстановить все данные после потери устройства.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (список бэкапов), `aws4fetch` или кастомный S3 client, `bun:ffi` (BLAKE3)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 7** — Host Shim: Storage (USB detection, file operations).
- **Этап 14** — Micro-Kernel: VFS (файлы, CID, Passport).
- **Этап 19** — CRDT Engine (oplog для инкрементального бэкапа).
- **Этап 20** — P2P Mesh (P2P backup target).

## Часть системы
**Level 1 — Бэк: Backup Engine** [См. layer-4 §4, layer-8 §4.8, layer-5 §3]

## Требования

### 18.1 Backup Targets
- **USB:** локальный USB-накопитель. Формат: папка `CORE_BACKUP_YYYYMMDD/` с зашифрованными blob'ами и SQLite snapshot.
- **S3:** облачное хранилище (AWS S3, Yandex Object Storage, MinIO). Данные шифруются перед отправкой (client-side encryption).
- **P2P:** доверенное устройство (другой компьютер или телефон пользователя). Бэкап передаётся через WireGuard.
- **Custom:** SFTP, WebDAV, Samba — через plugins (placeholder).

### 18.2 Backup Format
- **Snapshot:** полная копия `WORKSPACE.db` (SQLite) + все blob'ы из `blobs/`.
- **Incremental:** только новые/изменённые blob'ы (определяются по CID: если CID уже есть в backup — пропускается).
- **Deduplication:** backup хранит blob'ы по CID (как VFS). Одинаковые файлы не дублируются.
- **Encryption:** AES-256-GCM с ключом, derived from recovery phrase (этап 26). Каждый blob шифруется отдельно.
- **Manifest:** JSON-файл `backup.manifest` с:
  - `timestamp`, `device_id`, `profile_id`.
  - Список всех CID с их BLAKE3 хешами.
  - `oplog_range` (от какой операции до какой — для incremental).

### 18.3 Backup Policies
- **Полный бэкап:** раз в неделю (по умолчанию).
- **Инкрементальный:** каждые 6 часов.
- **Real-time (опционально):** каждая CRDT-операция реплицируется на backup target (только P2P).
- **Retention:** хранить 4 полных бэкапа (4 недели). Старые удаляются автоматически.

### 18.4 USB Backup
- **Автообнаружение:** при вставке USB Host Shim (этап 5) уведомляет Backup Engine.
- **Backup dialog:** пользователь подтверждает бэкап (или автоматический, если настроено).
- **Формат USB:** FAT32/exFAT-совместимый. Файлы шифрованные (расширение `.enc`).
- **Eject:** после завершения — safe eject через Host Shim.

### 18.5 S3 Backup
- **Multipart upload:** большие blob'ы (> 100 МБ) загружаются частями.
- **Checksum:** `x-amz-checksum` с BLAKE3.
- **Encryption:** client-side AES-256-GCM. Ключ не хранится на S3.
- **Bucket:** пользователь указывает endpoint, bucket, access key. Данные шифруются перед отправкой — провайдер не имеет доступа к содержимому.

### 18.6 P2P Backup
- **Доверенные пиры:** пользователь выбирает, какие устройства являются backup targets.
- **Background sync:** когда backup target online, Backup Engine передаёт новые blob'ы через WireGuard.
- **Storage quota:** каждый пир может ограничить объём backup (например, 50 ГБ).

### 18.7 Restore
- **Выбор точки восстановления:** список бэкапов с датой, размером, типом (USB/S3/P2P).
- **Восстановление:**
  1. Загрузка `backup.manifest`.
  2. Проверка целостности всех CID (BLAKE3).
  3. Расшифровка blob'ов.
  4. Восстановление `WORKSPACE.db`.
  5. Replay oplog (для incremental).
  6. Rebuild индексы (FTS5, tag graph).
- **Partial restore:** восстановление отдельного проекта или Space (не всего устройства).
- **Migration restore:** восстановление на новое устройство (например, после потери телефона).

### 18.8 Recovery Phrase
- При первом запуске (этап 10) генерируется recovery phrase (BIP-39, 24 слова).
- Phrase — единственный способ восстановить доступ к зашифрованным бэкапам.
- **Хранение:** Key Manager (этап 26). На этом этапе — placeholder, phrase хранится в SQLite с базовым шифрованием.
- **Проверка:** при восстановлении пользователь вводит phrase → derivation key → расшифровка backup.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Full backup USB | Полный бэкап | Запустить → файлы записаны на USB |
| Incremental S3 | Инкрементальный | Изменить файл → только новый CID загружен |
| P2P backup | На доверенное устройство | Запустить → blob'ы переданы по WG |
| Restore | Восстановление | Удалить WORKSPACE.db → восстановить → данные на месте |
| Encrypted backup | Шифрование | Попытка прочитать blob без ключа → мусор |
| Partial restore | Проект | Восстановить только один проект → остальные не тронуты |
| Recovery phrase | Генерация | First run → 24 слова показаны пользователю |

## Интеграция с будущими этапами
- **Вход:** этап 5 (Storage) — USB detection, read/write.
- **Вход:** этап 12 (VFS) — файлы, CID, blob'ы.
- **Вход:** этап 16 (CRDT) — oplog для incremental backup.
- **Вход:** этап 17 (P2P) — P2P backup target, WireGuard.
- **Выход:** restored data → этап 12 (VFS), 14 (Project Manager).
- **Вход:** этап 26 (Key Manager) — recovery phrase derivation.

## Критерии приёмки
- [ ] Full backup на USB: все blob'ы и WORKSPACE.db записаны, manifest создан.
- [ ] Incremental backup: только новые CID (< 1% данных при типичном использовании).
- [ ] S3 upload: multipart, checksum, client-side encryption.
- [ ] P2P backup: blob'ы переданы на доверенное устройство.
- [ ] Restore: после удаления WORKSPACE.db восстановление возвращает все данные.
- [ ] Encrypted blob: без recovery phrase чтение невозможно.
- [ ] Partial restore: восстановление одного проекта не затрагивает другие.
- [ ] Recovery phrase: BIP-39, 24 слова, derivation работает.

## Ссылки
- [layer-4-installation-scenarios.md](../layers/layer-4-installation-scenarios.md) — Бэкап 3-2-1, Recovery Phrase
- [layer-5-devices.md](../layers/layer-5-devices.md) — USB, дедупликация
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Backup Engine §4.8

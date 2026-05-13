# Этап 19 — CRDT Engine

## Цель
Создать движок CRDT (Conflict-free Replicated Data Types) для автоматической синхронизации данных между устройствами без конфликтов. После этого этапа Workspace умеет синхронизировать документы, настройки, и метаданные между локальными репликами.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** кастомная реализация CRDT (Causal Trees + LWW + HLC), `bun:sqlite` (хранение операций), `bun:ffi` (BLAKE3 для CID)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop).
- **Этап 14** — Micro-Kernel: VFS (файлы, версии, CID).

## Часть системы
**Level 2 — Mesh Engine: CRDT** [См. layer-8 §9, layer-5 §4, layer-3 §1.3]

## Требования

### 16.1 CRDT Model
- **Типы данных:**
  - **LWW Register:** одно значение, побеждает Last Write Wins (по HLC).
  - **Causal Tree:** упорядоченное дерево (для текста, списков). Каждый узел — операция с `parent_id`.
  - **LWW Map:** ассоциативный массив, каждый ключ — LWW Register.
  - **OR-Set:** множество (для тегов, участников чата).
- **Идентификаторы операций:** `{ hlc, peer_id, counter }` — уникальны глобально.
- **HLC (Hybrid Logical Clock):** комбинация физического времени и логического счётчика. Разрешение 1 мкс, допуск к дрейфу 10 мс.

### 16.2 Операционный журнал (OpLog)
- Каждая мутация данных записывается в `oplog` таблицу SQLite:
  - `op_id` (HLC + peer_id).
  - `type` (insert, update, delete, move).
  - `target` (table + row_id).
  - `payload` (JSON, delta).
  - `parent_ops` (JSON-массив parent op_ids, для Causal Trees).
- **Append-only:** журнал никогда не изменяется. Отмена — новая операция `undo`.
- **Compaction:** старые операции (> 30 дней) архивируются в cold storage (blob CID), если все пиры подтвердили получение.

### 16.3 Синхронизация (локальная)
- **XOR-sync (упрощённый):** два устройства обмениваются хешами своих oplog'ов. Если хеши отличаются — обмен недостающими операциями.
- **Bloom filter:** перед полной синхронизацией устройства обмениваются Bloom filter'ами своих op_ids. Это снижает трафик на 90% для синхронизированных устройств.
- **Delta encoding:** для текстовых Causal Trees передаются только новые узлы, а не весь документ.
- **Conflict resolution:** автоматическая, без user intervention (по свойству CRDT).

### 16.4 VFS Integration
- Все операции VFS (create, update, delete) автоматически генерируют CRDT-операции.
- **File Passport:** LWW Map (метаданные).
- **File Body:** если текстовый — Causal Tree (collaborative editing). Если бинарный — LWW Register (целиком заменяется).
- **Tags:** OR-Set.
- **Versions:** Causal Tree истории.

### 16.5 Offline-режим
- Все операции записываются локально в oplog.
- Синхронизация происходит при появлении связи с другим устройством.
- **Offline indicators:** в UI (этап 14, Project Manager) отображается статус синхронизации (synced / syncing / offline).

### 16.6 Merkle Search Trees (подготовка)
- Для масштабной синхронизации (> 1000 документов) используются Merkle Search Trees (MST).
- На этом этапе — базовая структура MST для oplog. Полная интеграция с P2P — в этапе 17.
- **MST node:** `{ key: op_id, hash: BLAKE3(children + op), left, right }`.
- **Root hash:** если root hash двух устройств совпадает — они синхронизированы.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Insert text | Causal Tree | Вставить "hello" → op записана, дерево корректно |
| Concurrent edit | CRDT merge | Два устройства редактируют → merge без потерь |
| HLC ordering | Время | Операции упорядочены корректно |
| XOR sync | Синхронизация | Два устройства → обмен операциями → данные идентичны |
| Bloom filter | Оптимизация | 1000 общих операций, 1 новая → передаётся только 1 |
| Offline write | Локальная запись | Отключить сеть → запись работает, oplog растёт |
| VFS CRDT | Автоматическая генерация | Создать файл в VFS → op появляется в oplog |

## Интеграция с будущими этапами
- **Вход:** этап 12 (VFS) — все мутации файлов.
- **Выход:** `oplog` → этап 17 (P2P Mesh) для передачи другим устройствам.
- **Выход:** MST root hash → этап 17 для быстрой проверки синхронизации.
- **Вход:** этап 17 — входящие операции от других устройств.

## Критерии приёмки
- [ ] Causal Tree: вставка текста, удаление, перемещение работают.
- [ ] Concurrent edit: два устройства редактируют один документ → merge без потерь (проверка через property-based testing).
- [ ] HLC: монотонное возрастание, корректное разрешение конфликтов при clock skew.
- [ ] XOR sync: синхронизация 1000 операций < 1 сек (локально).
- [ ] Bloom filter: при 99.9% совпадении передаётся < 1% данных.
- [ ] Offline: запись при отсутствии сети работает, oplog сохраняется.
- [ ] VFS integration: каждая операция VFS генерирует CRDT-op.

## Ссылки
- [layer-5-devices.md](../layers/layer-5-devices.md) — P2P синхронизация, CRDT
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Offline-режим, оптимистичные мутации
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — CRDT §9, Sync Engine

# Этап 25 — Messenger Engine

## Цель
Создать децентрализованный мессенджер на базе P2P Mesh. После этого этапа пользователь может отправлять текстовые сообщения, создавать группы, и общаться без центрального сервера.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `bun:sqlite` (история сообщений), `libp2p` (транспорт через этап 17), `@noble/ed25519` (подписи сообщений)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop).
- **Этап 19** — CRDT Engine (операционный журнал для сообщений).
- **Этап 20** — P2P Mesh (транспорт, обнаружение пиров).
- **Этап 22** — App Registry (WORKSPACE.messenger как system app).

## Часть системы
**Level 1 — Бэк: Chat Engine** [См. layer-8 §6, layer-1 §5, layer-3 §1.3]

## Требования

### 22.1 Message Model
- Сообщение — CRDT-операция (Causal Tree для текста сообщения):
  - `message_id` (HLC + peer_id).
  - `conversation_id` (группа или peer_id собеседника).
  - `author_id` (peer_id отправителя).
  - `content` (текст, Unicode, max 4096 символов).
  - `timestamp` (HLC).
  - `signature` (Ed25519 подпись content + timestamp, для аутентификации).
- **Статусы:** `sending` → `sent` (доставлено в P2P) → `delivered` (получатель online и принял) → `read`.
- **Редактирование:** `edit(message_id, new_content)` — новая CRDT-операция, старая помечается `edited`. История изменений сохраняется.
- **Удаление:** `delete(message_id)` — soft delete (флаг `deleted`), сообщение исчезает из UI но остаётся в oplog.

### 22.2 Conversations
- **Direct:** 1-on-1 чат между двумя peer_id.
- **Group:** множество участников (до 256). `conversation_id` = BLAKE3(sorted peer_ids + creation_timestamp).
- **Channel:** публичный или приватный канал (аналог Discord/Telegram). Участники присоединяются по invite link.
- **Metadata:** название, аватар, список участников, права (кто может писать, добавлять, удалять).

### 22.3 P2P Messaging
- **Transport:** сообщения передаются через P2P Mesh (этап 17) — WireGuard + libp2p.
- **Direct delivery:** если получатель online — сообщение отправляется напрямую.
- **Store-and-forward:** если получатель offline — сообщение хранится на доверенных пирах (supernode или mutual contacts) до 7 дней.
- **Gossip:** групповые сообщения рассылаются через GossipSub (этап 17) для эффективности.

### 22.4 Message History
- SQLite таблица `messages` с индексами по `conversation_id` + `timestamp`.
- **Полнотекстовый поиск:** FTS5 по содержимому сообщений.
- **Lazy load:** история загружается порциями (pagination) по 50 сообщений.
- **CRDT sync:** при подключении нового устройства все сообщения синхронизируются через CRDT Engine (этап 16).

### 22.5 Notifications
- **Local:** push-уведомление внутри Workspace (этап 15, Overlay Layer) при новом сообщении.
- **Priority:** сообщения от контактов с высоким приоритетом пробуждают устройство (WOL через P2P, опционально).
- **Mute:** возможность отключить уведомления для conversation или на время (1 час, 8 часов, до утра).

### 22.6 UI (WORKSPACE.messenger)
- **Список чатов:** слева, с preview последнего сообщения, timestamp, unread count.
- **История сообщений:** справа, с датами, аватарами, статусами.
- **Ввод:** текстовое поле с поддержкой markdown (bold, italic, code).
- **Поиск:** поиск по истории (FTS5).
- **Рендеринг:** через `@workspace/ui` (этап 20) → Display Server (этап 9).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Send message | Отправка | Написать "hello" → сообщение появляется, статус sent |
| Receive message | Получение | С другого устройства → уведомление + в истории |
| Group chat | Группа | Создать группу из 3 участников → все получают |
| Edit message | Редактирование | Изменить → "edited" метка, история сохранена |
| Delete message | Удаление | Удалить → исчезает из UI, в oplog осталось |
| Search history | Поиск | Поиск "hello" → находит сообщение |
| Offline delivery | Store-and-forward | Отправить offline → получатель подключился, получил |

## Интеграция с будущими этапами
- **Вход:** этап 16 (CRDT) — операционный журнал, Causal Trees.
- **Вход:** этап 17 (P2P) — transport, GossipSub, store-and-forward.
- **Вход:** этап 19 (App Registry) — WORKSPACE.messenger как system app.
- **Выход:** сообщения → этап 24 (Voice, Zero UI: "прочитай сообщение").
- **Выход:** уведомления → этап 15 (Window Manager, Overlay Layer).

## Критерии приёмки
- [ ] Отправка и получение сообщения между двумя устройствами < 2 сек (LAN).
- [ ] Групповой чат: сообщение доставлено всем 3 участникам.
- [ ] Offline delivery: сообщение отправлено offline → получено при подключении < 5 сек.
- [ ] Edit/delete: корректное отображение в UI, история в oplog.
- [ ] FTS5 поиск находит сообщение по содержимому.
- [ ] CRDT sync: два устройства синхронизируют историю при подключении.
- [ ] 1000 сообщений в чате — загрузка истории < 100 мс (pagination).

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Messenger, уведомления
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Multi-Back, offline
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Chat Engine §6

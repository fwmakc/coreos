# Этап 20 — P2P Mesh

## Цель
Создать одноранговую сеть (P2P Mesh) для обнаружения устройств, установления соединений, и синхронизации данных через WireGuard. После этого этапа Workspace умеет находить другие устройства пользователя, устанавливать с ними зашифрованные соединения, и синхронизировать данные.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `libp2p` (js-libp2p), `@libp2p/tcp`, `@libp2p/mdns`, `@libp2p/kad-dht`, `boringtun` (через `bun:ffi` или WASM), `bun:udp` (для STUN/hole punching)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 8** — Host Shim: Network (TCP/UDP, WireGuard туннель, P2P Transport API).
- **Этап 12** — Micro-Kernel: WORKSPACE (event loop, SQLite).
- **Этап 19** — CRDT Engine (операции для синхронизации).

## Часть системы
**Level 2 — Mesh Engine: P2P** [См. layer-8 §9, layer-5 §4–5, layer-3 §1.3]

## Требования

### 17.1 Peer Identity
- Каждое устройство имеет уникальный `peer_id` (Ed25519 публичный ключ).
- `peer_id` генерируется при первом запуске и хранится в Key Manager (этап 26).
- **Peer Discovery:** устройства обнаруживают друг друга по `peer_id`.

### 17.2 Local Discovery (LAN)
- **mDNS:** анонсирование `_Workspace._tcp.local` с `peer_id`, портом, и поддерживаемыми capabilities.
- **Bluetooth LE:** fallback для устройств без WiFi (опционально, mobile-first).
- **Direct IP:** ввод IP-адреса вручную (для отладки).
- **QR-код:** сканирование QR для добавления нового устройства (mobile).

### 17.3 Connection Establishment
- **WireGuard handshake:** при обнаружении пира устройства обмениваются публичными ключами WireGuard через сигнальный канал (mDNS или relay).
- **STUN:** определение внешнего IP и порта для NAT traversal.
- **Hole punching:** UDP hole punching для прямого соединения через NAT.
- **TURN relay:** fallback если прямое соединение невозможно. Relay-серверы WORKSPACE (публичные bootstrap nodes) пересылают трафик. Relay — последняя инстанция, не основной путь.
- **Connection state:** `disconnected` → `discovered` → `handshaking` → `connected` → `syncing` → `synced`.

### 17.4 libp2p Integration
- **Transport:** TCP через WireGuard туннель (не напрямую — всё шифруется WireGuard).
- **Protocol:** custom protocol `/Workspace/sync/1.0.0` для синхронизации CRDT.
- **DHT (Kademlia):** для поиска пиров в глобальной сети (bootstrap nodes).
- **PubSub (GossipSub):** для анонсирования изменений ("у меня новые ops для topic X").
- **NAT:** UPnP / NAT-PMP для открытия порта на роутере (опционально).

### 17.5 Sync Protocol
- **Анонс:** устройство публикует в PubSub: `{ peer_id, mst_root_hash, topic }`.
- **Запрос:** если другой пир видит другой `mst_root_hash` — запрашивает недостающие операции.
- **Batch sync:** накопление изменений 100 мс перед отправкой (уменьшает chatter).
- **Priority sync:** активные документы (открытые в текущем проекте) синхронизируются в первую очередь.
- **Delta sync:** передаются только CRDT-операции, не целые файлы. Для бинарных файлов — передаётся CID, пир запрашивает blob если у него его нет (lazy load).

### 17.6 Multi-Back (корпоративный сценарий)
- **Supernode:** в корпоративной сети есть выделенный Supernode (мощное устройство или сервер), который всегда online.
- **Hierarchical sync:** leaf-устройства синхронизируются с Supernode, а не напрямую друг с другом. Supernode отвечает за anti-entropy между всеми leaf'ами.
- **Firewall:** Supernode фильтрует sync-операции по corporate policy (блокировка личных Spaces).

### 17.7 Session Handoff
- **Сценарий:** пользователь работает на ноутбуке, закрывает крышку, открывает телефон — проекты и приложения должны быть доступны.
- **Checkpoint sync:** при закрытии крышки ноутбук отправляет checkpoint всех открытых проектов.
- **Lazy load:** телефон получает Passport'ы файлов, но не сами blob'ы. Blob'ы загружаются по требованию.
- **App state sync:** открытые приложения помечаются как `handed_off`. На новом устройстве они отображаются как "замороженные" до активации.

### 17.8 Seeding (Verified Seeding)
- **Сценарий:** пользователь хочет поделиться файлом с другом без центрального сервера.
- **Seeding:** файл анонсируется в P2P сети с CID. Другие пиры могут запросить blob.
- **Verification:** получатель проверяет BLAKE3 CID после загрузки.
- **Expiration:** seed автоматически истекает через 7 дней или при удалении файла.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| mDNS discovery | Локальное обнаружение | Два устройства в LAN → видят друг друга |
| WireGuard connect | Зашифрованное соединение | Установить связь → ping через WG проходит |
| NAT traversal | Hole punching | Два устройства за NAT → прямое соединение |
| CRDT sync | Синхронизация | Создать файл на A → появляется на B |
| Lazy load | Ghost file | Открыть ghost file на B → blob загружается с A |
| Session handoff | Переход устройства | Закрыть ноутбук → открыть телефон → проекты доступны |
| Verified seed | Публикация | Засидировать файл → друг скачивает, CID совпадает |

## Интеграция с будущими этапами
- **Вход:** этап 6 (Network) — TCP/UDP, WireGuard, STUN.
- **Вход:** этап 16 (CRDT) — операции для передачи.
- **Выход:** `PeerConnection` → этап 18 (Backup Engine) для P2P backup.
- **Выход:** `SyncStream` → этап 12 (VFS) для обновления ghost files.
- **Выход:** `SessionHandoff` → этап 14 (Project Manager) для восстановления проектов.
- **Вход:** этап 22 (Messenger) — P2P transport для сообщений.
- **Вход:** этап 23 (VoIP) — WireGuard для голосовых звонков.

## Критерии приёмки
- [ ] mDNS: два устройства в одной LAN обнаруживают друг друга < 3 сек.
- [ ] WireGuard: соединение установлено, ping < 10 мс (локально).
- [ ] NAT traversal: два устройства за NAT (разные сети) устанавливают прямое соединение через STUN.
- [ ] CRDT sync: создание файла на A → появление на B < 5 сек (LAN).
- [ ] Lazy load: ghost file открывается, blob загружается < 10 сек (LAN).
- [ ] Session handoff: checkpoint с ноутбука доступен на телефоне < 30 сек.
- [ ] Verified seed: seed → download → BLAKE3 совпадает.
- [ ] 10 устройств в Mesh — стабильность, нет коллапса.

## Ссылки
- [layer-5-devices.md](../layers/layer-5-devices.md) — P2P Mesh, WireGuard, devices
- [layer-3-system-split.md](../layers/layer-3-system-split.md) — Multi-Back, Session Handoff
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — P2P Mesh §9, Sync Engine

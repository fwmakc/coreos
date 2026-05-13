# Этап 8 — Host Shim: Network

## Цель
Добавить в Host Shim сетевую абстракцию: TCP/UDP сокеты, WireGuard туннель, и API для P2P Mesh. После этого этапа Workspace умеет устанавливать сетевые соединения и создавать зашифрованные P2P-туннели.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `tokio` (async runtime), `rustls` (TLS), `boringtun` или `wireguard-rs` (WireGuard), `igd` (UPnP port mapping, опционально)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 1–3** — Host Shim: платформенные основы.
- **Этап 7** — Host Shim: Storage (reference архитектура).

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1.4, layer-5 §4, layer-5 §5]

## Требования

### 6.1 Сетевая абстракция
- Определение trait `NetworkBackend`:
  - `tcp_connect(addr) -> TcpStream`
  - `tcp_bind(addr) -> TcpListener`
  - `udp_bind(addr) -> UdpSocket`
  - `get_local_addrs() -> Vec<IpAddr>`
  - `get_default_interface() -> NetworkInterface`
- Все операции неблокирующие (async через `tokio`).

### 6.2 DNS
- Кастомный DNS-резолвер, не зависящий от системного (для анонимного режима).
- Fallback на системный DNS если кастомный не настроен.
- Поддержка DNS-over-HTTPS (DoH) и DNS-over-TLS (DoT) [См. layer-7 §21.12].

### 6.3 WireGuard туннель
- Интеграция WireGuard через `boringtun` (userspace реализация).
- Управление туннелем:
  - `create_tunnel(private_key, peers) -> TunnelId`
  - `close_tunnel(tunnel_id)`
  - `get_tunnel_stats(tunnel_id) -> TrafficStats`
- **P2P Mesh:** каждое устройство в Mesh имеет свой WireGuard ключ. При установлении соединения устройства обмениваются публичными ключами через сигнальный канал (mDNS или bootstrap relay).
- **NAT Traversal:** STUN для определения внешнего IP, fallback на TURN relay если прямое соединение невозможно.

### 6.4 P2P Transport API
- `P2PTransport` — абстракция над WireGuard + UDP hole punching:
  - `listen() -> P2PListener` — принимать входящие соединения.
  - `dial(peer_id) -> P2PStream` — установить соединение с peer.
  - `announce(topic) -> AnnounceHandle` — анонсировать себя в топике.
  - `discover(topic) -> Stream<PeerInfo>` — обнаруживать пиров в топике.
- Этот API используется этапом 17 (P2P Mesh) для построения overlay network.

### 6.5 Безопасность сети
- **Firewall:** по умолчанию все исходящие соединения разрешены, входящие — только через WireGuard.
- **Certificate pinning:** для Cloud Bridge (этап 25) — публичные ключи серверов зашиты в бинарник.
- **No IPv6 leak:** если WireGuard активен, IPv6 трафик блокируется или туннелируется [См. layer-7 §21.12].

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| TCP echo | TCP-соединение | Подключиться к localhost:8080 → echo работает |
| UDP echo | UDP-пакеты | Отправить UDP → получить ответ |
| WireGuard ping | Туннель | Создать туннель между двумя инстансами → ping проходит |
| NAT traversal | Hole punching | Два устройства за NAT → устанавливают прямое соединение |
| DNS DoH | DNS-over-HTTPS | Резолв через DoH → корректный IP |
| Firewall | Блокировка | Попытка входящего TCP без WireGuard → отказ |

## Интеграция с будущими этапами
- **Выход:** `NetworkBackend` → этап 17 (P2P Mesh) для overlay network.
- **Выход:** `P2PTransport` → этап 17 (libp2p integration).
- **Выход:** WireGuard → этап 23 (VoIP) для encrypted voice.
- **Вход:** этап 17 → запросы на установление P2P-соединений.

## Критерии приёмки
- [ ] Компилируется на Windows, macOS, Linux, Android.
- [ ] TCP и UDP работают на всех платформах.
- [ ] WireGuard туннель между двумя инстансами на одной машине работает (ping через tun).
- [ ] NAT traversal: два устройства в одной LAN устанавливают прямое UDP-соединение (STUN не нужен).
- [ ] DNS-over-HTTPS резолвит корректно.
- [ ] Firewall блокирует неавторизованные входящие соединения.

## Ссылки
- [layer-5-devices.md](../layers/layer-5-devices.md) — P2P Mesh, WireGuard
- [layer-7-security.md](../layers/layer-7-security.md) — Сетевая безопасность, DNS
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Network §4.1.4

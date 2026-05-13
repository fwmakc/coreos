# Этап 27 — VoIP Engine

## Цель
Создать подсистему голосовых и видео звонков между пользователями WORKSPACE через WebRTC поверх WireGuard. После этого этапа пользователь может звонить другим пользователям WORKSPACE с шифрованием end-to-end.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `werift-webrtc` или `node-datachannel`, `bun:udp` (SRTP), `opus-decoder` / `opus-encoder`
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Этап 8** — Host Shim: Network (TCP/UDP, WireGuard).
- **Этап 20** — P2P Mesh (WireGuard туннели, peer discovery, signaling).
- **Этап 25** — Messenger Engine (contacts, signaling transport).
- **Этап 6** — Host Shim: Audio (Opus capture/playback).

## Часть системы
**Level 1 — Бэк: VoIP** [См. layer-8 §6.3, layer-1 §5, layer-5 §5]

## Требования

### 27.1 Signaling
- Сигналинг через P2P Mesh (этап 20). При звонке инициатор отправляет `call_offer` (SDP) через Messenger (этап 25).
- Сообщения: `offer`, `answer`, `ice-candidate`, `reject`, `hangup`.
- Fallback на signaling relay если оба устройства за symmetric NAT.

### 27.2 Media Transport
- WebRTC data channels и media tracks поверх WireGuard (не через публичные STUN/TURN).
- ICE: host candidates + reflexive через STUN (опционально). No TURN (private P2P only).
- SRTP: шифрование media payload.
- DTLS: handshake для SRTP key derivation.

### 27.3 Audio
- Кодек: Opus, 48 kHz, stereo (downmix to mono при слабом канале).
- AEC (Acoustic Echo Cancellation): включён по умолчанию (WebRTC AEC3 или системный).
- NS (Noise Suppression): WebRTC RNNoise или системный.
- AGC (Automatic Gain Control): системный или software.
- Jitter buffer: 200 мс адаптивный.
- PLC (Packet Loss Concealment): при потере < 10% — незаметно.

### 27.4 Video (placeholder)
- На этом этапе: только аудио-звонки. Видео — post-release.
- Архитектура ready для видео: VP8/VP9 tracks, H.264 (hardware encoder на mobile).

### 27.5 Call State Machine
- `idle` → `dialing` → `ringing` → `connected` → `ended`.
- `missed`, `declined`, `busy`.
- Hold/Resume: при hold — mute audio, при resume — unmute.
- Conference: merge 2 calls (3-way, опционально).

### 27.6 UI (WORKSPACE.voip)
- Incoming call overlay: поверх всех окон, через Display Server Overlay Layer.
- In-call UI: mute, speaker (hands-free), end call, duration, quality indicator (bars).
- Call history: список с duration, outcome, recording (если enabled).
- Contact card: при звонке — имя, аватар, статус.

### 27.7 Quality
- MOS estimation: из RTT, jitter, packet loss.
- Adaptive bitrate: при packet loss > 5% — снижение bitrate.
- Bandwidth estimation: GCC (Google Congestion Control) или simple loss-based.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| VoIP call | Звонок | Позвонить → соединение < 3 сек, голос слышен |
| Call end | Завершение | Положить трубку → state = ended, history обновлён |
| Mute | Без звука | Нажать mute → собеседник не слышит |
| Speaker | Громкая связь | Переключить → audio route меняется |
| Quality | MOS | При 0% loss → MOS > 3.5 |
| Incoming | Входящий | Входящий звонок → overlay появляется |

## Интеграция с будущими этапами
- **Вход:** этап 6 (Audio) — capture/playback.
- **Вход:** этап 8 (Network) — UDP, WireGuard.
- **Вход:** этап 20 (P2P) — signaling transport.
- **Вход:** этап 25 (Messenger) — contacts, message UI patterns.
- **Выход:** incoming call overlay → этап 18 (Window Manager, Overlay Layer).

## Критерии приёмки
- [ ] VoIP: соединение < 3 сек (LAN), latency < 200 мс.
- [ ] MOS > 3.5 при 0% packet loss.
- [ ] Mute/speaker работают.
- [ ] Incoming call overlay отображается.
- [ ] Call history: все звонки с duration.
- [ ] Adaptive bitrate при packet loss > 5%.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Messenger, контакты
- [layer-5-devices.md](../layers/layer-5-devices.md) — VoIP, медиа
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — VoIP §6.3

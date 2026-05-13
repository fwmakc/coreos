# Этап 34 — Performance Optimizations

## Цель
Провести комплексную оптимизацию системы: рендеринг, память, P2P, CPU. После этого этапа Workspace работает с заданным performance budget на рекомендуемом железе.

## Язык и стек
- **Язык:** Rust (рендеринг, память), TypeScript (runtime, P2P)
- **Runtime:** Bun, native Rust
- **Ключевые зависимости:** `wgpu` (GPU profiling), `tracy` (CPU profiler, опционально), `bun:ffi` (для Rust callbacks)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Все предыдущие этапы** (1–33).

## Часть системы
**Level 0–4 — Cross-cutting: Performance** [См. layer-8 §17, layer-9 §2, layer-11 §Performance Targets]

## Требования

### 34.1 Performance Budget
- **Frame Time:** < 16.67 мс (60 FPS) при 10 окон на рекомендуемом железе.
- **Input Latency:** < 8 мс (от нажатия клавиши до пикселя на экране).
- **Cold Boot:** < 5 сек (этапы 1–12 загружены, SQLite инициализирована).
- **Warm Boot:** < 2 сек (restore from checkpoint).
- **Memory (Base):** < 2 GB для base конфигурации (Host Shim + Display Server + Micro-Kernel + 3 окна).
- **Memory (Per App):** < 128 MB на V8 Isolate.
- **P2P Sync:** < 1 сек для 1000 документов (LAN).
- **ASR Latency:** < 500 мс (Whisper base).
- **TTS Latency:** < 100 мс (Piper).

### 34.2 Rendering Optimizations
- **Occlusion Culling:** не рендерить скрытые окна (полностью перекрытые другими).
- **Damage Tracking:** перерисовывать только изменённые области (scissor rects).
- **Instanced Rendering:** 1000 кнопок — один draw call (instance buffer).
- **Texture Atlas:** объединение маленьких текстур (иконки, глифы) в одну large texture.
- **Mipmaps:** для уменьшения bandwidth при отдалённых/масштабированных текстурах.
- **GPU Memory Budget:** 1 GB лимит, LRU eviction для неиспользуемых текстур.
- **Early-Z / Depth Pre-pass:** для сложных сцен (опционально).

### 34.3 Memory Optimizations
- **Arena Allocators:** для краткоживущих объектов (render frames, audio buffers).
- **Object Pooling:** переиспользование часто создаваемых объектов (GPU buffers, command encoders).
- **Texture Compression:** BC7 (Windows/DirectX), ASTC (Android/iOS), ETC2 (fallback).
- **VFS Cache:** LRU для blob'ов, 256 MB по умолчанию.
- **SQLite Optimization:** WAL mode, `PRAGMA mmap_size`, `PRAGMA cache_size`, prepared statements.

### 34.4 P2P Optimizations
- **Bloom Filter:** для пропуска уже синхронизированных операций.
- **Delta Encoding:** для текстовых CRDT (только изменённые узлы).
- **Batch Sync:** накопление изменений 100 мс перед отправкой.
- **Priority Sync:** активные документы первыми.
- **Kademlia DHT:** для быстрого поиска пиров.
- **Connection Pooling:** max 50 соединений, circuit breaker для нестабильных узлов.

### 34.5 Profiling Infrastructure
- **On-screen Profiler:** F12 для разработчиков — overlay с FPS, frame time, GPU memory, CPU load.
- **Chrome Trace Export:** `traceEvents` формат для анализа в Chrome DevTools Performance.
- **Real-time Metrics:** CPU%, RAM%, GPU%, battery, network throughput — обновление 1 Гц.
- **Alerts:** если frame time > 20 мс на 3+ кадра подряд — лог warning + notification (dev mode).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| 60 FPS 10 wins | Рендеринг | 10 окон → > 58 FPS |
| Input latency | Задержка | Key press → pixel < 8 мс |
| Memory base | Память | 3 окна + ядро → < 2 GB |
| P2P sync 1k docs | Синхронизация | 1000 документов → < 1 сек |
| Profiler overlay | Отладка | F12 → FPS, GPU mem видны |
| Chrome trace | Экспорт | Записать 5 сек → JSON открывается в Chrome |

## Интеграция с будущими этапами
- **Вход:** все предыдущие этапы.
- **Выход:** optimizations → все предыдущие этапы.

## Критерии приёмки
- [ ] 10 окон → > 58 FPS (frame time < 17.2 мс).
- [ ] Input latency < 8 мс.
- [ ] Cold boot < 5 сек.
- [ ] Warm boot < 2 сек.
- [ ] Base memory < 2 GB.
- [ ] P2P sync 1000 docs < 1 сек (LAN).
- [ ] Profiler overlay работает (F12).
- [ ] Chrome trace экспортируется и открывается.

## Ссылки
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Performance §17
- [layer-9-hardware-requirements.md](../layers/layer-9-hardware-requirements.md) — Performance targets

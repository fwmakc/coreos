# Этап 35 — Stress Tests

## Цель
Провести стресс-тестирование всех подсистем: рендеринг, синхронизация, память, сеть, батарея, voice. После этого этапа подтверждено, что Workspace устойчива под экстремальной нагрузкой.

## Язык и стек
- **Язык:** TypeScript (тестовые сценарии), Rust (низкоуровневые нагрузочные тесты)
- **Runtime:** Bun, Rust test harness
- **Ключевые зависимости:** кастомный stress test framework, `wgpu` (GPU stress), `bun:tcp` (network flood)
- **Целевые ОС:** Windows, macOS, Linux, Android, iOS

## Зависимости
- **Все предыдущие этапы** (1–34).

## Часть системы
**Level 0–4 — Cross-cutting: QA** [См. layer-8 §17.3, project/stress-tests.md]

## Требования

### 35.1 Test Scenarios
- **Window Stress:** открытие 1000 окон (пустых и с контентом). Цель: > 30 FPS, no crash, memory < 4 GB.
- **CRDT Sync Stress:** 1000 документов, 10 устройств, concurrent editing. Цель: sync < 1 сек, no conflicts, no data loss.
- **Memory Stress:** 95% RAM utilization. Цель: graceful degradation (checkpoint, kill background apps, no OOM crash).
- **Network Stress:** 500 мс latency, 10% packet loss, 1% packet reorder. Цель: P2P sync stability, VoIP MOS > 2.5.
- **Battery Critical:** 1% battery. Цель: graceful shutdown, all data saved, zero corruption.
- **CPU Stress:** 100% CPU load (background compilation). Цель: UI responsive (> 15 FPS), voice commands queued and executed.
- **GPU Stress:** GPU memory exhaustion. Цель: graceful fallback (lower textures, disable effects), no crash.
- **Audio Stress:** continuous voice recognition for 1 hour. Цель: no memory leak, accuracy maintained.

### 35.2 Test Infrastructure
- **Automated Harness:** каждый сценарий — скрипт, который запускает систему, генерирует нагрузку, измеряет метрики, сравнивает с budget.
- **Metrics Collection:** frame time, memory, CPU, GPU, network throughput, battery, latency — каждые 100 мс.
- **Regression Detection:** сравнение с baseline (предыдущий запуск). Деградация > 10% → fail.
- **Report Generation:** HTML-отчёт с графиками, таблицами, pass/fail статусом.

### 35.3 Acceptance Criteria
- Все сценарии проходят без crash.
- Деградация < 10% от baseline.
- No memory leaks (RSS возвращается к baseline после нагрузки).
- No data loss (CRDT consistency check).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| 1000 windows | Окна | 1000 окон → > 30 FPS, < 4 GB |
| 1000 docs CRDT | Синхронизация | 10 устройств → sync < 1 сек |
| RAM 95% | Память | Graceful degradation, no OOM |
| 500 ms latency | Сеть | P2P stable, VoIP MOS > 2.5 |
| Battery 1% | Батарея | Graceful shutdown, no corruption |
| 1h voice | Голос | No memory leak, accuracy > 85% |

## Интеграция с будущими этапами
- **Вход:** все предыдущие этапы.
- **Выход:** отчёты → этап 37 (Documentation).

## Критерии приёмки
- [ ] 1000 окон: > 30 FPS, < 4 GB, no crash.
- [ ] CRDT 1000 docs/10 devices: sync < 1 сек, no conflicts.
- [ ] RAM 95%: graceful degradation, no OOM.
- [ ] Network 500 мс + 10% loss: P2P stable.
- [ ] Battery 1%: graceful shutdown, data intact.
- [ ] 1h voice: no memory leak.
- [ ] HTML report generated with graphs.

## Ссылки
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Stress Tests §17.3
- [project/stress-tests.md](../project/stress-tests.md) — Результаты стресс-тестов

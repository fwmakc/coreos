# Layer 9 — Hardware Requirements | Требования к железу

На каком железе работает Workspace. Минимальные, рекомендуемые и максимальные конфигурации. Оптимизации под слабое железо и специфические устройства.

---

## Архитектуры и платформы

| Платформа | Статус | Примечания |
|-----------|--------|------------|
| x86_64 | ✅ Первичная | ПК, ноутбуки, серверы |
| ARM64 (aarch64) | ✅ Первичная | Apple Silicon, современные планшеты, Raspberry Pi 4+ |
| ARMv7 | ⚠️ Lazy Boot only | Старые планшеты, embedded. Только Headless Logic + Remote Isolate Call |
| RISC-V | 🔮 В планах | Потенциальная поддержка после стабилизации Host Shim |

**Хост-ОС:** Windows 10+, macOS 12+, Linux (kernel 5.15+), Android 10+.

---

## Уровни конфигураций

### 1. Минимальная (Lazy Boot / Headless Logic)

Для устройств, которые физически не могут запустить полный WORKSPACE, но работают как "умный терминал".

| Компонент | Минимум |
|-----------|---------|
| CPU | 1 ядро ARMv7 / x86, 400 MHz |
| RAM | 256 MB |
| GPU | Любая, способная вывести framebuffer. WebGPU не требуется |
| Хранилище | 128 MB (только Host Shim + Display Server) |
| Сеть | Wi-Fi 802.11n или Ethernet |

**Что работает:**
- Display Server (рендеринг через Host Shim, возможно программный)
- Input Handler (тач, мышь, клавиатура)
- **Intent Parser (базовый)** — rule-based разбор текста в Command Bar. Локальный, лёгкий, без нейронок. Понимает "открой калькулятор", "переключи на проект Ремонт"
- P2P Mesh (WireGuard, минимальный footprint)
- Remote Isolate Call — вся логика выполняется на удалённом узле (WORKSPACE Base, ПК, сервер)

**Что НЕ работает:**
- Локальный Micro-Kernel (Bun + V8)
- Локальные приложения (V8 Isolates)
- Локальная индексация (Search Engine)
- Локальный App Registry
- Voice Engine (Whisper)
- Генеративный UI
- SLM / облачные модели (Cloud Bridge)

**Примеры устройств:** старый Kindle Fire, Raspberry Pi Zero 2W, китайские планшеты за $50.

---

### 2. Рекомендуемая (Полноценный WORKSPACE)

Для комфортной работы: локальные приложения, синхронизация, индексация, отрисовка через WebGPU.

| Компонент | Рекомендация |
|-----------|--------------|
| CPU | 2+ ядра x86_64 / ARM64, 1.5 GHz |
| RAM | 4 GB |
| GPU | Поддержка Vulkan 1.1 / DirectX 12 / Metal / OpenGL ES 3.1 (для wgpu fallbacks) |
| Хранилище | 16 GB (система + кэш приложений + локальные данные) |
| Сеть | Wi-Fi 5 (802.11ac) или Ethernet 1 Gbps |

**Что работает:**
- Все уровни архитектуры (Host Shim, Micro-Kernel, Mesh Engine, Display Server, Intent API)
- Локальные V8 Isolates (приложения)
- WebGPU-рендеринг (60 FPS, эффекты)
- **Intent Parser (полный)** — Semantic Kernel, векторный поиск по файлам и контактам
- P2P Mesh (полноценный узел: sync, seeding, VoIP)
- Search Engine (полнотекстовая индексация)
- Voice Engine (локальный Whisper)
- Генеративный UI (простые виджеты на лету)
- SLM (Small Language Model) — локальные лёгкие модели через Ollama

**Примеры устройств:** средний ноутбук 2020+, iPad Air 4+, Raspberry Pi 4/5, современный Android-планшет.

---

### 3. Максимальная (WORKSPACE Base / Разработка / Enterprise)

Для серверных узлов, рабочих станций разработчиков, корпоративных Бэк-офисов.

| Компонент | Максимум |
|-----------|----------|
| CPU | 4+ ядра x86_64 / ARM64, 3.0 GHz |
| RAM | 16 GB+ |
| GPU | Дискретная или мощная встроенная (для WebGPU-разработки и рендеринга) |
| Хранилище | 256 GB+ SSD (VFS + кэш + App Registry + Audit logs) |
| Сеть | Ethernet 1 Gbps, статический IP (для WORKSPACE Base) |

**Что работает:**
- WORKSPACE Base (всегда включённый узел, seeding для других устройств)
- Множественные V8 Isolates одновременно
- **Полный AI-стек:** Intent API, Semantic Kernel, Voice Engine, Generative UI, SLM/LLM локально, Cloud Bridge
- Hypervisor-режим (Type-1 для промышленных сценариев)
- Полный аудит и логирование

**Примеры устройств:** домашний NAS, NUC, Mac Mini, корпоративный сервер, industrial PC.

---

## Требования к GPU

Workspace использует **WebGPU** (через wgpu-native) для рендеринга. Это критичный компонент.

| API | Минимум | Оптимально |
|-----|---------|------------|
| Vulkan | 1.1 | 1.3 |
| DirectX | 12 | 12 (Feature Level 12_0) |
| Metal | 2 | 3 |
| OpenGL ES | 3.1 (fallback) | — |

**Если GPU не поддерживает WebGPU:**
- Host Shim переключается на программный рендеринг (CPU) или OpenGL fallback
- Производительность падает до 15–30 FPS
- Эффекты (blur, shadows) отключаются
- Рекомендуется обновить драйверы или использовать устройство с поддержкой Vulkan/Metal

---

## Требования к сети

| Сценарий | Минимум | Комфорт |
|----------|---------|---------|
| Локальная sync (LAN) | 10 Mbps | 100 Mbps+ |
| Удалённая sync (интернет) | 1 Mbps uplink | 10 Mbps+ |
| VoIP / звонки | 100 Kbps | 1 Mbps |
| Стриминг медиа (P2P) | 5 Mbps | 25 Mbps |
| Seeding (перенос primary Бэка) | 10 Mbps | 100 Mbps+ |

**NAT traversal:** работает через Tailscale-подобный механизм. Требуется исходящее UDP-соединение (любой порт). Если UDP заблокирован — используется TCP relay (замедление ~20%).

---

## Энергопотребление и термалы

| Режим | Потребление | Температура |
|-------|-------------|-------------|
| Idle (Shell, фоновая sync) | 2–5W | Комнатная |
| Активная работа (приложения, рендеринг) | 5–15W | < 70°C |
| Seeding / индексация | 10–25W | < 80°C |
| Lazy Boot (Headless Logic) | 0.5–2W | Комнатная |

**Оптимизации для мобильных устройств:**
- Adaptive Sync Window снижает частоту P2P-активности при низком заряде
- Display Server переходит в режим пониженного энергопотребления при < 20% батареи
- Background sync ограничивается Wi-Fi (опционально, настраивается пользователем)

---

## Специфические устройства

### Raspberry Pi

| Модель | Статус | Режим |
|--------|--------|-------|
| Pi 4 (4GB) | ✅ Полноценный WORKSPACE | Рекомендуемая конфигурация |
| Pi 5 (8GB) | ✅ Отлично | Максимальная конфигурация |
| Pi Zero 2W | ⚠️ Lazy Boot | Headless Logic + Remote Isolate Call |
| Pi 3 | ⚠️ Ограниченно | Возможен полный WORKSPACE, но медленно (V8 Isolates тяжелы) |

**Особенности:**
- Зашифрованный rootfs (LUKS) по умолчанию для WORKSPACE Base
- Boot-time passphrase через приложение WORKSPACE на телефоне (по QR)

### Apple Silicon

| Чип | Статус |
|-----|--------|
| M1/M2/M3 | ✅ Отлично. Metal 3, Unified Memory, низкое энергопотребление |
| A15+ (iPad) | ✅ Хорошо. Полноценный WORKSPACE, ограничения iOS на фоновые процессы |
| A12 и старше | ⚠️ Ограниченно. Возможен Lazy Boot или ограниченный полный WORKSPACE |

### Windows-планшеты / 2-in-1

| Характеристика | Статус |
|----------------|--------|
| Intel Atom / Celeron | ⚠️ Lazy Boot или медленный полный WORKSPACE |
| Intel WORKSPACE i3+ | ✅ Полноценный WORKSPACE |
| Surface Pro 7+ | ✅ Отлично |

---

## Требования к ИИ

Workspace разделяет ИИ на **обязательный** (базовый Intent Parser) и **опциональный** (Workspace.Mind).

### Обязательный: Intent Parser

Работает всегда, даже на минимальной конфигурации. Не требует GPU.

| Компонент | Минимум | Рекомендация |
|-----------|---------|--------------|
| CPU | 200 MHz (1 ядро) | 500 MHz |
| RAM | 16 MB (модель + словарь) | 32 MB |
| Хранилище | 10 MB (rule-based словари) | 50 MB (lightweight embeddings) |
| Сеть | Не требуется | Не требуется |

**Как работает:**
- Rule-based pattern matching + lightweight NLP
- Локальный словарь Intents ("открой", "найди", "переключи", "создай")
- Базовый Semantic Kernel: FTS5 поиск по SQLite (имена файлов, теги, контакты) без векторных embeddings
- Не использует нейронные сети

### Опциональный: Workspace.Mind (полный AI-стек)

| Компонент | Минимум | Рекомендация | Оптимально |
|-----------|---------|--------------|------------|
| Voice Engine (Whisper) | — | CPU 2 ядра, 2 GB RAM | Apple Neural Engine / NVIDIA GPU |
| OCR (документы) | — | CPU 2 ядра, 2 GB RAM | Apple Neural Engine / NVIDIA GPU |
| Generative UI | — | CPU 2 ядра, 4 GB RAM | GPU с 4GB+ VRAM |
| SLM (Ollama, 7B) | — | CPU 4 ядра, 8 GB RAM | GPU с 8GB+ VRAM |
| Cloud Bridge | 1 Mbps | 10 Mbps | Не ограничено |

**Ограничения при отключении Workspace.Mind:**
- Command Bar работает с клавиатуры (без голоса)
- Нет генеративного UI
- Нет векторного поиска (только FTS5)
- Нет облачных моделей
- **Intent Parser остаётся активным** — базовое понимание текста работает всегда

---

## Диагностика совместимости

При первом запуске Workspace проводит автоматический тест:

1. **GPU Test:** wgpu инициализация, создание swapchain, простой draw call
2. **CPU Test:** запуск тестового V8 Isolate, измерение времени компиляции
3. **RAM Test:** проверка доступной памяти, оценка max isolate count
4. **Network Test:** mDNS broadcast, WireGuard handshake с relay
5. **Storage Test:** скорость записи в SQLite, оценка VFS throughput

**Результат:**
- Все тесты пройдены → "Полноценный WORKSPACE"
- GPU failed → "Ограниченная графика" (программный fallback)
- CPU/RAM failed → "Режим терминала" (Lazy Boot)
- Network failed → "Офлайн-режим"

---

## Рекомендации по железу для сценариев

| Сценарий | Рекомендуемое железо |
|----------|---------------------|
| Домашний пользователь (1–3 устройства) | Любой ПК/ноутбук 2020+, Raspberry Pi 4 как WORKSPACE Base |
| Корпоративный (10–100 сотрудников) | Выделенный сервер (4 ядра, 16GB RAM, SSD) как Supernode + рабочие станции |
| Промышленный (Газпром, Ноябрьск) | Industrial PC с RT-Linux + Bare Metal / Type-1 Hypervisor |
| Образование (дешёвые планшеты) | Lazy Boot + мощный WORKSPACE Base в классе |
| Разработчик (workspace-dev, TSCLANG) | Мощный ПК (8+ ядер, 32GB RAM, дискретная GPU) |

---

## Предыдущий слой

Layer 8 описывает техническую декомпозицию подсистем — Command Bar, Project Manager, Window Manager, App Runtime, Sync Engine, Security Layer и другие компоненты. [См. layer-8-technical-decomposition.md](layer-8-technical-decomposition.md).

---

## Следующий слой

Layer 10 описывает бизнес-модель и Go-to-Market — монетизацию, целевые сегменты, стратегию выхода на рынок и партнёрства. [См. layer-10-business-model.md](layer-10-business-model.md).

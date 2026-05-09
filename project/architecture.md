# CORE OS — Архитектура системы

## Общая схема: 5 уровней

```
┌─────────────────────────────────────────────┐
│  Level 4: Intent API (Core.Mind)            │  ИИ-слой
├─────────────────────────────────────────────┤
│  Level 3: Display Server (Native Canvas)    │  Графика
├─────────────────────────────────────────────┤
│  Level 2: Application Sandbox (V8 Isolates) │  Приложения
├─────────────────────────────────────────────┤
│  Level 1: Micro-Kernel (Bun + TSCLANG)      │  Ядро
├─────────────────────────────────────────────┤
│  Level 0: Host Shim (Rust)                  │  Железо
├─────────────────────────────────────────────┤
│  Host OS (Windows / Linux / macOS / Android)│  Физическая ОС
└─────────────────────────────────────────────┘
```

---

## Level 0: Host Shim (Rust)

Прослойка между физической ОС и нашим миром.

### Задачи
- Захват системных ресурсов без посредников
- Hardware Abstraction Layer

### Компоненты

**Networking (XDP/eBPF):**
- Фильтрация пакетов на уровне драйвера сетевой карты
- Лимит сетевого стека: < 5 MB RAM
- Rate Limiting для защиты от штормов при синхронизации

**Thread Management:**
- Core Pinning: выделение физического ядра под процесс Display Server
- SCHED_FIFO для предотвращения квантовых задержек хост-ОС
- Real-time thread priority через SetThreadPriority / thread_policy_set

**Virtual File System (VFS):**
- Мост к реальному диску
- Ядро видит только абстрактное дерево
- Shim решает: файл на SSD, запись в SQLite или стрим из P2P-туннеля

**Memory Bridge (Zero-copy ABI):**
- SharedArrayBuffer для передачи данных между Shim и V8
- Handle Ownership: объекты в Off-heap, V8 получает External-ссылку
- ARC (Atomic Reference Counting) для тяжелых объектов (100MB+)
- AdjustAmountOfExternalAllocatedMemory — V8 "чувствует" вес нативной памяти
- Memory Pressure Threshold — принудительный GC при распухании

---

## Level 1: Micro-Kernel (Bun + TSCLANG)

Сердце системы.

### Runtime: Bun
- Скорость (встроенный SQLite, TLS, HTTP)
- V8 под капотом
- Поддержка TypeScript из коробки

### Системные модули: TSCLANG
- TypeScript-синтаксис, компиляция в натив через LLVM
- Ручное управление памятью / ARC — нет GC в критических путях
- Прямой маппинг C-типов (structs, pointers)
- SIMD и векторизация для графического движка

### Isolate Management
- Каждое приложение в отдельном V8 Isolate
- Изоляция памяти без накладных расходов на процессы
- Жесткие лимиты: `v8::Isolate::SetResourceConstraints`
- Мгновенная терминация `TerminateExecution()` при превышении

### IPC (Inter-Isolate Communication)
- SharedArrayBuffer + бинарные сообщения (Zero-copy)
- Никакого JSON-парсинга между слоями

### Capability-based Security
- Приложение получает объект `context` при старте
- `fs.read` доступен только для разрешенных хэшей/путей
- Суперюзер определяет права

---

## Level 2: Mesh Engine (P2P / CRDT)

Самая сложная часть. Делает систему "невидимой".

### Протокол
- WireGuard для туннелирования
- libp2p для поиска узлов
- mDNS для обнаружения в локальной сети

### State Sync
- CRDT (Causal Trees + Hybrid Logical Clocks)
- LWW-Element-Set для простых UI-состояний
- Operational Transformation для текста и конфигов
- Graceful Degradation: при потере связи → локальный Fork, потом Merge

### Content Addressing
- Merkle DAG (как в IPFS)
- Файл = хэш. Один файл у двоих в локалке → тянется по P2P
- Дедупликация: один и тот же файл в трех "папках" = одна копия на диске

### Anti-entropy
- Merkle Search Trees (MST) для поиска дельт
- Adaptive Sync Window: экспоненциальная задержка при перегрузке
- Bit-Diff на XOR-дельте (15x экономия трафика)
- Zstd-сжатие с кастомным словарем под структуру Merkle-дерева

### Erasure Coding (FEC)
- Избыточное кодирование вместо ретрансмиссии
- Восстановление данных при потере 30% пакетов без запроса повторов

---

## Level 3: Display Server (Native Canvas)

Забудь про DOM и CSS-движки браузеров.

### Renderer
- WebGPU Pipeline (wgpu-native) или Skia
- Весь интерфейс рисуется как сцена в 3D-игре
- 120 FPS

### Layout Engine
- Yoga-подобный движок (Flexbox) на Rust/TSCLANG
- Координаты кнопок считаются за микросекунды

### Universal Shell
- Системное приложение с высшим приоритетом
- Управляет Z-индексом всех окон
- Живет в отдельном высокоприоритетном процессе
- Если приложение зависло — мышь продолжает двигаться

### Shadow State Recovery
- Shim хранит последний отрендеренный кадр (Bitmap) каждого изолята
- При падении: мгновенно подставляет Shadow Frame
- Пользователь видит фриз 100-120мс, пока Isolate перезапускается
- State восстанавливается из CRDT-графа

### Shared WebGPU Context
- При переходе в Exclusive — графический контекст не реинициализируется
- Переключение режимов = смена размера Canvas (мгновенно, без мерцаний)

---

## Level 4: Intent API (Core.Mind)

ИИ-слой. Не чат-бот, а системный оператор.

### Logic
- Локальный инференс через WebGPU (Llama.cpp / ONNX)
- SLM (Small Language Models) для базовых команд
- Cloud Bridge для сложных задач (с запросом разрешения)

### Intent Map
- Глобальный реестр функций приложений
- ИИ не "угадывает" — он видит типизированный список методов
- Пример: `media.player.pause()` → вызывается напрямую

### Generative UI
- Если готового приложения нет — ИИ собирает данные и пишет JS-код визуализации на лету
- Временный виджет в пространстве юзера

### Голосовой слой (Zero UI)
- Локальный Whisper-движок на отдельном ядре/NPU
- Работает в фоне, даже в Exclusive Mode (игры)
- Не отвлекает от основного экрана

---

## Разделение ответственности

Критический принцип: Shell может падать, ядро продолжает работать.

1. Пользователь кликает иконку в Shell
2. Shell шлет команду в Ядро: "Запусти calculator.pkg"
3. Ядро создает V8 Isolate, дает права, назначает ID окна
4. Калькулятор шлет инструкции отрисовки напрямую в Display Server
5. Display Server накладывает эффекты (прозрачность, блюр) и выводит на экран

### Преимущества разделения
- **Стабильность:** Падение Shell → перезапуск процесса, приложения остаются (Snapshot)
- **Удаленный UI:** Ядро на сервере, Shell на слабом планшете
- **Мультиюзерность:** Одно Ядро, несколько Display Server-ов на разных экранах

---

## Сценарий "Ноябрьск" (Industrial Hardening)

Для промышленных объектов (Газпром и аналоги):

1. **Автономность:** P2P через mDNS + Bluetooth LE. Обновления через физические носители
2. **Изоляция:** Passive Tap для SCADA — нулевое влияние на аварийные клапаны
3. **Self-Healing:** При критической ошибке → откат к Golden Image (Read-only SquashFS)
4. **Type-1 Hypervisor:** Для Hard Real-time — Bare Metal без хост-ОС

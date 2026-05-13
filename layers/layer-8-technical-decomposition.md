# Layer 8 — Technical decomposition | Техническая декомпозиция

Как каждый сценарий из Layer 1 и Layer 3 реализуется технически. Какие подсистемы участвуют, как данные текут между ними, на каком уровне архитектуры каждая живёт. Сценарии установки описаны в [Layer: Установка](layer-4-installation-scenarios.md). Взаимодействие с устройствами и носителями — в [Layer Devices](layer-5-devices.md).

Ссылки: [Layer 3 — System Split](layer-3-system-split.md), [Layer 5 — Devices](layer-5-devices.md), [Layer 2 — AI](layer-2-ai.md), [Layer 7 — Security](layer-7-security.md).

---

## Карта подсистем

```
Workspace Subsystems:

Command Bar      Project Mgr      Window Mgr       App Runtime
  Input Router     Lifecycle        Z-index          V8 Isolates
  Suggest          Sessions         Web-окна         App Registry
  8 режимов        Layout           Нативные         Lifecycle
  Settings         Home             Layout           Sandboxing
                                    Remote Renderer  Permissions UI
                                    Game Mode        Security Hooks
                                    Frame Pacing     Session Mgmt
                                    Raw Input        Accessibility
                                    Low-latency      Warm Recovery
                                                     Secure Txn API

Search &         Communication    Voice Engine     Profile Mgr
Tag Engine       Layer
  Full-text        Messenger        Whisper          Switching
  Tags             Email            Intent Parser    Freeze/Thaw
  Graph            VoIP             Zero UI          Anonymous
  Smart Folders    AI bridge        LED / Buffer     Background
                   Техподдержка

Sync Engine      Security Layer   Display Server   Intent API
  P2P mesh         Key Manager      WebGPU           Semantic Kernel
  CRDT             Auth Proxy       Layout Engine    Intent Map
  Devices          Encrypt          Effects          Generative UI
  Lazy load        Supply Chain     Remote Renderer
  Backup Engine    Incident Resp.   Game Mode
  Energy Manager                    Direct GPU ctx
  Verified Seeding                  Raw Input
                                    Frame Pacing

Host Shim (Level 0)
  eBPF/XDP         OverlayFS        Network Bridge   VFS Bridge
  Zero-copy ABI    dm-verity        Window Mgr       Memory Bridge

Storage Mgr
  Import Engine
  Export Engine
  Deduplication
  Rule Engine
  Device Registry
```

### Уровни архитектуры (справочно)

| Уровень | Что живёт | Ссылка |
|---------|----------|--------|
| Level 0: Host Shim (Rust) | Window, GPU, Input, Memory Bridge, VFS bridge | Host Shim |
| Level 1: Micro-Kernel (Bun) | IPC, Isolate Management, SQLite, Capability Security | Micro-Kernel |
| Level 2: Mesh Engine | P2P, CRDT, Content Addressing, Anti-entropy | Mesh Engine |
| Level 3: Display Server | WebGPU Renderer, Layout Engine, Universal Shell, Effects | Display Server |
| Level 4: Intent API | Semantic Kernel, Intent Map, Generative UI, Voice | Intent API |

---

## VFS — Content Addressable Store

Фундаментальная подсистема хранения данных. Все файлы, метаданные и версии проходят через VFS.

### Архитектура

```
VFS
├── Passport Store (SQLite)
│   ├── files: id, name, cid, size, project_id, owner_id, created_at, updated_at
│   ├── versions: version_id, file_id, cid, timestamp, author_id, diff_cid
│   ├── tags: id, name, color, project_id
│   └── file_tags: file_id, tag_id
│
├── Body Store (blobs/)
│   └── CID = BLAKE3(content) → префикс 2 символа → blob-файл
│
├── Version Store (SQLite, таблица versions)
│   ├── Полные снапшоты: для бинарных файлов (фото, видео, PDF)
│   ├── Diff'ы: для текстовых файлов (XOR-дельта между версиями)
│   └── Retention: старые версии удаляются по политике Owner (default: 30 дней)
│
├── Lazy Load Engine
│   ├── ghost_registry: file_id, cid, available_local, available_on[]
│   ├── on_demand_fetch: запрос blob'а через P2P Mesh
│   └── prefetch_queue: приоритетный фоновый скачивание
│
├── Mirror Engine
│   ├── watched_paths: project_id, host_path, fs_type
│   ├── inotify/FSEvents/ReadDirectoryChanges watcher (Host Shim)
│   └── sync_log: операция (create|delete|rename), host_path, file_id, timestamp
│
└── Deduplication
    └── CID уже существует → новая passport-запись, blob не копируется
```

### Content Addressable Storage

- **CID (Content Identifier)** = BLAKE3-хеш содержимого файла. 256 бит, hex-кодирование
- **Дедупликация:** два одинаковых файла = один blob на диске. Passport-записи ссылаются на один CID
- **Целостность:** при чтении blob пересчитывается BLAKE3. Если не совпадает с CID → corruption detected → запрос с другого устройства через P2P
- **Merkle DAG:** опционально, для пакетной верификации. Корневой хеш группы файлов = хеш от concatenated CID'ов. Используется при seeding/backup для проверки целостности пакета

### Passport + Body separation

- **Passport** (SQLite) — «лёгкий» слой: метаданные, теги, права, история версий, ссылки на CID. Мгновенный поиск по миллионам файлов
- **Body** (blobs/) — «тяжёлый» слой: реальные байты. Управляется жизненным циклом: если ни один passport не ссылается на CID → blob удаляется (garbage collection)

### Version Store

**Copy-on-Write для бинарных файлов:**
- Пользователь отредактировал `договор.docx` → новый blob (новый CID) → новая запись в `versions`
- Старый CID остаётся в blobs/ до истечения retention

**Diff'ы для текстовых файлов:**
- Исходная версия: полный blob
- Каждое последующее изменение: XOR-дельта между старой и новой версией
- Восстановление: исходный blob + применение цепочки diff'ей
- Экономия: правка в одном слове = хранится только изменение, не весь файл

**Retention policy:**
- Owner настраивает: сколько дней хранить версии, сколько версий максимум
- Фоновая задача (Scheduler) удаляет устаревшие версии и осиротевшие blob'ы

### Lazy Load Engine

**Ghost-файлы:**
- Запись в passport с CID, но blob отсутствует локально
- `available_local = false` → UI показывает облачко рядом с именем файла
- При открытии: Lazy Load Engine отправляет запрос в Sync Engine → P2P Mesh → устройство с blob'ом
- Фоновое скачивание: если файл добавлен в проект на ПК, телефон получает ghost-запись мгновенно, а blob подтягивается в фоне

**Prefetch:**
- Паттерн: пользователь открыл файл 1 из папки → система предзагружает файл 2, 3, 4 (соседи в passport)
- Плейлист: все треки помечены как `prefetch_priority = high` → загружаются при подключении Wi-Fi

### Mirror Engine

**Двусторонняя синхронизация с хост-ОС:**

```
Windows: D:\Work
  |
  v
Host Shim (ReadDirectoryChanges)
  |
  v
Mirror Engine:
  +-- Новый файл "отчет.xlsx" → создать passport-запись → вычислить CID → импорт blob'а
  +-- Удалён файл "старый.doc" → архивировать passport (мягкое удаление)
  +-- Переименован файл → обновить имя в passport
  +-- Изменён файл → новая версия (новый CID)
  |
  v
Storage Manager → VFS (passport + body)
```

**Обратное направление (CORE → хост-ОС):**
- Файл создан в CORE → Host Shim создаёт файл в `D:\Work` через стандартные API хост-ОС
- Файл удалён в CORE → удаление в `D:\Work`
- Теги: если хост-ОС поддерживает расширенные атрибуты (NTFS ADS, xattr) → теги записываются туда. Иначе — теги только в SQLite CORE

**Watcher API (Host Shim):**
- Windows: `ReadDirectoryChangesW` на mirror-пути
- Linux: `inotify` на mirror-пути
- macOS: `FSEvents` на mirror-пути
- События: `create`, `delete`, `rename`, `modify` → debounce 100ms → batch → Mirror Engine

### Карта связей VFS

| UX-сценарий | Подсистема | Architecture Level |
|-------------|-----------|-------------------|
| Открыть файл | Passport Store → Body Store | Level 1 |
| История версий | Version Store | Level 1 |
| Откат к версии | Version Store → replace CID в passport | Level 1 |
| Дедупликация | CID comparison при импорте | Level 1 |
| Lazy Load | Lazy Load Engine + Sync Engine | Level 1 + Level 2 |
| Prefetch | Lazy Load Engine (фоновая задача) | Level 1 |
| Mirror Folder | Mirror Engine + Host Shim | Level 0 + Level 1 |
| Garbage Collection | orphaned blob removal | Level 1 (Scheduler) |
| Целостность blob | BLAKE3 recalc on read | Level 1 |

---

## Host Shim (Level 0)

**Что:** Единственный слой контакта с хост-ОС. Rust + winit + wgpu + CPAL + HID.

**Компоненты:**

```
Host Shim (Rust):
  |
  +-- Window Manager (winit): окно, resize, DPI, multi-monitor
  +-- WebGPU Backend (wgpu-native): Vulkan / Metal / DX12 / OpenGL ES fallback
  +-- Audio (CPAL): микрофон, динамики, аудиопотоки
  +-- Input (HID): клавиатура, мышь, тачпад, геймпад
  +-- VFS Bridge: чтение/запись файлов хост-ОС, inotify/FSEvents/ReadDirectoryChanges
  +-- Network Bridge: raw sockets для WireGuard
  +-- Memory Bridge: SharedArrayBuffer между Rust и Bun
```

**Software Renderer / CPU Fallback:**
- Если GPU не поддерживает WebGPU → Host Shim переключается на программный рендеринг (CPU) или OpenGL ES fallback
- Производительность падает до 15–30 FPS, эффекты (blur, shadows) отключаются

**Memory Bridge / Zero-copy ABI:**

```
TSCLANG / Rust (Host Shim, Level 0)
  |
  +-- Создаёт объект в Off-heap памяти (C-layout)
  +-- Оборачивает в v8::External → передаёт в V8 Isolate
  +-- JS-сторона видит ArrayBuffer, указывающий на ту же память
  +-- ARC (Atomic Reference Count): Rust держит счётчик, V8 — Weak Callback
  +-- При GC обёртки → декремент счётчика → освобождение нативной памяти
```

**External Memory Tracking:**
- `AdjustAmountOfExternalAllocatedMemory`: V8 GC чувствует вес нативных блобов
- Большой External-блоб (100 MB+) триггерит GC раньше, чем куча V8 забьётся
- Memory Pressure Threshold: если «ожидающая» нативная память > 20% от доступной → принудительный `LowMemoryNotification()`

**Где:** Level 0 (Host Shim, Rust) ↔ Level 1 (Micro-Kernel, V8/Bun).

**Storage Watcher:**
- Подписка на события хост-ОС: udev (Linux), WM_DEVICECHANGE (Windows), DiskArbitration (macOS)
- Уведомляет Storage Manager о подключении/отключении носителей

**Где:** Level 0 (Rust, системные вызовы).

### eBPF / XDP (Core Base only)

**Что:** На выделенном железе (Core Base, Raspberry Pi) Host Shim использует eBPF/XDP для низкоуровневой обработки сетевого трафика — P2P mesh, WireGuard, rate limiting.

**Как:**

```
P2P Mesh (Level 2) -> Host Shim Network Bridge
  |
  +-- Обычные устройства (Windows/macOS/Linux): raw sockets через std API
  +-- Core Base (Linux без хост-ОС):
      +-- XDP program (eBPF) — фильтрация пакетов на уровне NIC driver
      +-- Rate limiting: пакеты сверх лимита дропаются в ядре, не доходят до userspace
      +-- WireGuard: обработка handshake в eBPF, data packets — fast path
      +-- DDoS mitigation: SYN flood / UDP flood фильтруются на NIC
```

**Почему только Core Base:** На Windows/macOS eBPF/XDP недоступен — CORE работает как приложение. На Core Base (Linux bare metal) — полный контроль над сетевым стеком.

**Где:** Level 0 (Host Shim, eBPF loader + XDP program).

### OverlayFS / Immutable Core

**Что:** Системные файлы Workspace неизменяемы. Обновления применяются атомарно через overlay: новая версия монтируется поверх, старая остаётся для rollback.

**Как:**

```
Core Base / Raspberry Pi (Linux):
  |
  +-- Lowerdir (read-only): системные файлы Workspace (/usr/core/, /lib/core/)
  +-- Upperdir (read-write): пользовательские данные, кэш, логи
  +-- Workdir: рабочая директория OverlayFS
  +-- Merged: то, что видит CORE runtime
  |
  v Обновление:
      +-- Новая версия CORE скачивается в /var/core/updates/v2/
      +-- Atomically remount: lowerdir = v2, upperdir сохраняется
      +-- Если v2 не стартует → rollback к v1 за 3 секунды
      +-- Если v1 тоже не стартует → fallback на «Золотой образ» (golden lowerdir, read-only SquashFS)
      +-- Если v2 работает 24ч → v1 удаляется (garbage collect)
```

**Безопасность:**
- Lowerdir монтируется read-only + verity (dm-verity на Linux, signed catalog на Windows)
- Подмена системного файла → verity mismatch → отказ загрузки → rollback
- Upperdir шифруется AES-256-GCM (ключ на TPM/device key)

**Где:** Level 0 (Host Shim, mount/verity) + Level 1 (Update Engine, atomic switch).

> Подробно: security-аспекты immutable rootfs, dm-verity, rollback attacks — в [Layer Security, §22.5.4](layer-7-security.md).

### CPU Affinity Policy (Core Pinning)

**Что:** Critical threads закрепляются на dedicated CPU cores для минимального latency.

**Как:**

```
Adaptive Core Pinning (Host Shim, Level 0):
  |
  +-- Core Base (Linux bare metal):
  |   +-- 2 ядра: pin только render thread Display Server
  |   +-- 4+ ядер: pin весь Display Server process + Voice Engine thread
  |   +-- SCHED_RR (round-robin) + high priority — без system hang risk
  |   +-- Watchdog: если pinned thread не отвечает 100мс → forced unpause + restart
  |
  +-- Windows / macOS (CORE как приложение):
  |   +-- Best effort: HIGH_PRIORITY_CLASS / QOS_CLASS_USER_INTERACTIVE
  |   +-- Без explicit pinning — ОС не даёт права
  |
  +-- Integrity gate:
      +-- Pin устанавливается только если BLAKE3-hash бинарника совпадает с expected
      +-- Integrity Monitoring Agent проверяет hash перед установкой affinity
```

**Почему SCHED_RR, а не SCHED_FIFO:** FIFO + deadlock = system hang. RR вытесняется по кванту времени — безопаснее.

**Где:** Level 0 (Host Shim, thread scheduler).

---

## 1. Command Bar (Строка)

### 1.1 Input Router

**Что:** Определяет режим строки по формату ввода.

**Как:**

```
Пользователь вводит текст
        |
        v
  Input Router (Level 1, Bun)
        |
        +-- Регулярные выражения / паттерны:
        |   +-- /^@/             -> Мессенджер
        |   +-- /@\w+\.\w+/      -> Email
        |   +-- /^\+\d/          -> Мессенджер (звонок как опция)
        |   +-- /^(\$|>)/         -> Терминал
        |   +-- /\.(com|ru|org)/  -> Браузер
        |   +-- /[\d+\-*/^()]+/   -> Калькулятор
        |   +-- /напомни|завтра/  -> Напоминание
        |   +-- fallback           -> Поиск + ИИ-агент
        |
        +-- Голос: первое слово -> Intent Parser -> режим
        |
        +-- Ручной: клик по иконке / горячая клавиша (Tab / 1-8)
        |
        v
  Режим определён -> UI строки обновляется (иконка, placeholder)
```

**Где:** Level 1 (Micro-Kernel), работает в главном Isolate (Shell).

**Если не угадал:**
- Router выдаёт top-3 варианта с confidence score
- Пользователь выбирает стрелками
- Результат записывается в персональную историю -> следующий раз точнее

### 1.2 Suggestion Engine

**Что:** Подсказки при вводе — приложения, действия, контакты.

**Как:**

```
Каждый символ ввода
        |
        v
  Suggestion Engine (Level 1)
        |
        +-- Источник 1: App Registry -- совпадение по имени/тегам
        +-- Источник 2: Search Index -- файлы, проекты
        +-- Источник 3: Contact Book -- люди, email
        +-- Источник 4: History -- недавние действия
        +-- Источник 5: Intent Map -- доступные команды приложений
        |
        v
  Ранжирование:
        1. Точное совпадение имени приложения
        2. Контекст текущего проекта
        3. Частота использования
        4. Давность
        |
        v
  Выпадающий список под строкой (Level 3, Display Server)
  Первый элемент выделен -> Enter активирует
```

**Где:** Level 1 (поиск по SQLite + индекс) + Level 3 (рендеринг выпадающего списка).

### 1.3 Восемь режимов

#### Поиск

```
Ввод -> Search Engine (Level 1, SQLite + FTS)
     -> Результаты в выпадающем списке
     -> Enter -> открывает файл/проект/контекст
```

**Подсистемы:** Search & Tag Engine (секция 5).

#### Заметка

```
Ввод -> App: Notes (Level 2, V8 Isolate)
     -> Создаёт заметку в текущем проекте
     -> Теги извлекаются автоматически (#хештеги)
     -> Заметка появляется в графе связей проекта
```

**Подсистемы:** App Runtime (секция 4), Project Manager (секция 2).

#### Напоминание

```
Ввод -> Intent Parser (Level 4) -> извлекает дату/время
     -> Scheduler (Level 1) -> регистрирует событие
     -> В момент времени -> Push Notification (Level 3)
     -> Задача привязана к текущему проекту
```

**Подсистемы:** Intent API (Level 4), SQLite (хранилище задач), Display Server (уведомления).

#### Калькулятор

```
Ввод -> Math Parser (Level 1)
     -> Результат в строке (красивая нотация через Level 3)
     -> История -> SQLite, привязана к проекту
     -> "Построить график" -> App: Calculator (V8 Isolate)
```

**Подсистемы:** Собственный math-модуль в ядре, App Runtime для продвинутых функций.

#### Терминал

```
Ввод ($ команда) -> Shell Process (Level 0, Host Shim)
                 -> PTY (псевдотерминал) -> вывод в выпадающем списке
                 -> Или открытие полного окна терминала в проекте
```

**Подсистемы:** Host Shim (создание процессов, PTY), Window Manager (окно терминала).

#### Браузер

```
Ввод (URL) -> Island Mode (Level 2, Chromium sandbox)
           -> Создаёт веб-окно в текущем проекте
           -> Window Manager размещает окно
```

**Подсистемы:** Window Manager (секция 3), App Runtime (Island Mode).

#### Мессенджер

```
@ivan       -> Contact Book -> находит контакт
            -> Открывает чат (выпадающий список или окно)
email@domain-> Форма написания письма
+7999...    -> Контакт по номеру -> мессенджер по умолчанию
            -> Опция "Позвонить" рядом с чатом
```

**Подсистемы:** Communication Layer (секция 6).

#### ИИ-агент

```
Ввод -> Intent API (Level 4)
     -> Semantic Kernel анализирует запрос
     -> Определяет цепочку действий через Intent Map
     -> Выполняет: вызывает методы приложений, ищет файлы, создаёт заметки
     -> Результат: текст в строке / Generative UI / действие
```

**Подсистемы:** Intent API (Level 4), все остальные подсистемы как исполнители.

### 1.4 Настройки строки

**Что:** Позиция, размер, отступы, форма, внешний вид.

**Как:**

```
Настройки хранятся в:
  SQLite -> таблица shell_settings
  |
  +-- bar_position: "bottom" | "top"
  +-- bar_halign: "center" | "left" | "right"
  +-- bar_width: "adaptive" | "full" | число (px)
  +-- bar_height: { min: число, max: число }
  +-- bar_padding: { top, right, bottom, left }
  +-- bar_margin: { top, right, bottom, left }
  +-- bar_radius: число | { tl, tr, bl, br }
  +-- bar_bg_color: string
  +-- bar_bg_image: string (путь)
  +-- bar_opacity: 0..1
```

**Где:** Level 1 (хранение) + Level 3 (рендеринг). Universal Shell читает настройки при запуске и применяет к layout.
---

## 2. Project Manager

### 2.1 Жизненный цикл проекта

```
Создание:
  Пользователь -> "Новый проект" / кнопка ⊕ / голос
  -> Project Manager создаёт запись в SQLite:
      id, name, created_at, tags, icon, color
  -> Создаёт пустой layout
  -> Проект становится активным

Переключение:
  Текущий проект -> заморозка (save state)
  Новый проект -> загрузка state -> рендеринг

Удаление:
  Проект -> архив (мягкое удаление)
  -> Через 30 дней -> физическое удаление
  -> Или мгновенное по подтверждению
```

**Где:** Level 1 (Micro-Kernel). Project Manager — системный модуль Bun.

### 2.2 Session Persistence

**Что:** Автосохранение окон, позиций, состояний приложений.

**Как:**

```
Каждое изменение layout:
  |
  +-- Окно перемещено -> новый layout record
  +-- Окно resized -> resize event -> layout record
  +-- Приложение изменило state -> app state -> CRDT-операция
  |
  v
Layout Store (SQLite):
  project_id, window_id, app_id, x, y, w, h, z_index, state, updated_at
  |
  v
CRDT Layer -> синхронизация на другие устройства
```

**Чистая сессия:**
- Флаг `ephemeral: true` -> при закрытии проекта все записи layout удаляются
- Приложения получают уведомление -> очищают свои данные

**Где:** Level 1 (SQLite + CRDT) + Level 2 (Mesh для синхронизации).

### 2.3 Home Project

**Что:** Нулевой проект "Home" — главный экран. Всегда существует, не удаляется.

**Как:**
- При первом запуске создаётся автоматически
- Имя = имя пользователя
- Layout = пустой (только строка)
- Закреплённые проекты отображаются как иконки на главном экране
- Home Project = контекст по умолчанию для строки

**Где:** Level 1. Специальная запись в Project Manager с `id = "home"`.

### 2.4 Layout Engine (сплит, мульти-мониторы)

**Что:** Размещение окон внутри проекта. Сплит проектов.

**Как:**

```
Один экран:
  Project Layout -> tree of containers
  +-- Split (horizontal/vertical)
  |   +-- Window (app instance)
  |   +-- Window (app instance)
  +-- Tab Group
      +-- Window A
      +-- Window B

Несколько экранов:
  Screen Manager (Level 0, Host Shim)
  |
  +-- Monitor 1 -> Project A (full layout)
  +-- Monitor 2 -> Project B (full layout)
  +-- Monitor 1+2 -> Project C (layout spans both)
  |
  v
  Display Server рендерит layout для каждого монитора
  Host Shim отслеживает подключение/отключение мониторов
```

**Перетаскивание:**
- Окно за край экрана -> переброска на другой монитор
- Проект за край -> предложение открыть на втором мониторе

**Где:** Level 0 (мониторы, Host Shim) + Level 3 (Display Server, layout) + Level 1 (Project Manager, state).

---

## 3. Window Manager

### 3.1 Z-индекс, фокус, перетаскивание

```
Universal Shell (Level 3)
  |
  +-- Z-стек: список окон по порядку (фоновые -> активное)
  +-- Фокус: только одно окно получает keyboard input
  +-- Перетаскивание: Host Shim -> mouse events -> Shell -> обновление layout
  +-- Примагничивание (snap): к краям, к другим окнам, к половинам экрана
```

**Принцип:** Shell может падать, но окна продолжают рендериться. Shadow State Recovery восстанавливает состояние из SQLite + CRDT при перезапуске.

**Где:** Level 3 (Universal Shell) + Level 0 (input через Host Shim).

### 3.2 Web-окна (Island Mode)

**Что:** Сайты и веб-приложения в изолированном Chromium-движке.

**Как:**

```
URL введён в строке
  |
  v
App Runtime -> создаёт Island Process
  |
  +-- Chromium sandbox (один на веб-окно)
  +-- Изоляция: нет доступа к системным API CORE
  +-- Tabs: внутри каждого веб-окна свои вкладки
  |   +-- Ссылки открываются как вкладки внутри того же окна
  |   +-- Новое веб-окно — только через Command Bar (набрал URL)
  +-- Incognito: флаг при создании Island Process
  +-- DevTools: F12 -> встроенный инспектор Chromium
  |
  v
Window Manager -> размещает Island как обычное окно в проекте
```

**Где:** Level 2 (App Runtime, V8 Isolate + Chromium embed) + Level 3 (Window Manager).

### 3.3 Нативные окна (WebGPU)

**Что:** Собственные приложения CORE — рендер через WebGPU напрямую.

**Как:**

```
Приложение (V8 Isolate)
  |
  +-- Вызывает Core.Graphics API (Level 3)
  +-- Отправляет draw commands -> WebGPU Pipeline
  +-- Display Server композитит с другими окнами
  +-- Effects: blur, transparency, shadows -> Display Server
```

**Где:** Level 2 (приложение) + Level 3 (Display Server, WebGPU).

#### 3.3.1 Remote Renderer

**Что:** Бэк рендерит UI и стримит bitmap на Фронт. Для слабых устройств, которым нужно запустить тяжёлое приложение (видеоредактор, 3D). Как GeForce Now / Citrix.

**Архитектура:**

```
Бэк (V8 Isolate + WebGPU)
  |
  +-- Рендеринг в offscreen texture
  +-- Кодирование в video stream (H.264/AV1)
  +-- TLS-шифрованный bitmap-стрим -> Фронт
  |
  v
Фронт (Display Server)
  +-- Декодирование
  +-- Отображение как обычное окно
  +-- Ввод (мышь/клавиатура) -> обратно на Бэк
```

**Security:**
- Bitmap-стрим шифруется TLS (внутри WireGuard-туннеля)
- Бэк защищён от screen scraping на стороне сервера (watermark / blur для confidential Space)
- Фронт не имеет доступа к логике приложения — только к пикселям

**Где:** Level 2 (Бэк, V8 Isolate + WebGPU) + Level 3 (Display Server, decoder).

#### 3.3.2 Static UI Overlay

**Что:** Если CPU задушен (400 МГц, 256 МБ RAM) или V8 Isolate падает — система показывает статичный bitmap-интерфейс вместо живого рендеринга.

**Как:**

```
Пользователь нажимает кнопку
  |
  v
Display Server проверяет: V8 Isolate готов? CPU > порога?
  |
  +-- Да → живой рендеринг через WebGPU
  +-- Нет → Static UI Overlay:
      +-- Последний snapshot интерфейса (bitmap в GPU memory)
      +-- Активные зоны (hotspots) для тач/клика
      +-- Действие пользователя ставится в Intent Queue
      +-- Как только V8 «продышится» — очередь исполняется пачкой
```

**Результат для пользователя:** не «фриз 12 секунд», а «медленный, но отзывчивый интерфейс». Нажал «Save» → интерфейс моргнул «Принято», файл сохранился через 30 секунд в фоне.

**Где:** Level 3 (Display Server, GPU texture cache) + Level 1 (Intent Queue).

### 3.4 Тайловый и плавающий layout

**Что:** Два режима размещения окон внутри проекта.

**Тайловый (по умолчанию):**
- Окна автоматически заполняют пространство
- Нет перекрытий, нет пустот
- Разделители между окнами — перетаскиваются

**Плавающий:**
- Свободное размещение как в обычных ОС
- Перекрытие, свободный resize

**Переключение:** настройка проекта или голосом.

**Где:** Level 3 (Display Server, Layout Engine).

### 3.5 Полноэкранный layout

**Что:** Окно занимает 100% viewport монитора. Не путать с Exclusive-режимом — это layout внутри Managed Space, а не захват GPU/ввода у хост-ОС.

**Как:**

```
Приложение вызывает requestFullscreen()
  |
  v
Window Manager (Level 3)
  |
  +-- Сохраняет предыдущий layout (state_before_fullscreen)
  +-- Убирает рамки и декорации окна
  +-- Скрывает Command Bar (или сворачивает в indicator line)
  +-- Устанавливает окну z_index = MAX, размер = viewport монитора
  |
  v
Display Server (Level 3)
  +-- Рендерит только это окно, без композитинга остальных
  +-- Все input events -> это окно (кроме зарезервированных жестов)
```

**Выход из полноэкрана:**
- `Esc` -> Window Manager восстанавливает state_before_fullscreen
- Системный жест (свайп от края, угловой жест) -> Input Router (Level 2) -> Window Manager
- Panic Gesture (тройное касание угла) -> Input Router перехватывает на уровне Host Shim, принудительный выход

**Где:** Level 3 (Window Manager + Display Server) + Level 2 (Input Router, жесты).

### 3.6 Game Mode API

**Что:** Execution profile для игр и real-time приложений. Система отдаёт игре максимум железа, минуя оверхед композитора и оконного менеджера.

**Не путать с полноэкранным layout:** Полноэкранный layout — это просто размер окна. Game Mode — это захват GPU context, raw input и low-latency audio.

**Как:**

```
Приложение вызывает Core.Graphics.requestGameMode()
  |
  v
Display Server проверяет:
  +-- Приложение level >= 4? (verified publisher)
  +-- Пользователь подтвердил в Permissions UI?
  |
  v
Game Mode activated:
  +-- Direct GPU Context:
  |   +-- Display Server отдаёт swapchain напрямую приложению (без композитинга)
  |   +-- Игра рисует в framebuffer напрямую (wgpu surface без intermediate texture)
  |   +-- VSync control: приложение само управляет present timing (frame pacing)
  |
  +-- Raw Input:
  |   +-- Host Shim перенаправляет HID events напрямую в Isolate (без Input Router)
  |   +-- Исключение: Panic Gesture (тройное касание угла) — всегда ловится Host Shim
  |   +-- Клавиатура/мышь/gamepad → напрямую в V8 Isolate
  |
  +-- Low-latency Audio:
  |   +-- CPAL в режиме REALTIME (минимальный буфер, exclusive stream)
  |   +-- Без системного микшера (no system audio ducking)
  |
  +-- CPU Affinity:
  |   +-- Game thread pinned на dedicated core (см. Host Shim, Core Pinning)
  |   +-- SCHED_RR + high priority (Core Base)
  |
  +-- Network Overlay (опционально):
      +-- UDP-based real-time socket (не CRDT, не TCP)
      +-- Для multiplayer: unreliable, ordered, low-latency
      +-- Шифрование: DTLS (Datagram TLS) — без лишнего RTT
```

**Frame Pacing:**
- Игра запрашивает target FPS (60/120/144/240)
- Host Shim настраивает VSync + present mode (FIFO_RELAXED для Variable Refresh Rate)
- Если игра не укладывается в target — automatic resolution scaling (render scale 0.5–1.0)

**Переключение контекста (Alt+Tab):**

Когда пользователь переключается из Game Mode в Shell (Alt+Tab, Panic Gesture, уведомление):

1. **Display Server** не уничтожает GPU-контекст, а переводит его в **shadow state**: последний валидный framebuffer сохраняется в dedicated GPU memory region
2. **Isolate игры** приостанавливается (freeze), не убивается — состояние сериализуется в checkpoint (как в Warm Recovery)
3. **Display Server** возвращается к композиции: shadow framebuffer используется как «застывший» background для рабочего стола
4. **Переключение назад** (повторный Alt+Tab):
   - Resume isolate из checkpoint (< 100 мс)
   - GPU-контекст восстанавливается из shadow state
   - Игра продолжает с того же кадра (если поддерживает pause-on-focus-lost) или с минимальным откатом

**Время переключения:**
- Shell → Game Mode: 1–2 кадра (16–33 мс на 60 Гц)
- Game Mode → Shell: мгновенно (shadow framebuffer уже в памяти)
- Game Mode → Shell → Game Mode: < 100 мс (resume isolate)

**Исключение — exclusive fullscreen (Vulkan/DirectX):**
- Если игра требует exclusive GPU access (не borderless), переключение требует recreate swapchain: 100–300 мс
- Host Shim предпочитает borderless mode, но предоставляет exclusive по запросу приложения

**Выход из Game Mode:**
- `Esc` (удержание 2 сек) → Window Manager восстанавливает композитинг
- Panic Gesture → Host Shim принудительно убивает isolate, возвращает Display Server
- Game crashed → Shadow State Recovery → fallback к Static UI Overlay

**Безопасность:**
- Game Mode — только для приложений уровня 4+ (verified publisher)
- Direct GPU context обнуляется перед передачей (GPU memory zeroize)
- Raw input не перехватывает системные жесты (Panic Gesture, screenshot)
- Host Shim watchdog: heartbeat каждые 50 мс, dead isolate → kill + recovery

**Где:** Level 3 (Display Server, direct surface) + Level 0 (Host Shim, GPU/input/audio) + Level 2 (Mesh Engine, UDP overlay).

> Подробно: GPU memory isolation, input sandbox, Game Mode security hooks — в [Layer Security, §22](layer-7-security.md).

### 3.7 Optimistic Rendering

**Что:** UI рисует локальные изменения мгновенно, не дожидаясь подтверждения от других устройств. Если прилетает "проигравший" winner по хэшу — экран **не откатывается назад**.

**Как:**

```
Пользователь ввёл текст / переместил окно / изменил файл
  |
  v
Display Server (Level 3)
  |
  +-- Применяет изменение локально и мгновенно (0ms feedback)
  +-- Отправляет мутацию в CRDT Layer (Level 2) → Sync Engine → P2P Mesh
  |
  v
Если прилетает remote winner (Hash-based Ordering выбрал другое значение):
  +-- UI **не откатывает** состояние назад (это бесит)
  +-- Показывает индикатор синхронизации: «Обновлено на другом устройстве»
  +-- Плавно применяет winner через 300ms fade / blink
  +-- Пользователь видит результат, а не конфликт
```

**Где:** Level 3 (Display Server) + Level 2 (CRDT Layer).

---

## 4. App Runtime

Модель приложений — 5 уровней интеграции: от «набрал URL» до «полный натив на WebGPU». Манифест `core.json`, app-scoped SQLite, `@core/*` API, App Registry, безопасность — подробно в [Layer Apps](layer-6-apps.md).

### 4.1 V8 Isolates

**Что:** Каждое приложение в отдельном V8 Isolate — изоляция памяти, квоты ресурсов.

**Как:**

```
Запуск приложения:
  |
  v
App Runtime (Level 1)
  |
  +-- Создаёт V8 Isolate
  +-- Устанавливает ResourceConstraints:
  |   +-- max_old_generation_size: по квоте приложения
  |   +-- max_young_generation_size: по квоте приложения
  +-- Загружает код приложения из App Registry
  +-- Передаёт context object:
  |   +-- fs.read (только разрешённые пути)
  |   +-- graphics (WebGPU draw commands)
  |   +-- mind (Intent API registration)
  |   +-- network (ограниченный доступ)
  +-- Запускает execution
  |
  v
Мониторинг:
  +-- Превышение памяти -> TerminateExecution()
  +-- Зависание (no response 5s) -> предложение убить
  +-- Падение -> Shadow State Recovery -> перезапуск
```

**Resource Quotas и Memory Pressure:**

| Лимит | Что | Действие при превышении |
|-------|-----|------------------------|
| max_old_generation_size | Лимит heap V8 | `TerminateExecution()` + уведомление пользователя |
| max_young_generation_size | Лимит young generation | Форсированный minor GC |
| cpu_quota | % CPU per isolate | Throttle до базового приоритета |
| external_memory_limit | Лимит External ArrayBuffer | Блокировка новых allocation до освобождения |

**Memory Pressure Handler:**
- Host Shim мониторит доступную RAM устройства
- При < 20% свободной памяти → `v8::Isolate::LowMemoryNotification()` для всех isolates
- При < 10% → приостановка фоновых isolates, освобождение кэша
- При < 5% → graceful termination неактивных приложений с сохранением state

### 4.1.1 Secure Transaction API

**Что:** Биометрическая подпись данных с использованием device key + secure enclave / TPM. Приложение не видит приватный ключ — только результат подписи.

**Как:**

```
Приложение (банк, крипто-кошелёк):
  |
  +-- Запрашивает capability: "secure_sign" в манифесте
  +-- Пользователь подтверждает в Permissions UI
  |
  v
Вызов API:
  Core.Security.signWithBiometry({
    data: transaction_payload,     // данные для подписи
    key_id: "payment_key_001",     // ID ключа (не сам ключ!)
    biometry: "required",          // FaceID / TouchID / Windows Hello
    confirmation_ui: true          // показать системный overlay с деталями
  })
  |
  v
Micro-Kernel (Level 1):
  +-- Проверяет capability: есть ли "secure_sign"?
  +-- Проверяет key_id: принадлежит ли он этому приложению?
  |
  v
Display Server (Level 3) — системный overlay:
  +-- Рендерит детали транзакции (сумма, получатель, комиссия)
  +-- Overlay рисуется системой, не приложением — защита от spoofing
  +-- Пользователь подтверждает: biometric prompt (FaceID / TouchID / PIN)
  |
  v
Host Shim (Level 0) — Secure Enclave / TPM:
  +-- Приватный ключ никогда не покидает secure enclave
  +-- Подпись вычисляется внутри enclave: sign(data, private_key) → signature
  +-- Возвращается только signature (Ed25519)
  |
  v
Приложение получает signature + public_key_fingerprint
```

**WYSIWYS (What You See Is What You Sign):**
- Детали транзакции рендерит Display Server, не приложение
- Приложение не может подменить сумму или получателя — пользователь видит системный overlay
- Overlay защищён от скриншотов и screen readers (Secure Field)

**Ключи:**
- Каждое приложение получает app-scoped signing key (derived от device key)
- Ключи хранятся в TPM / Secure Enclave / Android Keystore / Apple Secure Enclave
- Backup: ключи шифруются recovery-фразой Owner, хранятся в зашифрованном CRDT

**Rate limiting:**
- Max 10 подписей / мин на приложение
- Max 100 подписей / час на профиль
- Превышение → временная блокировка + уведомление Owner

**Audit:**
- Каждая подпись записывается в audit log (категория "transaction")
- Запись: timestamp, app_id, key_id, data_hash (BLAKE3), signature, biometry_result

**Где:** Level 0 (Host Shim, TPM/Secure Enclave) + Level 1 (Micro-Kernel, capability check) + Level 3 (Display Server, confirmation overlay).

> Подробно: WYSIWYS overlay, audit log, rate limiting, side-channel защита — в [Layer Security, §22](layer-7-security.md).

**Idle CPU Policy («Холодный камень»):
- Host Shim мониторит CPU usage каждого isolate через `v8::Isolate::GetHeapStatistics()` + OS-level CPU accounting
- Фоновый isolate (background mode) потребляет > 0.5% CPU > 30 секунд подряд → принудительный Suspend
- Suspend: сериализация state в encrypted SQLite (per-profile) → `v8::Isolate::Dispose()` → 0% CPU
- Wake-up: по событию (push, user action, timer). Rate limit: max 10 wake-ups/min на isolate. Превышение → isolate помечается suspicious, Owner уведомляется
- Pinned isolates (Display Server, Voice) — exempt из «Холодного камня»

**Где:** Level 1 (Micro-Kernel, Bun + V8) + Level 0 (Host Shim, CPU monitor).

### 4.2 App Registry

**Что:** Реестр всех доступных приложений. Установка = адрес. Модель приложений (5 уровней, манифест, хранение, магазин) — в [Layer Apps, секция App Registry](layer-6-apps.md).

**Как:**

```
App Registry (SQLite):
  |
  +-- Системные (встроены):
  |   +-- Core.Notes -- заметки
  |   +-- Core.Calculator -- калькулятор
  |   +-- Core.Player -- медиаплеер
  |   +-- Core.Terminal -- терминал
  |   +-- Core.Files -- файловый менеджер
  |   +-- Core.Settings -- настройки
  |
  +-- Веб-приложения (по адресу):
  |   +-- youtube.com -> Island Mode, без установки
  |
  +-- Сторонние (магазин):
      +-- pkg.core.app/spotify -> загрузка кода -> V8 Isolate
```

**Установка:**
- Набрал имя/адрес -> код загружен в кэш
- Закрыл -> Isolate уничтожен, память освобождена
- Код остаётся в кэше для быстрого следующего запуска

**Удаление:**
- Очистка кэша -> не осталось ни байта
- Нет реестра Windows, нет `/Library`, нет `.config`

**Где:** Level 1 (SQLite, кэш) + Level 2 (загрузка из сети/магазина).

### 4.3 Lifecycle

```
Не запущено -> Кэш (код на диске, ~0 RAM)
     |
     v Запуск
Активно -> V8 Isolate, окно в проекте, полный доступ к квоте
     |
     v Сворачивание / потеря фокуса
Приостановлено -> Isolate заморожен, память удерживается
     |                (быстрое восстановление)
     v Закрытие
Уничтожено -> Isolate убит, память освобождена, кэш кода остаётся
```

#### 4.3.1 Warm Recovery

**Что:** Если Isolate падает (panic, OOM, deadlock) — система восстанавливает его состояние из последнего checkpoint за < 100 мс. Пользователь видит «моргание», а не перезапуск приложения.

**Как:**

```
Isolate работает (Активно):
  |
  +-- Checkpoint каждые 5 сек (async, не блокирует UI):
  |   +-- Сериализация JS state (только user data, не DOM/V8 internals)
  |   +-- Запись в SQLite: app_id, checkpoint_blob, timestamp
  |   +-- Размер checkpoint: типично 10–500 KB (состояние форм, позиция скролла, открытые документы)
  |
  +-- При сбое Isolate:
      +-- Host Shim обнаруживает crash (segfault, OOM kill)
      +-- Micro-Kernel читает последний checkpoint из SQLite
      +-- Создаётся новый Isolate с тем же app_id
      +-- JS state восстанавливается из checkpoint
      +-- Display Server показывает Static UI Overlay + progress: «Восстановление...» (< 100 мс)
      +-- Пользователь продолжает работу с того же места
```

**Что НЕ сохраняется в checkpoint:**
- WebGPU textures / buffers (пересоздаются)
- Network connections (переподключаются)
- Таймеры / анимации (сбрасываются)

**Что сохраняется:**
- Значения форм, текстовых полей
- Позиция скролла, открытые вкладки
- Выделенные элементы, фильтры поиска
- Несохранённые изменения документа (autosave → checkpoint)

**Где:** Level 1 (Micro-Kernel, Checkpoint Manager) + Level 2 (SQLite) + Level 0 (Host Shim, crash detection).

#### 4.3.2 Native Process Monitor

**Что:** Механизм принудительного завершения нативных процессов (level 4 приложения в Game Mode, нативные модули), которые потребляют чрезмерные ресурсы или зависли вне V8 Isolate.

**Проблема:** Warm Recovery работает только для V8 Isolates. Нативное приложение (level 4, direct GPU context) может:
- Утечь памяти в native heap (вне контроля V8 heap limit)
- Зависнуть в бесконечном цикле на CPU (SCHED_RR не спасает от логических deadlock)
- Заблокировать GPU командой (например, infinite loop в shader)

**Как:**

```
Host Shim мониторит нативные процессы:
  |
  +-- Memory watchdog:
  |   +-- Процесс потребил >85% доступной RAM хост-ОС
  |   +-- Процесс потребил >95% RAM, выделенной CORE
  |   +-- Action: graceful suspend → сохранение checkpoint → kill process
  |
  +-- CPU watchdog:
  |   +-- Процесс не отвечает на heartbeat 500 мс (в Game Mode: 50 мс)
  |   +-- CPU usage >95% на одном ядре >5 сек
  |   +-- Action: kill process + Static UI Overlay «Приложение не отвечает»
  |
  +-- GPU watchdog:
  |   +-- GPU fence не возвращается >2 сек (deadlock в драйвере)
  |   +-- Action: GPU reset (TDR на Windows, аналог на других платформах) + recovery
  |
  +-- User-initiated kill:
      +-- Command Bar → «задачи» → список процессов с RAM/CPU → «завершить»
      +-- Panic Gesture (тройное касание) → мгновенный kill активного приложения
```

**Восстановление после kill:**
- Если есть checkpoint (Game Mode приложения обязаны писать checkpoint каждые 5 сек) → Warm Recovery из checkpoint
- Если нет checkpoint → приложение запускается заново, пользователь теряет несохранённое состояние

**Безопасность:**
- Kill выполняется Host Shim на уровне ОС (SIGKILL / TerminateProcess), не через API CORE
- После kill — GPU memory zeroize (очистка framebuffer и textures перед передачей другому приложению)
- Нативный процесс не может перехватить watchdog — heartbeat проверяется Host Shim извне

**Где:** Level 0 (Host Shim, OS-level process management) + Level 1 (Micro-Kernel, policy engine).

### 4.4 Sandboxing

**Capability-based Security:**

```typescript
const context = {
  fs: {
    read: ["/project/abc/**"],
    write: ["/project/abc/**"],
  },
  network: {
    domains: ["api.example.com"],
  },
  graphics: true,
  mind: true,
  contacts: false,
};
```

**Где:** Level 1 (Micro-Kernel, Capability Security).

> Подробно: seccomp-профили, namespace-конфиги, V8 Sandbox parameters, firewall-правила localhost — в [Layer Security, §22](layer-7-security.md).

### 4.5 Permissions UI

**Что:** Интерфейс для просмотра и управления разрешениями приложений. Пользователь видит, какие capabilities выданы каждому приложению, и может их отозвать.

**Как:**

```
Permissions UI (Level 3, Display Server):
  |
  +-- Список приложений (Command Bar / контекстное меню окна)
  +-- Для каждого приложения:
      +-- capabilities: fs, network, graphics, mind, contacts, notifications, microphone, camera
      +-- toggle: разрешено / запрещено
      +-- fs: детализация — какие папки/файлы (scope)
      +-- network: детализация — какие домены (whitelist)

Запрос нового разрешения:
  |
  +-- Приложение вызывает API, которого нет в его capability-контексте
  +-- Micro-Kernel (Level 1) перехватывает → отправляет запрос в Permissions UI
  +-- Display Server показывает модальное окно с запросом
  +-- Пользователь: "Разрешить" / "Запретить" / "Разрешить один раз"
  +-- Решение записывается в SQLite (profile_apps) → обновляется capability-контекст Isolate
```

**Где:** Level 1 (Capability Security, SQLite) + Level 3 (Display Server, модальные окна) + Level 4 (Intent API, голосовое управление правами).

### 4.12 Security Hooks (App Lifecycle)

**Hook points:**

```rust
enum SecurityHook {
    BeforeIsolateCreate { app_id, level, manifest },
    AfterIsolateCreate { isolate_id, pid },
    BeforeApiCall { isolate_id, api, args },
    AfterApiCall { isolate_id, api, result },
    BeforeNetworkRequest { isolate_id, domain, port },
    OnIsolateFreeze { isolate_id },
    OnIsolateThaw { isolate_id },
    OnIsolateDestroy { isolate_id },
    OnProfileSwitch { from_profile, to_profile },
}
```

**Default handler:**
- Проверка domain whitelist перед сетевым запросом (`BeforeNetworkRequest`)
- Secure wipe профиля при переключении (`OnProfileSwitch`)
- Проверка capabilities перед API-вызовом (`BeforeApiCall`)

**Где:** Level 1 (Micro-Kernel, Security Hook subsystem). Интерфейс — Rust trait `SecurityHookHandler`.

> Подробно: seccomp-профили, namespace-конфиги, V8 Sandbox parameters — в [Layer Security, §22](layer-7-security.md).

### 4.13 Session Management

**TTL и auto-lock:**

```rust
struct SessionConfig {
    ttl_minutes: u32,           // 30 по умолчанию
    auto_lock_after_idle: u32,  // 5 минут простоя
    require_biometry: bool,     // true для уровня «Повышенный»
}
```

- Проверка каждую минуту: idle > auto_lock → lock screen; total > ttl → logout
- Owner может принудительно завершить сессию устройства через Core.Hardcore или другой Фронт

**Remote wipe:**
- Owner инициирует remote wipe → Бэк отзывает session token → push-уведомление Фронту → lock screen + очистка кэша
- Шифрование кэша Фронта ключом device key + screen lock

**Где:** Level 1 (Micro-Kernel, Session Manager) + Level 0 (Host Shim, screen lock / idle detection).

### 4.14 Accessibility и защита от screen readers

- **Accessibility API доступен только для системных приложений.** Сторонние приложения (уровни 1–5) изолированы — screen reader не может читать их окна
- **Изоляция per Space:** screen reader, запущенный в Space A, не видит окна Space B
- **Secure fields:** поля ввода паролей и recovery-фраз блокируют accessibility API даже для системных приложений

**Где:** Level 3 (Display Server, accessibility API gating) + Level 0 (Host Shim, platform accessibility bridge).

---

## Display Server (Level 3)

**Что:** Композитинг и рендеринг всего UI через WebGPU. Никакого DOM, никакого Chromium для системного UI.

**Компоненты:**

```
WebGPU Pipeline (Level 3):
  |
  +-- Surface: swapchain для каждого монитора
  +-- Render Pass: фон Space → окна приложений → эффекты → курсор
  +-- Effects: blur, transparency, shadows, rounded corners (шейдеры)
  +-- Text Rendering: glyph rasterization через font atlas
  |
  v
  Command Encoder -> Queue -> GPU -> Экран
```

**Композитинг окон:**
- Каждое окно = текстура (или direct rendering для WebGPU-приложений)
- Z-сортировка Window Manager → порядок отрисовки
- Dirty rectangles: перерисовываем только изменённые области

**Shadow State Recovery:**
- Если Shell падает → Display Server продолжает рендерить последний кадр (статичный)
- После перезапуска Shell восстанавливает layout из SQLite + CRDT
- Пользователь видит «фриз» на 100–300мс, а не чёрный экран

**Theme Engine / Design System:**
- `@core/ui` — библиотека компонентов, адаптирующаяся под текущую тему
- Тема = набор цветов, шрифтов, форм, анимаций
- Компоненты перерисовываются автоматически при смене темы
- Адаптация под форм-фактор: десктоп (мышь) vs мобильный (тач)

**Где:** Level 3 (WebGPU Pipeline) + Level 0 (Host Shim для system calls).

---

### 4.6 Scheduler

**Что:** Планировщик задач, напоминаний и фоновых операций.

**Как:**

```
Scheduler (Level 1):
  |
  +-- Таймеры: напоминания, будильники, cron-подобные задачи
  +-- Фоновые задачи: индексация Search Engine, prefetch файлов, проверка обновлений
  +-- Приоритеты: user-visible (напоминания) > background (индексация)
  +-- Энергосбережение: при <20% батареи — background задачи только по Wi-Fi
```

**Где:** Level 1 (Micro-Kernel, SQLite).

---

### 4.7 Update Engine

**Что:** Проверка, скачивание и установка обновлений приложений.

**Как:**

```
Update Engine (Level 1):
  |
  +-- Проверка обновлений: по расписанию (Scheduler) или вручную
  +-- Формат пакета: tar.zst, подпись Ed25519, хеш BLAKE3
  +-- Проверка: подпись + хеш + min_core_version
  +-- Установка: atomic replace, rollback при ошибке
  +-- App Registry обновляет каталог
```

**Где:** Level 1 (App Registry + Scheduler).

---

### 4.8 Error Reporting Engine

**Что:** Сбор и отправка отчётов об ошибках.

**Как:**

```
Error Reporting Engine (Level 1):
  |
  +-- Белый список полей: error.code, error.message (опционально), stack trace
  +-- Rate limiting: max 3 отчёта в час
  +-- Preview: пользователь видит, что отправляется, перед отправкой
  +-- HTTPS only, mandatory TLS pinning
  +-- Контроль отправки error.message: пользователь может отказаться
```

**Где:** Level 1 (Micro-Kernel) + Level 2 (P2P / HTTPS).

---

### 4.9 core-dev (Developer CLI)

**Что:** CLI-инструмент разработчика приложений для Workspace.

**Команды:**
- `core-dev install` — установка зависимостей `@core/*`
- `core-dev run` — запуск приложения в dev-режиме (hot reload)
- `core-dev build` — сборка в V8 Bytecode (`bun build --bytecode`)
- `core-dev validate` — проверка манифеста `core.json`
- `core-dev publish` — публикация в App Registry (pkg.core.app)
- `core-dev check-updates` / `core-dev update` / `core-dev rollback`

**Где:** Опциональный компонент Level 1 (Micro-Kernel).

---

### 4.10 @core/mock

**Что:** npm-пакет для тестирования приложений уровней 4–5 вне Workspace.

**Как:**
- In-memory реализации `@core/*` API (`@core/db`, `@core/fs`, `@core/network` и др.)
- Позволяет разрабатывать и тестировать приложение в обычном браузере/Node.js
- Не требует запуска Workspace

**Где:** Dev-dependency, вне Level-архитектуры.

---

### 4.11 Window Injection & Env Injection

**Window Injection (`window.__WORKSPACE__`):**
- Передача конфигурации в Island Mode (Chromium sandbox) до загрузки страницы
- Данные: версия CORE, доступные API, тема, локаль
- Приложение определяет среду выполнения и адаптируется

**Env Injection (`process.env.WORKSPACE*`):**
- Передача переменных окружения в V8 Isolate
- `CORE_OS_DB_PATH` — путь к app-scoped SQLite
- `CORE_OS_VFS_PATH` — путь к app-scoped VFS
- `CORE_OS_PROFILE_ID` — ID текущего профиля
- `CORE_OS_SPACE_ID` — ID текущего Space

**Где:** Level 2 (App Runtime) + Level 3 (Island Mode для Window Injection).

---

## 5. Search & Tag Engine

### 5.1 Полнотекстовый поиск (Deep Indexing)

**Что:** Поиск по файлам, заметкам, задачам, истории, переписке.

**Как:**

```
Индексация (фоновый процесс, Level 1):
  |
  +-- Файлы: извлечение текста (PDF, DOCX, TXT)
  +-- Изображения: OCR (распознавание текста на скриншотах)
  +-- Заметки, задачи: прямой индекс
  +-- Переписка: индекс сообщений
  +-- История действий: что открывал, что искал
  |
  v
FTS5 (SQLite Full-Text Search):
  |
  +-- Токенизация: русская + английская
  +-- Стемминг: "строительство" -> "строительств"
  +-- Ранжирование: BM25
  |
  v
Результаты -> Suggestion Engine -> выпадающий список
```

**Контекстный поиск:**
- Открыт проект "ЖК Скандинавия" -> результаты из этого проекта приоритетнее
- Текущий тег -> файлы с этим тегом приоритетнее

**Где:** Level 1 (Bun, SQLite FTS5, фоновые workers).

### 5.2 Теги

**Что:** Один файл — много тегов — много точек входа. Нет иерархии папок.

**Как:**

```
Tag Store (SQLite):
  |
  +-- tags: id, name, color, project_id
  +-- file_tags: file_id, tag_id
  +-- note_tags: note_id, tag_id
  +-- task_tags: task_id, tag_id
  |
  v
Любая сущность (файл, заметка, задача) может иметь N тегов
  |
  v
В проекте "ЖК Скандинавия":
  файл "смета.pdf" виден через тег #строительство
  |
В проекте "Бюджет семьи":
  тот же файл виден через тег #бюджет
  |
Это один и тот же файл (один CID), видимый из разных контекстов
```

**Smart Folders:**
- Папка = сохранённый поисковый запрос (live filter)
- "Все PDF от бухгалтерии за март" -> `type:pdf AND from:бухгалтерия AND date:march`
- Автоматически обновляется при появлении новых файлов

**Где:** Level 1 (SQLite) + Level 2 (CRDT для синхронизации тегов).

### 5.3 Граф связей

**Что:** Всё связано со всем. Задача -> файл -> письмо -> контакт.

**Как:**

```
Graph Store (SQLite):
  |
  +-- nodes: id, type (file|note|task|contact|email|tag), data
  +-- edges: from_id, to_id, relation_type
  |
  Пример графа:
  |
  Задача "Согласовать фундамент"
    +--> Файл "договор_подряда.pdf" (reference)
    |     +--> Email "Правки от заказчика" (attachment)
    |     +--> Контакт "Алексей" (автор)
    +--> Идея "Использовать другой бетон" (related)
```

**Навигация по графу:**
- В любом приложении: "Показать связи" -> дерево связанных сущностей
- Переход по клику -> открывается соответствующее приложение/контекст

**Где:** Level 1 (SQLite + графовые запросы).
---

## 6. Communication Layer

### 6.1 Мессенджер

**Что:** Чаты внутри CORE. `@ivan` -> открывает чат.

**Как:**

```
@ivan в строке
  |
  v
Contact Book (SQLite):
  +-- Поиск по имени/нику/номеру
  +-- Результат: контакт найден
  |
  v
Выпадающий список (всегда, даже при одном совпадении):
  +-- Контакты с совпадением (имя, платформы: Telegram/Slack/почта)
  +-- Последние сообщения рядом с каждым контактом
  +-- Стрелками выбираешь → Enter
  |
  v
Выбор → полное окно чата в проекте
  |
  v
Отправка сообщения:
  +-- CORE-контакт -> P2P, CRDT, мгновенно
  +-- Внешний контакт -> bridge (Telegram/WhatsApp API, email)
  +-- Шифрование: end-to-end через WireGuard tunnel
```

### 6.2 Почта

```
email@domain в строке
  |
  v
Email Engine (Level 1):
  +-- Форма написания письма (в строке или в окне)
  +-- SMTP отправка
  +-- IMAP/POP3 получение
  +-- Индексация входящих -> Search Engine
  |
  v
"проверь почту" -> непрочитанные в выпадающем списке строки
"что писал Алексей" -> Search Engine по индексу почты
```

### 6.3 Звонки (VoIP)

```
+7999... в строке
  |
  v
Contact Book -> контакт по номеру
  |
  v
UI строки:
  +-- Основное действие: открыть чат (мессенджер по умолчанию)
  +-- Опция: "Позвонить" (кнопка рядом)
  |
  v
Звонок:
  +-- CORE user -> P2P VoIP (WebRTC через WireGuard)
  +-- Внешний -> SIP bridge / GSM (через провайдера)
  +-- Аудио -> Core.Audio (Level 0, Host Shim)
```

### 6.4 ИИ-мост ("скинь документ Ивану")

```
"скинь документ Ивану" в строке
  |
  v
Intent API (Level 4):
  +-- Парсит: действие="отправить", объект="документ", получатель="Иван"
  +-- Search Engine -> находит последний документ в текущем проекте
  +-- Contact Book -> находит Ивана
  +-- Спрашивает подтверждение: "Отправить 'смета.pdf' Ивану?"
  +-- Подтверждено -> Chat Engine -> отправка
```

**Где:** Level 1 (Chat Engine, Email Engine) + Level 2 (P2P) + Level 4 (Intent API).

### 6.5 Техподдержка (Remote Support)

**Архитектура:**

```
Пользователь: "техподдержка" (голос / Command Bar)
  |
  v
Бэк открывает окно приёма (TTL 10 мин) + генерирует код: SUPPORT-9284-6173
  |
  v
Пользователь диктует код специалисту
  |
  v
Relay-сервер CORE Corp (NAT traversal) -> WireGuard-туннель до Бэка
  |
  v
Бэк проверяет: код валиден? подпись CORE Corp (Ed25519, HSM) верна?
  |
  v
На экране пользователя: "Подтверждённый специалист. Разрешить?"
  |
  v
Сессия открыта (TTL 30 мин). Все действия логируются в аудит (категория «system»)
```

**Security controls:**
- **Dual approval** для уровня доступа Full (требуется второе подтверждение)
- **Видеозапись** сессий уровня Full — обязательна
- **HSM key ceremony:** подпись запросов техподдержки через HSM CORE Corp (ключ никогда не покидает железо)
- **Anomaly detection:** если техподдержка обращается к одному Бэку > N раз — alert Owner
- **Rate limiting:** максимум 3 попытки подключения с одного IP, потом блокировка 30 мин

**Где:** Level 0 (Host Shim, SSH-сервер) + Level 1 (Key Manager, Auth Proxy) + Level 2 (Relay-сервер).

---

## 7. Voice Engine

### 7.1 Whisper (локальный)

**Что:** Фоновое распознавание речи. Работает всегда, даже в играх.

**Как:**

```
Микрофон (Level 0, Host Shim, cpal)
  |
  v
Audio Stream -> Whisper Model (Level 4)
  |
  +-- Работает на отдельном ядре процессора (Core Pinning, Level 0)
  +-- Или на NPU (если доступна)
  +-- Модель: Whisper small/medium (зависит от железа)
  +-- Результат: текст + confidence
  |
  v
Wake word detection (опционально):
  +-- "CORE" / "Компьютер" / настраиваемый
  +-- Или всегда слушает (privacy: локально, данные не уходят)
  |
  v
Текст -> Input Router -> дальше как клавиатурный ввод
```

**Exclusive Mode (игры):**
- Whisper на отдельном ядре -> 0% влияние на игру
- Распознаёт команды даже при громком звуке игры
- Результаты -> только в наушники (TTS) или overlay

**Где:** Level 0 (аудио через Host Shim) + Level 4 (Whisper model).

### 7.2 TTS Engine

**Что:** Локальный синтез речи. Результаты голосовых команд озвучиваются в наушники.

**Как:**
- Модель: Piper / Coqui
- Задержка: <100 мс
- Опциональный облачный fallback
- Работает в Zero UI (игры, без экрана) и в обычном режиме

### 7.3 Zero UI

**Что:** Голос работает без экрана. Результаты — в наушники или как действия.

**Как:**

```
Голосовая команда в Exclusive Mode (игра/видео):
  |
  +-- "Сделай музыку тише" -> Core.Audio.setVolume(0.5)
  +-- "Поставь будильник на 7" -> Scheduler.setAlarm(7:00)
  +-- "Скинь кадр Ване" -> Screenshot -> Chat -> отправка
  +-- "Что по проекту?" -> ИИ сводка -> TTS в наушники
  |
  Никаких окон, никаких отвлечений от основного экрана
```

**Где:** Level 4 (Intent API) + Level 0 (Audio, Host Shim).

### 7.3.1 Intent Queue

**Что:** Очередь намерений пользователя, которая сохраняет действия даже при неготовности V8 Isolate или намеренном выключении UI.

**Как:**

```
Пользователь нажимает «Сохранить» (CPU задушен до 400 МГц):
  |
  v
Intent Queue принимает Intent = "saveProject(project_id=42)"
  |
  v
Вместо мгновенного выполнения:
  +-- ACK: «Принято» (Static UI Overlay показывает checkmark)
  +-- Подписка: как только CPU разгонится — Intent исполнится
  +-- Если Intent > 5 сек — бэк-офис показывает progress bar
  +-- Если Intent невозможен (нет прав, нет сети) — TTS: "Не удалось сохранить, нужен интернет"
```

**Почему это важно:** Без очереди пользователь при задушенном CPU видит «фриз» и думает, что система сломалась. С очередью — видит «Принято» и доверяет системе.

**Где:** Level 1 (UX) + Level 2 (Бэк, V8 Isolate queue) + Level 3 (Display Server, overlay).

### 7.4 Безопасность голосового ввода

- **Аппаратный индикатор записи (LED):** Host Shim активирует LED устройства при захвате микрофона. Пользователь видит, что CORE слушает.
- **Аудио-буфер обнуляется** после распознавания — сырые audio samples не сохраняются на диск и не передаются
- **Integrity check Whisper model:** при загрузке модели проверяется BLAKE3-хеш. Подмена модели → отказ загрузки
- **Opt-out:** пользователь может полностью отключить микрофон в настройках (Settings → Privacy → Voice Input)
- **Speaker identity (опционально):** локальная проверка голоса для sensitive-команд (требует PIN/биометрии)

**Где:** Level 0 (Host Shim, LED + audio buffer) + Level 4 (Whisper integrity check).

---

## Intent API (Level 4)

**Что:** Программный интерфейс для регистрации и вызова Intents. Позволяет приложениям и системе понимать намерения пользователя.

### 7.5 Intent Parser

**Что:** Текст (из Command Bar или Whisper) → типизированная команда (Intent).

**Как:**

```
"Запиши: купить краску для ванной"
  |
  v
Intent Parser (Level 4):
  +-- Base Tier (всегда работает): rule-based + lightweight NLP
      +-- Словари Intents: "открой", "найди", "переключи", "создай"
      +-- FTS5 поиск по SQLite (имена файлов, теги, контакты)
      +-- 16 MB RAM, не требует GPU
  +-- Full Tier (Core.Mind включён): Semantic Kernel + векторный поиск
      +-- Embeddings (nomic-embed-text/bge-m3) → векторный индекс
      +-- Context Enricher: обогащает запрос данными из индекса
  |
  v
Intent Map -> находит зарегистрированный Intent
```

### 7.6 Intent Resolver

**Что:** Сопоставляет распознанный Intent с приложением или системной командой.

**Как:**
- Сопоставление с зарегистрированными Intents приложений (`os.mind.registerIntent`)
- Системные Intents: яркость, громкость, переключение профиля, скриншот
- Неоднозначность: если 2 приложения с Intent `open_terminal` → выбор по приоритету (последнее использование, владелец проекта) или уточнение в Command Bar

### 7.7 Action Executor

**Что:** Выполняет Intent: вызывает приложение, системную команду, генерирует UI или обращается к Cloud Bridge.

**Как:**
- **Intent API вызов:** `Core.Notes.create_note({ content, project })`
- **Системный вызов:** изменение настроек, управление окнами
- **Generative UI:** если готового приложения нет → генерация JS-виджета в Level 5 sandbox
- **Cloud Bridge:** прокси-мост к облачным LLM (OpenAI, Anthropic, GLM, Kimi, Gemini) через OpenAI-compatible API

### 7.8 Response Formatter

**Что:** Формирует ответ пользователю.

**Как:**
- Текст в Command Bar
- TTS (Piper/Coqui) — локальный синтез речи
- UI-виджет (временный, через `@core/graphics`)
- Безмолвное действие (выполнено, ничего не показано)

### 7.9 TTS Engine

**Что:** Локальный синтез речи.

**Как:**
- Модель: Piper / Coqui
- Задержка: <100 мс
- Опциональный облачный fallback

### 7.10 Generative UI Engine

**Что:** Генерация JS-кода визуализации на лету.

**Как:**
- ИИ собирает данные из Semantic Kernel
- Пишет JS-код в Level 5 sandbox
- Отрисовывает временный виджет через `@core/graphics`
- Пример: «Покажи график трат на кофе за год» → сгенерированный виджет

### 7.11 Cloud Bridge

**Что:** Прокси-мост к облачным LLM-провайдерам.

**Как:**
- OpenAI-compatible API (OpenAI, Anthropic, GLM, Kimi, Gemini)
- Explicit consent per request (по умолчанию)
- Prompt filtering: DLP, удаление чувствительных данных перед отправкой
- Аудит всех облачных запросов

### 7.12 Smart Scheduler

**Что:** Динамическое распределение AI-нагрузки между GPU/NPU/CPU.

**Как:**
- Мониторинг загрузки системы
- Если запущена игра → снижение приоритета Whisper/SLM
- Переключение между CPU и NPU в зависимости от доступности

### 7.13 AI Event Bus

**Что:** Внутренняя шина событий AI-конвейера.

**События:**
- `ai.speech.recognized` — Whisper завершил распознавание
- `ai.intent.resolved` — Intent Parser определил намерение
- `ai.action.executed` — Action Executor выполнил команду
- `ai.cloud.requested` — запрос ушёл в Cloud Bridge

### 7.14 Pipeline Hooks

**Точки расширения для разработчиков:**
- `preIntentParse` — модификация текста перед парсингом
- `postIntentResolve` — обработка после определения Intent
- `preActionExecute` — проверка перед выполнением

### 7.15 Model Sandbox

**Что:** Изоляция AI-процессов.

**Как:**
- Whisper, SLM, TTS, Ollama — каждый в отдельном процессе или V8 Isolate
- Нет прямого доступа к VFS, SQLite, сети (кроме Cloud Bridge через прокси)
- Доступ только через `@core/*` wrappers

**Где:** Level 4 (Intent API, AI Engine) + Level 2 (Ollama/llama.cpp) + Level 0 (Host Shim для GPU/NPU).

---

## 8. Profile Manager

### 8.1 Переключение профилей

**Что:** Мгновенное переключение между контекстами (работа/личное/фриланс).

**Как:**

```
Profile Store (SQLite):
  |
  +-- profiles: id, name, icon, settings, created_at
  +-- profile_projects: profile_id, project_id
  +-- profile_apps: profile_id, app_id, permissions
  |
  v
Переключение:
  |
  +-- Текущий профиль -> заморозка (см. 8.2)
  +-- Новый профиль -> загрузка state
  |   +-- Project Manager загружает проекты профиля
  |   +-- App Runtime загружает layout последнего проекта
  |   +-- Display Server рендерит
  +-- Время переключения: < 500ms (цель)
```

**Где:** Level 1 (Project Manager + SQLite).

### 8.2 Заморозка неактивного профиля (Freeze/Thaw)

**Что:** Неактивный профиль не потребляет CPU, но фоновые данные подтягиваются.

**Как:**

```
Заморозка:
  |
  +-- Все V8 Isolates профиля -> V8::TerminateExecution()
  +-- Layout state -> сериализуется в SQLite
  +-- App states -> сериализуются в CRDT
  +-- CPU = 0 для всех процессов профиля
  +-- SharedArrayBuffer region профиля -> zeroize + unmap (изоляция от других профилей)
  |
  v
Фоновые данные (background):
  |
  +-- Sync Engine продолжает работать (Level 2)
  +-- Новые сообщения -> складываются в очередь (SQLite)
  +-- Push-уведомления -> минимальный обработчик (Level 1)
  +-- При разморозке -> все накопленные данные доступны
  |
  v
Разморозка:
  |
  +-- V8 Isolates пересоздаются
  +-- States восстанавливаются из CRDT
  +-- Display Server рендерит последний layout
```

**Где:** Level 1 (Micro-Kernel) + Level 2 (Sync Engine).

### 8.3 Анонимный профиль

**Что:** Временный профиль. Закрыл — все данные стёрлись.

**Как:**

```
Создание:
  +-- Флаг: anonymous = true
  +-- Хранилище: RAM only (in-memory SQLite)
  +-- Нет CRDT-синхронизации (данные не покидают устройство)

Закрытие:
  +-- Все Isolates уничтожены
  +-- In-memory SQLite -> drop
  +-- Кэш -> очистка
  +-- RAM освобождена полностью
```

**Где:** Level 1 (Micro-Kernel).

---

### 8.4 Мультипользовательская аутентификация и изоляция

**Device key — один на устройство:**

- Генерируется при первом запуске Workspace. Хранится в TPM/Secure Enclave (или зашифрован в keychain при отсутствии TPM).
- Назначение: WireGuard handshake, device authentication при подключении к Бэку, шифрование profile keys.
- Не привязан к пользователю. При краже устройства — device key не даёт доступа к данным профилей без дополнительных секретов.

**Profile key — один на профиль:**

- Источник: BIP-39 recovery phrase (24 слова) → BLAKE3 → 256-bit profile key.
- Хранение: `encrypted_profile_key = AES-256-GCM(profile_key, device_key || user_secret)`.
  - `user_secret`: biometric template (TPM-backed) или PIN-код.
  - При уровне «Базовый»: `user_secret` пустой, шифрование только device key.
- Локация: OS keychain (Windows DPAPI, macOS Keychain, Linux libsecret/Keyring).
- Назначение: расшифровка локального state профиля (VFS metadata, CRDT snapshot, app-scoped SQLite), подпись запросов на session token.

**Как пользователь входит в свой профиль:**

1. На экране входа — список профилей устройства («Папа», «Мама», «Анонимный»).
2. Выбор профиля → если уровень безопасности «Повышенный» или «Максимальный» — запрос биометрии/PIN.
3. Key Manager расшифровывает profile key: `decrypt(encrypted_profile_key, device_key + биометрия/PIN)`.
4. Фронт запрашивает у Бэка session token для этого профиля. Запрос подписан profile key + device key.
5. Бэк проверяет: device key известен? profile key валиден для данного profile_id? → выдаёт session token.
6. Фронт загружает layout, проекты, приложения профиля.

**Session token:**

- Формат: JWT-подобный, подписан Ed25519 приватным ключом Бэка.
- Payload: `profile_id`, `device_key_fingerprint` (BLAKE3 публичного ключа), `issued_at`, `expires_at` (default 24h).
- Передача: в заголовке каждого API-запроса (`X-Core-Session`).
- Валидация на Бэке: подпись валидна? `device_key_fingerprint` известен? `profile_id` существует? token не в revoke-листе?
- При переключении профиля: старый token сбрасывается (discard на Фронте), новый запрашивается после загрузки профиля.

**Как Бэк различает профили:**

- Каждый профиль — отдельный session token. Бэк хранит Profile Store: каждый профиль изолирован (свой VFS, app-scoped SQLite, CRDT-журнал, аудит-лог).
- Запрос от Фронта всегда содержит session token. Бэк не отдаст данные чужого профиля, даже если запрос пришёл с того же device key.
- Device key = «это доверенное устройство». Session token = «это конкретный пользователь на этом устройстве».

**Device key mapping:**

- TPM/Secure Enclave: один root device key на устройство. Не покидает железо.
- Key Manager: для каждого профиля derived encryption key (из recovery-фразы профиля).
- Profile key зашифрован на устройстве. При краже устройства без биометрии/PIN profile key недоступен.
- При уровне «Базовый»: device key открывает канал, но profile key тоже зашифрован только device key. Риск: если TPM взломан — данные профилей доступны.

**Profile Store изоляция на Бэке:**

- `profiles` таблица (SQLite Бэка): `id`, `name`, `device_id`, `recovery_hash`, `created_at`.
- Per-profile данные:
  - VFS: отдельная директория `/vfs/profiles/{profile_id}/`
  - App-scoped data: `/vfs/apps/{app_id}/profiles/{profile_id}/`
  - CRDT-журнал: отдельный Merkle Search Tree per profile
  - Аудит-лог: отдельная таблица `audit_log_{profile_id}`
- Бэк использует `profile_id` из session token для маршрутизации всех запросов. Запрос без валидного token → 401.

**Переключение профилей (detailed flow):**

1. **Триггер:** Пользователь выбирает профиль в UI (Display Server показывает список профилей с аватарами).
2. **Freeze текущего:** Profile Manager вызывает `FreezeProfile(current_profile_id)`:
   - V8 Isolates → `TerminateExecution()` (§8.2).
   - Layout state → сериализуется в SQLite.
   - App states → сериализуются в CRDT.
   - Clipboard → clear().
   - SharedArrayBuffer region текущего профиля → zeroize + unmap.
3. **Аутентификация нового:** Profile Manager вызывает `LoadProfile(target_profile_id)`:
   - Если уровень безопасности > «Базовый»: запрос биометрии/PIN через Host Shim → OS biometric API.
   - Key Manager: `profile_key = decrypt(encrypted_profile_key, device_key + user_secret)`.
   - Локальный state профиля расшифровывается: VFS metadata, CRDT snapshot из SQLite.
4. **Session handshake:** Фронт отправляет Бэку `SessionRequest`:
   - Подпись: `sign(profile_key + timestamp, device_key)` (Ed25519).
   - Payload: `profile_id`, `device_key_fingerprint`, `timestamp`.
   - Бэк валидирует подпись, проверяет `recovery_hash` профиля, выдаёт session token.
5. **Загрузка state:** Profile Manager загружает проекты, приложения, layout. Display Server рендерит.
6. **Время переключения:** цель < 500ms (биометрия может добавить 200–400ms).

**Security considerations:**

- **Cold boot attack:** master key в RAM при активном профиле. При lock/switch — profile key выгружается из RAM (explicit zeroize).
- **Side-channel:** SharedArrayBuffer изолируется per-profile (новый region при каждом переключении).
- **Brute-force PIN:** rate limiting на биометрию/PIN через OS API. После N неудач — профиль блокируется, требуется recovery-фраза.
- **Revocation:** Owner может отозвать профиль с устройства через Бэк. При следующем переключении — Profile Manager получает `ProfileRevoked`, локальные данные профиля шифруются и помечаются для удаления.

---

## 9. Sync Engine

### 9.1 P2P-меш

**Что:** Устройства объединяются в частную сеть.

**Как:**

```
Mesh Engine (Level 2):
  |
  +-- WireGuard: зашифрованные туннели между устройствами
  |   +-- Каждое устройство = постоянный внутренний IP
  |   +-- NAT traversal (Tailscale-подобный механизм)
  |
  +-- libp2p: поиск узлов в децентрализованной сети
  |   +-- DHT для адресации контента
  |   +-- Gossip протоколы для распространения данных
  |
  +-- mDNS: обнаружение устройств в локальной сети
  |   +-- Без интернета — прямая передача по Wi-Fi
  |
  +-- Core Base (опционально):
      +-- Всегда включённый узел (коробочка в розетку)
```

**Где:** Level 2 (Mesh Engine) + Level 0 (WireGuard, Host Shim).

### 9.1.1 Verified Content Seeding (Public P2P CDN)

**Что:** Опциональный режим, когда пользователь становится seed'ом для публичного контента (приложения, обновления, verified media). Не BitTorrent — только verified publisher content.

**Не путать с private P2P:** Private mesh — это sync между устройствами одного пользователя. Public seeding — раздача контента другим пользователям через P2P.

**Как:**

```
Publisher (App Registry):
  |
  +-- Загружает пакет в App Registry
  +-- Пакет подписан Ed25519 (publisher key)
  +-- CID = BLAKE3(content) — неизменяемый идентификатор
  +-- Metadata подписана отдельно (name, description, version, dependencies)
  |
  v
App Registry объявляет пакет "public"
  |
  v
User A включает "Public Seeding" (opt-in):
  +-- Система кэширует пакет локально
  +-- User A регистрируется в DHT как seed для этого CID
  +-- Другие пользователи могут найти User A через libp2p DHT
  |
  v
User B запрашивает пакет:
  +-- Mesh Engine ищет CID в DHT → находит User A (и других seeds)
  +-- Handshake: ECDH ephemeral keys → encrypted connection
  +-- User B скачивает пакет по chunks (1 MB each)
  +-- Каждый chunk проверяется по BLAKE3 →CID
  +-- Metadata проверяется по publisher signature
  |
  v
User B тоже может стать seed'ом (опционально)
```

**Защита:**
- **Verified content only:** seed разрешён только для пакетов, подписанных App Registry. Самоподписанные / неподписанные пакеты не seed'ятся
- **CID integrity:** любой chunk проверяется по BLAKE3. Подмена → mismatch → отбрасывание
- **Metadata signing:** имя, описание, иконка подписаны publisher key. Mesh Engine отбрасывает пакеты с невалидной metadata signature
- **IP privacy:** handshake через relay (onion routing). Прямой P2P — только после ECDH ephemeral key exchange

**Ограничения:**
- **Opt-in:** пользователь явно включает seeding в Settings. По умолчанию — выключено
- **Bandwidth cap:** max 10% upload bandwidth для public seed. Настраивается пользователем
- **Storage cap:** max 5 GB кэша для public seed. LRU eviction
- **Connection limit:** max 50 одновременных peers на файл
- **Content filter:** только categories из whitelist (apps, updates, verified media). Не user data, не personal files

**Экономика:**
- Seeding не оплачивается — это добровольная поддержка экосистемы
- Publisher может стимулировать: "стать seed'ом → скидка на premium"
- Для корпоративных: mandatory seeding внутри LAN (офисные машины раздают корпоративные пакеты)

**Где:** Level 2 (Mesh Engine, libp2p DHT) + Level 1 (App Registry, publisher verification) + Level 0 (Host Shim, bandwidth monitor).

> Подробно: verified content signing, IP privacy relay, bandwidth caps — в [Layer Security, §22](layer-7-security.md).

**Erasure Coding (FEC):**
- Избыточное кодирование в P2P-поток (как в спутниковой связи)
- Приёмник восстанавливает данные при потере до 30% пакетов без ретрансмита
- Нагрузка на канал: 95% (бесконечные повторы) → 40% (FEC + избыточность)
- Особенно критично для мобильных сетей и "микроволновок"

**TCP Relay (NAT Traversal Fallback):**
- Если UDP заблокирован (firewall, провайдер) → fallback на TCP relay
- Потеря скорости: ~20% по сравнению с UDP
- Relay-серверы CORE Corp (опционально) или корпоративные Supernodes
- Автоматическое переключение UDP → TCP при обнаружении блокировки

**Dark-Mesh / Протокол Омега:**
- План "Б" при попытке блокировки P2P-трафика
- Обфускация: трафик маскируется под обычный HTTPS-шум или видеозвонки (WebRTC-стеганография)
- Активация: автоматически (при обнаружении блокировки) или вручную (Owner)
- Неотличим от легитимного трафика для DPI (Deep Packet Inspection)
- Небольшое снижение пропускной способности (~15%)

### 9.2 CRDT-синхронизация

**Что:** Конфликт-free репликация всех данных между устройствами.

**Как:**

```
Изменение данных (любое):
  |
  v
CRDT Layer (Level 2):
  |
  +-- Causal Trees: для текста и упорядоченных данных (merge без потерь)
  +-- Operational Transformation (OT): для real-time collaborative editing
      поверх Causal Trees. Совместное редактирование документов в реальном времени.
  +-- LWW-Element-Set: для простых UI-состояний (last write wins по clock)
  +-- Hybrid Logical Clocks: для порядка событий
  +-- Hash-based Ordering: финальный арбитр для true concurrent conflict
      на регистре. Сравниваем BLAKE3 хэши значений, winner детерминирован.
      Никаких веток, никакого ручного merge.
  |
  v
Anti-entropy:
  |
  +-- Merkle Search Trees: быстрое обнаружение расхождений
  +-- Adaptive Sync Window: экспоненциальная задержка при перегрузке
  +-- Bit-Diff (XOR-дельта): 15x экономия трафика
  +-- Zstd-сжатие с кастомным словарём
  |
  v
При потере связи:
  +-- Локальное накопление мутаций -> CRDT-журнал растёт локально
  +-- При восстановлении связи -> автоматический детерминированный Merge
      (CRDT math + Hash-based Ordering для edge cases)
```

**Copy-on-Write для MST:**
- MST не обновляется "на месте"
- Новая версия дерева строится в свободных блоках
- Пока Root Hash не записан атомарно → валидна старая версия
- Питание упало → при перезагрузке CORE видит незавершённый Root Hash v.2 и мгновенно откатывается к v.1
- Ни одного битого байта

**Adaptive Sync Window (расширенное описание):**
- Экспоненциальная задержка (Backoff) при высокой нагрузке на канал
- Если канал забит → частота обмена хэшами падает до 1 раза в 5 секунд
- Приоритет: User Interactivity Stream (Z-layer) первым, Cold Storage — по остаточному принципу

**P2P Traffic Priority (IDLE / URGENT):**

| Приоритет | Что | Preemption |
|-----------|-----|------------|
| **URGENT** | CRDT-дельты, Intents, User Interactivity Stream | Прерывает IDLE немедленно |
| **NORMAL** | Metadata sync, MST anti-entropy | Стандартная очередь |
| **IDLE** | Бинарные blob (видео, логи), prefetch, background sync | Приостанавливается при URGENT |

- URGENT-флаг подписан device key (Ed25519). Mesh Engine отбрасывает неподписанные URGENT-пакеты (защита от fake-URGENT DoS)
- Padding: все URGENT-пакеты дополняются до фиксированного размера (traffic analysis mitigation)
- Bandwidth allocation: URGENT = 60% канала, NORMAL = 30%, IDLE = 10% (динамически)

**Bit-Diff (XOR-дельта) — расширенное описание:**
- Вместо полных 256-битных хэшей → XOR-разница между текущим и предыдущим состоянием узла
- Zstd-сжатие с кастомным словарём, заточенным под структуру Merkle-дерева
- Служебный трафик падает в 12–15 раз
- "Микроволновка" получает микро-пакет в 40 байт и восстанавливает изменения

**Где:** Level 2 (Mesh Engine, CRDT).

### 9.3 Устройства как один экран

**Что:** Фильм начат на компе -> продолжен на телефоне с той же секунды.

**Как:**

```
Session Handoff:
  |
  +-- Project Manager -> сериализует state проекта
  +-- CRDT -> синхронизирует state на все устройства
  +-- На телефоне -> Project Manager загружает state
  |   +-- Позиция воспроизведения -> timestamp
  |   +-- Окно плеера -> восстанавливается
  |   +-- Layout -> адаптируется под размер экрана
  +-- Плеер -> стримит видео с домашнего компа (P2P)
```

**Где:** Level 1 (Project Manager) + Level 2 (CRDT) + Level 3 (Layout Engine, адаптация).

### 9.4 Ленивая загрузка (On-Demand)

**Что:** На телефоне видна вся медиатека, но файлы весят 0 байт.

**Как:**

```
Файловая система:
  |
  +-- VFS (Level 0) показывает все файлы как "призраки"
  +-- Метаданные (паспорт) синхронизированы через CRDT
  +-- Тело файла — только по запросу
  |
  v
Запрос файла:
  +-- Сначала: локальный кэш
  +-- Затем: P2P-стрим с другого устройства
  +-- Последним: облачный буфер (если настроен)
  |
  v
Умный кэш:
  +-- Система видит паттерны использования
  +-- Предзагрузка часто используемых файлов
  +-- Автоматическая очистка по LRU
```

**Где:** Level 0 (VFS) + Level 2 (P2P, CRDT).

### 9.5 Seeding и смена primary Бэка

**Что:** Перенос всех данных с текущего primary Бэка (source) на новый узел (target) и переключение роли primary. Работает между любыми узлами: ноутбук → Core Base, ноутбук → сервер, сервер → VDS, VDS1 → VDS2.

**Как:**

```
Обнаружение target:
  |
  +-- Target включается → mDNS/Bonjour анонс (локальная сеть)
  +-- Или: ручное добавление по IP/адресу + код приглашения (удалённая сеть)
  +-- Source (текущий primary) видит target в списке доступных Бэков
  +-- Фронт показывает UI: «Сделать [Имя Бэка] основным?»
  |
  v
Подготовка:
  |
  +-- Если target пустой (новый узел) → полный seeding (см. ниже)
  +-- Если target уже имеет данные (бывший сбалансированный Бэк):
  |   +-- Система предлагает: «Объединить данные» (CRDT merge) или «Перезаписать target"
  |   +-- CRDT merge: оба журнала сливаются, конфликты разрешаются автоматически (CRDT math + Hash-based Ordering)
  |
  v
Seeding (инициирует source):
  |
  +-- ECDH handshake между source и target
  +-- WireGuard tunnel устанавливается автоматически
  +-- Source отправляет полный CRDT snapshot:
  |   +-- VFS metadata (Merkle Search Tree)
  |   +-- Profile Store (все профили, ключи, настройки)
  |   +-- App-scoped SQLite (данные приложений)
  |   +-- Audit logs
  |   +-- Configuration (App Registry, RBAC, политики)
  +-- Передача идёт по частям (chunked), с докачкой при обрыве
  +-- Progress отображается на Фронте
  |
  v
Переключение ролей:
  |
  +-- Target валидирует целостность snapshot (Merkle root)
  +-- Target активирует Profile Store → становится primary
  +-- Source меняет mode: primary → cached (или сбалансированный)
  +-- Sync Engine на всех узлах обновляет primary_node в mesh-конфиге
  +-- P2P Mesh перестраивает топологию: target = центральный узел
  |
  v
Fallback:
  +-- Если target недоступен > N часов: source предлагает вернуть primary
  +-- Owner подтверждает → source восстанавливает mode = primary
  +-- Данные не теряются: CRDT-ноды синхронизировались до отказа
  +-- Обратная миграция: повторный seeding source → target при восстановлении
```

**Где:** Level 2 (P2P Mesh, CRDT snapshot) + Level 1 (Profile Store, VFS) + Level 0 (WireGuard, Host Shim).

**Безопасность:**

- **Аутентификация target:** Перед seeding'ом source проверяет device key target через mesh-реестр. При ручном добавлении — одноразовый код приглашения от Owner (TTL 10 мин).
- **Авторизация:** Инициировать seeding может только Owner или админ с правом `back:promote`. RBAC Engine на source проверяет права перед запуском.
- **Целостность snapshot:** Snapshot подписан source device key (Ed25519). Target проверяет подпись + Merkle root перед активацией Profile Store.
- **Конфиденциальность:** Весь трафик через WireGuard (ChaCha20-Poly1305). ECDH handshake обеспечивает forward secrecy.
- **Rate limiting:** Один seeding в 24 часа на source. Повторный seeding требует явного подтверждения Owner.
- **Аудит:** Операция записывается в audit log: `action: back_seeding`, source device, target device, timestamp, profile_id, snapshot_size, result.
- **Recovery:** Для уровней безопасности «Повышенный» и «Максимальный» — ввод recovery-фразы или биометрия на source перед началом переноса.
- **Secure wipe:** Команда `core-cli back wipe --device <id>` — перезапись всех данных профиля на source случайными данными (AES-256-GCM encrypted zeros) при выводе из эксплуатации.

### 9.6 Lazy Boot / Headless Logic

**Что:** На устройствах с <512MB RAM и слабым CPU CORE запускается не полностью, а как "умный терминал".

**Как:**

```
Устройство с ограниченными ресурсами:
  |
  +-- Host Shim активирует режим Headless Logic
  +-- Запускается только Level 3 (Display Server) + Input Handler
  +-- Level 1 (Micro-Kernel) и Level 2 (Mesh Engine) — не запускаются локально
  +-- Вся тяжёлая логика делегируется ближайшему мощному узлу (Core Base, Бэк-офис, ПК)
      через Remote Isolate Call по P2P Mesh
  |
  v
Результат: Планшет за $50 превращается в умный терминал
  - Не греется
  - CPU 15%
  - На экране — живой, отзывчивый нативный интерфейс
  - 90% JS-кода крутится на сервере в соседней комнате
```

**Fallback:** Если мощный узел недоступен → система показывает последний кэшированный layout (Shadow State) и плашку "Офлайн-режим".

**Где:** Level 0 (Host Shim) + Level 3 (Display Server) + Level 2 (P2P Mesh для Remote Isolate Call).

---

### 9.7 Backup Engine

**Что:** Резервное копирование данных Бэка на внешние носители.

**Как:**

```
Backup Engine (Level 1):
  |
  +-- BackupTarget interface:
      +-- UsbTarget: USB-накопитель
      +-- CoreTarget: другой узел CORE через P2P
      +-- S3Target: облачное хранилище
      +-- SftpTarget: SFTP-сервер
      +-- CustomTarget: плагин
  +-- Шифрование: Backup Key (derived из recovery-фразы, BIP-39)
  +-- Инкрементальные бэкапы: только изменённые CID
  +-- Проверка целостности: Merkle root backup snapshot
```

**Где:** Level 1 (VFS + Key Manager) + Level 0 (Host Shim для записи на носители).

---

### 9.8 Energy Manager

**Что:** Управление энергопотреблением и производительностью.

**Как:**

```
Energy Manager (Level 1 + Level 0):
  |
  +-- Адаптивная синхронизация: при <20% батареи — P2P backoff, background sync только по Wi-Fi
  +-- Display Server low-power mode: снижение FPS, отключение эффектов
  +-- CPU governor: снижение частоты при перегреве / низком заряде
  +-- NPU/GPU scheduling: переключение AI-нагрузки на CPU при недоступности акселераторов
```

**Где:** Level 1 (Scheduler + Sync Engine) + Level 0 (Host Shim для CPU/GPU governors).

---

## 10. Security Layer

Безопасность — кросс-слойная тема, подробно описана в [Layer Security](layer-7-security.md).

**Что реализовано на уровне подсистем:**

| Подсистема | Level | Описание |
|-----------|-------|----------|
| Key Manager | Level 1 | Хранение ключей, генерация производных ключей. Пароли не покидают устройство |
| Auth Proxy | Level 1 | OAuth flow, выдача токенов. Приложения не видят пароли |
| Шифрование | Level 0 + Level 1 | AES-256-GCM (at rest), Ed25519 (ключи), WireGuard (транзит), BLAKE3 (хеш) |
| RBAC Engine | Level 1 | Owner/Member/Guest + кастомные роли, группы, scope (Space/проект) |
| Audit Engine | Level 1 | 13 категорий, include/exclude, SQLite |
| User Directory | Level 1 | Local/LDAP/OAuth/Custom провайдеры |
| Capability Security | Level 1 | Контекст приложения с разрешениями (fs, network, graphics, mind, contacts) |

### 10.1 CRDT Rate Limiting & Quarantine
- Rate limiting: max N операций в секунду от одного устройства
- CRDT garbage collection: операции старше M дней, уже merged — удаляются
- CRDT checkpoint: периодический снапшот состояния для отката
- CRDT quarantine: подозрительный поток от узла приостанавливается до подтверждения Owner

### 10.2 Seeding Security Protocol
- Аутентификация target через device key pinning
- Только Owner или админ с правом `back:promote` может инициировать seeding
- Snapshot подписан source device key (Ed25519)
- Target проверяет подпись + Merkle root перед активацией
- Rate limiting: один seeding в 24 часа на source

### 10.3 Session Management
- TTL сессии, auto-lock после простоя
- Принудительный отзыв сессии Owner'ом
- Уведомление push на Фронт при отзыве

### 10.4 Secure Memory Wipe
- Zeroize V8 heap regions при freeze/thaw профиля
- Обнуление SharedArrayBuffer regions при переключении профиля
- Очистка clipboard при переключении

### 10.5 Encrypted Memory (уровень «Максимальный»)
- Шифрование страниц AES-256-GCM-SIV
- Page fault handler: расшифровка только при доступе

### 10.6 Boot Integrity (Core Base / Raspberry Pi)
- LUKS encrypted rootfs по умолчанию
- Signed kernel + initrd
- QR-based passphrase через BLE/LAN с телефона
- Integrity Monitoring Agent (BLAKE3)

### 10.7 Recovery & Shamir's Secret Sharing
- Recovery-фраза (BIP-39, 24 слова) → backup key + profile key
- Shamir's Secret Sharing: 3 части, нужно 2 из 3
- Social recovery через доверенные контакты

### 10.8 Incognito Mode
- Отдельный V8 Isolate, no cache, no cookies
- DNS через DoH
- Auto-wipe при закрытии

### 10.9 Remote Attestation
- TPM-based proof-of-integrity по запросу Бэка
- Опционально для корпоративных сценариев

### 10.10 Behavioural Analysis
- ML-модель для выявления аномалий в API-вызовах и сетевом трафике приложений
- Пример: массовое чтение SQLite + сетевые запросы = высокий риск

### 10.11 Supply Chain Protection

- **Pinning зависимостей:** Cargo.lock и bun.lockb коммитятся в репозиторий. Exact versions в package.json, без `^` или `~`
- **Reproducible builds:** Docker-контейнер с фиксированным toolchain. Два разных разработчика → одинаковый build.hash
- **CI security gates:**
  - `seccomp_validate` — сверка seccomp-профилей
  - `namespace_test` — проверка изоляции namespace
  - `v8_sandbox_test` — тесты на escape из V8 Sandbox
  - `threat_model_check` — автоматическая проверка изменений threat model

**Где:** CI/CD pipeline (вне runtime Workspace).

### 10.12 Incident Response

**Runbook при security-инциденте:**

1. **Изоляция узла** — отключение компрометированного устройства от mesh, отзыв session token
2. **Отзыв ключей** — Owner инициирует rotation device keys через Key Manager
3. **Аудит** — полный разбор аудит-логов (13 категорий), выявление затронутых данных
4. **Уведомление** — push-уведомление всем Owner'ам затронутых Space
5. **Восстановление** — восстановление из backup, проверка целостности CRDT-истории
6. **SIEM integration** — опциональная отправка критичных событий во внешнюю SIEM-систему

**Где:** Level 1 (Audit Engine, Key Manager, Backup Engine) + Level 2 (Sync Engine для mesh isolation).

Подробно: аутентификация, шифрование, алгоритмы, RBAC, аудит, User Directory, изоляция, векторы атак — в [Layer Security](layer-7-security.md).

---

## 11. Space Manager

### 11.1 Размещение Space

**Что:** Каждый Space занимает область экрана. Space Manager управляет размещением, разделителями, переходами между состояниями.

**Как:**

```
Space Layout (Level 3, Display Server):
  |
  +-- Space Container: bounds, z-index, state
  +-- Разделители: перетаскиваемые, горизонтальные/вертикальные
  +-- Размещение:
  |   +-- Один Space на все экраны (full)
  |   +-- Каждый Space на своём экране (per-monitor)
  |   +-- Сплит (split) — Space на части экрана
  |   +-- Мульти-монитор (span) — один Space на несколько экранов
  |
  +-- Screen Manager (Level 0, Host Shim):
      +-- Отслеживает подключение/отключение мониторов
      +-- Передаёт размеры экранов в Space Manager
      +-- Space Manager пересчитывает layout
```

**Где:** Level 0 (Host Shim, мониторы) + Level 3 (Display Server, layout).

### 11.2 Состояния Space

**Что:** Space может быть открытым, скрытым, фоновым, оффлайн, отключённым, отозванным.

**Как:**

```
State Machine (Level 1):
  |
  +-- Open → Hide (пользователь скрыл)
  +-- Open → Background (пользователь сделал фоновым)
  +-- Open → Offline (потеряна связь с Бэком)
  +-- Open → Disconnected (пользователь отключил)
  +-- Offline → Open (связь восстановлена)
  +-- Disconnected → Open (переподключение)
  +-- Any → Revoked (Бэк отозвал доступ, необратимо)
  |
  +-- При переходе в Offline:
  |   +-- Space Manager уведомляет Display Server
  |   +-- Display Server накладывает серый оверлей
  |   +-- Command Bar переключается на локальный кэш
  |   +-- Мутации складываются в CRDT-очередь
  |
  +-- При переходе в Open (из Offline):
      +-- CRDT-синхронизация с Бэком
      +-- Оверлей снимается
      +-- Command Bar возвращается к онлайн-режиму
```

**Где:** Level 1 (Micro-Kernel, state machine) + Level 3 (Display Server, оверлей) + Level 2 (CRDT).

### 11.3 Визуальное отличие Space

**Что:** Пользователь видит, в каком Space находится: фоновая подложка + метка в Command Bar.

**Как:**

```
Space Settings (SQLite, Level 1):
  |
  +-- space_id, name, icon, color, bg_type, bg_value
  +-- bg_type: "solid" | "gradient" | "pattern" | "image"
  +-- bg_value: цвет/путь/параметры
  |
  +-- Command Bar (Level 3):
      +-- Слева в строке: [icon] [mode_icon] [input...]
      +-- icon = space.icon, цвет фона = space.color
      +-- При нескольких Space на экране — визуальная граница (разделитель) + разный фон
```

**Где:** Level 1 (настройки в SQLite) + Level 3 (рендеринг подложки и метки).

### 11.4 Оффлайн-Space

**Что:** При потере связи с Бэком — Space продолжает работать с кэшем.

**Как:**

```
Потеря связи:
  |
  +-- P2P Mesh (Level 2) → heartbeat timeout
  +-- Space Manager → state = Offline
  +-- Display Server → серый оверлей + сообщение
  |
  +-- Работа с кэшем:
  |   +-- Data Cache (Level 1) → отдаёт кэшированные данные
  |   +-- Мутации → CRDT-очередь (Level 2)
  |   +-- Файлы не в кэше → "Файл доступен только при подключении"
  |   +-- Command Bar → поиск по локальному кэшу
  |
  +-- Восстановление связи:
      +-- P2P Mesh → reconnect
      +-- CRDT → merge
      +-- Space Manager → state = Open
      +-- Display Server → снять оверлей
```

**Где:** Level 2 (P2P, CRDT) + Level 1 (Data Cache, state machine) + Level 3 (Display Server, оверлей).

---

### 11.5 Фоновый Space

**Что:** Space, который не занимает графическую область на экране, но продолжает работать: синхронизация, фоновые задачи, приём уведомлений.

**Как:**

```
Background Space (Level 1):
  |
  +-- Нет Display Server области (не рендерится, не композитится)
  +-- Sync Engine (Level 2) — продолжает синхронизацию с Бэком
  +-- Notification Manager — принимает push-уведомления от Бэка
  +-- Scheduler — выполняет фоновые задачи (напоминания, закачки)
  |
  +-- Взаимодействие с пользователем:
      +-- Через уведомления активного Space (Notification Manager агрегирует)
      +-- Через Command Bar активного Space (пользователь может развернуть фоновый Space обратно в Open)
      +-- Voice-команды маршрутизируются в активный Space; фоновый не обрабатывает ввод напрямую
```

**Где:** Level 1 (Micro-Kernel, state machine) + Level 2 (Sync Engine). Display Server не участвует.

---

## 12. Notifications

### 12.1 Единый поток уведомлений

**Что:** Все уведомления из всех Space собираются в единый поток. Каждое помечено иконкой и цветом Space.

**Как:**

```
Push от Бэков:
  |
  +-- Space A (Бэк A) → push: { type, message, space_id }
  +-- Space B (Бэк B) → push: { type, message, space_id }
  |
  +-- Notification Manager (Level 1):
      +-- Собирает push от всех подключённых Бэков
      +-- Помечает каждое уведомление space_id
      +-- Применяет пользовательские приоритеты
      |
      +-- Приоритеты (настройки на Фронте, SQLite):
      |   +-- Прерывающий (interrupt): звук + вибрация + всплывающее окно
      |   +-- Тихий (silent): значок в индикаторе
      |   +-- Молчащий (muted): не показывается
      |   +-- Default: первый Space — прерывающий, остальные — тихие
      |
      +-- Display Server (Level 3):
          +-- Рендерит всплывающее уведомление с иконкой/цветом Space
          +-- Скрытые уведомления — в истории (доступна через Command Bar)
```

**Где:** Level 1 (Notification Manager, SQLite) + Level 3 (Display Server, рендеринг) + Level 2 (P2P, приём push).

### 12.2 Центр уведомлений

**Что:** История уведомлений с навигацией и управлением.

**Как:**

```
Notification Center (Level 1 + Level 3):
  |
  +-- SQLite (Level 1): таблица notifications
  |   +-- id, space_id, type, message, timestamp, read_status, action_url
  |   +-- Группировка по space_id + date bucket (сегодня, вчера, ранее)
  |   +-- Очистка: DELETE по space_id или полная TRUNCATE
  |
  +-- Display Server (Level 3):
      +-- Command Bar → "уведомления" → список
      +-- Свайп с верхнего края → панель истории
      +-- Клик по уведомлению → switch Space + open action_url
      +-- Badge (точка) на иконке Space при непрочитанных
```

**Где:** Level 1 (SQLite, хранение) + Level 3 (Display Server, UI).

---

## 13. Clipboard (Мультибуфер обмена)

### 13.1 Стек копирований

**Что:** Фронт хранит историю копирований — не один элемент, а стек.

**Как:**

```
Clipboard Manager (Level 1):
  |
  +-- Clipboard Store (in-memory, per Space):
  |   +-- stack: [{ content, type, timestamp }]
  |   +-- max_size: настраивается пользователем (default: 20)
  |   +-- types: text, image, file, url
  |
  +-- Ctrl+C → push в стек
  +-- Ctrl+V → pop верхний элемент → вставка
  +-- Ctrl+Shift+V → UI показывает весь стек (Level 3)
  +-- Command Bar: "буфер обмена" → история + настройки
  |
  +-- Настройки пользователя (комфорт):
  |   +-- Размер стека (default: 20)
  |   +-- Типы контента (default: все)
  |   +-- Очистка при выходе из Space (default: нет)
  |
  +-- Политики Бэка (безопасность, настраивает Owner):
      +-- clipboard_allow_export: none / text / all
      +-- clipboard_life_time: максимальный срок хранения
      +-- clipboard_persist: разрешить ли сохранять между сессиями
```

### 13.2 Буфер обмена между Space

**Как:**

```
Копирование между Space:
  |
  +-- Пользователь копирует из Space A
  +-- Вставляет в Space B
  +-- Clipboard Manager проверяет политику Бэка A (clipboard_allow_export):
  |   +-- "all" → вставка разрешена
  |   +-- "text" → только текст, файлы → "Используйте 'Скопировать в Space'"
  |   +-- "none" → "Политика запрещает копирование из этого Space"
  |
  +-- Для файлов: механизм копирования между Space (см. 13.3)
```

### 13.3 Копирование файлов между Space

**Что:** Бэк-контролируемая операция. Фронт — триггер, не видит содержимое.

**Как:**

```
Пользователь → drag-and-drop / Command Bar / контекстное меню
  |
  +-- Фронт → Бэк A: "экспорт файла X для Space B"
  +-- Бэк A → проверка политики → генерация токена экспорта (TTL)
  +-- Фронт → Бэк B: "импорт по токену"
  +-- Бэк B → проверка политики → связывается с Бэком A напрямую (P2P/IPC)
  +-- Бэк A → Бэк B: передача данных напрямую, Фронт не участвует
  |
  +-- UI: прогресс-бар "Копирование... 45%"
  +-- При отказе: "Корпоративная политика запрещает экспорт файлов"
```

**Где:** Level 1 (Clipboard Manager) + Level 2 (P2P, передача между Бэками) + Level 3 (UI, drag-and-drop).

### 13.4 Бесшовный буфер обмена P2P

**Что:** Копирование на одном устройстве → вставка на другом через 50мс.

**Как:**

```
Пользователь нажал Ctrl+C на ноутбуке:
  |
  v
Clipboard Manager (Level 1, ноутбук)
  |
  +-- Push в локальный стек
  +-- Отправляет top-of-stack через P2P Mesh (WireGuard tunnel) на все связанные устройства
  +-- Payload: content (encrypted), type, timestamp, TTL (default: 60 сек)
  |
  v
Clipboard Manager (Level 1, телефон)
  +-- Получает пакет → push в стек
  +-- Показывает ненавязчивый индикатор: "Скопировано с ноутбука"
  |
  v
Пользователь нажал Ctrl+V на телефоне → вставляет содержимое с ноутбука
```

**Ограничения:**
- Только текст и URL (не файлы — файлы через VFS Sharing)
- TTL 60 секунд (буфер не разрастается)
- Шифрование: WireGuard (ChaCha20-Poly1305) + payload encrypted device key
- Политика Бэка: `clipboard_allow_export` применяется и к P2P-sync (если "none" — P2P-буфер отключён)

**Где:** Level 1 (Clipboard Manager) + Level 2 (P2P Mesh).

---

## 14. RBAC (Role-Based Access Control)

Три встроенные роли (Owner/Member/Guest), кастомные роли, группы, два уровня назначения (Space/проект). RBAC Engine на Level 1 (Micro-Kernel, SQLite). Проверка при каждом API-запросе.

Управление: контекстное меню (Level 3), Command Bar (Level 1 + Level 4), Core.Hardcore (CLI).

Подробно: роли, наследование, группы, техническая реализация — в [Layer Security, секция 3](layer-7-security.md).

---

## 15. Audit

Бэк логирует действия пользователей. 13 категорий, include/exclude фильтры, настраиваемый retention. Audit Engine на Level 1 (Micro-Kernel, отдельная SQLite-таблица). Owner видит всё, Member/Guest — не видят.

Просмотр: Core.Hardcore (CLI), Command Bar (Intent Parser), контекстное меню (история доступа).

Подробно: 13 категорий, настройки, JSON-конфиг, техническая реализация — в [Layer Security, секция 4](layer-7-security.md).

---

## 16. User Directory & Backoffice Profiles

### 16.1 User Directory

Абстракция `UserDirectory` с четырьмя провайдерами: Local (default), LDAP (AD, FreeIPA), OAuth (Google, Microsoft, GitHub), Custom (плагин). Маппинг групп → роли CORE. Настройка через Core.Hardcore (CLI) или Command Bar (Owner).

**Где:** Level 1 (Micro-Kernel, UserDirectory interface + implementations).

Подробно: провайдеры, настройка, техническая реализация — в [Layer Security, секция 5](layer-7-security.md).

### 16.2 Профили Бэка (Backoffice Profiles)

**Что:** Бэк — единый пакет компонентов. Ядро обязательно, остальные настраиваются.

**Как:**

```
Component Manager (Level 1):
  |
  +-- Ядро (обязательные):
  |   +-- SQLite, CRDT-журнал, Key Manager
  |
  +-- Опциональные:
  |   +-- Chat Engine, Scheduler, Search Engine, P2P-сервер
  |   +-- App Registry, VoIP, Tag Engine, Sync Engine (сервер), Auth Proxy
  |
  +-- Профили настроек:
  |   +-- Минимальный: только ядро
  |   +-- Сбалансированный: ядро + Chat + Scheduler (default для телефонов)
  |   +-- Полный: все компоненты (default для ПК/Core Base)
  |
  +-- Конфигурация:
      +-- При установке: мастер настройки (рекомендует профиль по устройству)
      +-- После: Command Bar "settings backoffice components"
      +-- Core.Hardcore: core-cli component enable/disable
      +-- Компоненты переключаются динамически
```

**Где:** Level 1 (Micro-Kernel, Component Manager).

### 16.3 Self-healing Back-office

**Что:** Бэк-офис обнаруживает собственные сбои и восстанавливается без участия пользователя.

**Как:**

```
Watchdog (уровень Бэка):
  |
  +-- Health Check каждые 30 сек:
  |   +-- SQLite — запрос SELECT 1
  |   +-- CRDT Journal — последняя запись < 5 мин назад
  |   +-- P2P Mesh — количество пиров > 0 (если устройство не офлайн)
  |   +-- Memory — RSS < 80% от лимита
  |
  +-- При сбое:
      +-- SQLite: автоматический ROLLBACK + REINDEX при битом индексе
      +-- CRDT Journal: truncate до последнего валидного root hash
      +-- P2P Mesh: реинициализация сетевого стека, fallback на TCP relay
      +-- Memory: graceful restart компонента (не всего Бэка)
      +-- Если 3 сбоя подряд → уведомление Owner: "Требуется внимание"
```

**Принцип Digital Refrigerator:** Пользователь включает Бэк-офис и забывает про него. Watchdog гарантирует, что система сама себя чинит 95% случаев.

**Где:** Level 1 (Micro-Kernel, Watchdog) + Level 2 (Бэк, Health Check API).

---

## 17. Connection Protocol (Подключение к чужому Бэку)

### 17.1 Процесс подключения

**Что:** Три способа (код, QR, адрес). Технически — одноразовый токен → ECDH → WireGuard → device key.

**Как:**

```
Шаг 1: Генерация приглашения (Бэк):
  |
  +-- Owner → "Пригласить в Space"
  +-- Invite Engine (Level 1):
      +-- Генерирует токен (32 байта, base32: CORE-A7X9-K2M4)
      +-- Привязка: role, TTL (24ч default)
      +-- Хранит: invite_tokens (SQLite)
      |
      +-- QR-код: { back_address, token } — одно сканирование
      +-- Прямой адрес: без токена, политика allow_anonymous_connect

Шаг 2: Подключение (Фронт):
  |
  +-- Фронт получает токен (ввод / QR / адрес)
  +-- Connection Manager (Level 1):
      +-- Отправляет токен на Бэк по указанному адресу
      |
      +-- Бэк верифицирует:
      |   +-- Токен валиден? → продолжить
      |   +-- Токен истёк? → отказ
      |   +-- Токен использован? → отказ
      |
      +-- ECDH Handshake:
      |   +-- Бэк → публичный ключ
      |   +-- Фронт → публичный ключ
      |   +-- Обе стороны → session key (shared secret)
      |
      +-- WireGuard Tunnel:
      |   +-- Установка зашифрованного туннеля
      |   +-- Внутренний IP для нового устройства
      |
      +-- Бэк выдаёт credentials:
      |   +-- device_key (привязан к устройству + роли)
      |   +-- refresh_token
      |
      +-- Токен приглашения уничтожается (одноразовый)

Шаг 3: Последующие подключения:
  |
  +-- Фронт → device_key → Бэк
  +-- Бэк верифицирует device_key → открывает сессию
  +-- Без повторного приглашения
```

**Где:** Level 1 (Invite Engine, Connection Manager) + Level 2 (WireGuard, P2P) + Level 3 (QR-сканер UI).

---

| Layer 1 (UX-сценарий) | Layer 2 (подсистема) | Architecture Level |
|------------------------|---------------------|--------------------|
| Строка — ввод текста | Input Router | Level 1 |
| Строка — подсказки | Suggestion Engine | Level 1 + Level 3 |
| Строка — 8 режимов | Command Bar (8 модулей) | Level 0-4 |
| Настройки строки | Shell Settings (SQLite) | Level 1 + Level 3 |
| Настройки главного экрана | Home Project Settings | Level 1 + Level 3 |
| Проект — создание | Project Manager | Level 1 |
| Проект — автосохранение | Session Persistence (SQLite + CRDT) | Level 1 + Level 2 |
| Проект — сплит | Layout Engine | Level 0 + Level 3 |
| Окна — размещение | Window Manager | Level 3 |
| Окна — веб-сайты | Island Mode | Level 2 + Level 3 |
| Окна — нативные | WebGPU Renderer | Level 2 + Level 3 |
| Приложения — запуск | App Runtime (V8 Isolates) | Level 1 |
| Приложения — установка | App Registry | Level 1 + Level 2 |
| Приложения — изоляция | Capability Security | Level 1 |
| Поиск по файлам | Search Engine (FTS5) | Level 1 |
| Теги | Tag Store (SQLite) | Level 1 + Level 2 |
| Связи между сущностями | Graph Store | Level 1 |
| Мессенджер | Chat Engine + P2P | Level 1 + Level 2 |
| Почта | Email Engine | Level 1 |
| Звонки | VoIP (WebRTC) | Level 0 + Level 2 |
| "Скинь документ Ивану" | Intent API + Chat Engine | Level 4 + Level 1 |
| Голос — распознавание | Whisper Model | Level 0 + Level 4 |
| Голос — команды | Intent Parser | Level 4 |
| Голос — без экрана | Zero UI | Level 0 + Level 4 |
| Профили — переключение | Profile Manager | Level 1 |
| Профили — заморозка | Freeze/Thaw | Level 1 |
| Профили — анонимный | RAM-only Storage | Level 1 |
| Синхронизация | Sync Engine (CRDT) | Level 2 |
| Устройства — один экран | Session Handoff | Level 1 + Level 2 + Level 3 |
| Безопасность — ключи, прокси, шифрование, аутентификация | → [Layer Security](layer-7-security.md) | Level 0 + Level 1 + Level 2 |
| Space — размещение | Space Manager | Level 0 + Level 3 |
| Space — состояния | Space Manager (state machine) | Level 1 + Level 2 |
| Space — визуальное отличие | Space Manager (bg + метка) | Level 1 + Level 3 |
| Space — оффлайн | Space Manager + CRDT + Data Cache | Level 1 + Level 2 + Level 3 |
| Space — подключение | Connection Protocol (токен, ECDH) | Level 1 + Level 2 |
| Уведомления | Notification Manager (единый поток, приоритеты) | Level 1 + Level 2 + Level 3 |
| Буфер обмена | Clipboard Manager (мультибуфер, политики) | Level 1 + Level 2 + Level 3 |
| Копирование между Space | P2P direct transfer (Бэк-контролируемая) | Level 2 |
| Роли и права, Аудит, User Directory | → [Layer Security](layer-7-security.md) | Level 1 |
| Профили Бэка | Component Manager (ядро + опциональные) | Level 1 |
| Носители — импорт/экспорт | Storage Manager (Import/Export Engine, Dedup) | Level 0 + Level 1 + Level 3 |

---

## Предыдущий слой

Layer 7 описывает безопасность — аутентификацию без паролей, шифрование, RBAC, аудит, изоляцию и модель угроз. [См. layer-7-security.md](layer-7-security.md).

---

## Следующий слой

Layer 9 описывает **требования к железу** — минимальные и рекомендуемые характеристики устройств, поддерживаемое оборудование, оптимизации под слабое железо. [См. layer-9-hardware-requirements.md](layer-9-hardware-requirements.md).

Layer 10 описывает **бизнес-модель и Go-to-Market** — монетизацию, целевые сегменты, стратегию выхода на рынок и партнёрства. [См. layer-10-business-model.md](layer-10-business-model.md).
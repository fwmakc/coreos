# Layer 7 — Technical decomposition | Техническая декомпозиция

Как каждый сценарий из Layer 1 и Layer 2 реализуется технически. Какие подсистемы участвуют, как данные текут между ними, на каком уровне архитектуры каждая живёт. Сценарии установки описаны в [Layer: Установка](layer-installation-scenarios.md). Взаимодействие с устройствами и носителями — в [Layer Devices](layer-devices.md).

Ссылки: [architecture.md](../project/architecture.md), [tech-stack.md](../project/tech-stack.md), [ui-framework.md](../project/ui-framework.md), [filesystem.md](../project/filesystem.md), [ai-layer.md](../project/ai-layer.md).

---

## Карта подсистем

```
CORE OS Subsystems:

Command Bar      Project Mgr      Window Mgr       App Runtime
  Input Router     Lifecycle        Z-index          V8 Isolates
  Suggest          Sessions         Web-окна         App Registry
  8 режимов        Layout           Нативные         Lifecycle
  Settings         Home             Layout           Sandboxing

Search &         Communication    Voice Engine     Profile Mgr
Tag Engine       Layer
  Full-text        Messenger        Whisper          Switching
  Tags             Email            Intent Parser    Freeze/Thaw
  Graph            VoIP             Zero UI          Anonymous
  Smart Folders    AI bridge                         Background

Sync Engine      Security Layer   Display Server   Intent API
  P2P mesh         Key Manager      WebGPU           Semantic Kernel
  CRDT             Auth Proxy       Layout Engine    Intent Map
  Devices          Encrypt          Effects          Generative UI
  Lazy load

Space Manager    Notifications    Clipboard        RBAC & Audit
  Размещение       Единый поток     Мультибуфер      Роли (Owner/Member/Guest)
  Состояния        Приоритеты       Стек истории     Группы
  Визуальное       Метки Space      Фильтры          Аудит-лог (13 категорий)
  Оффлайн          Push             Политики Бэка    User Directory

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
| Level 0: Host Shim (Rust) | Window, GPU, Input, Memory Bridge, VFS bridge | [architecture.md](../project/architecture.md) |
| Level 1: Micro-Kernel (Bun) | IPC, Isolate Management, SQLite, Capability Security | [architecture.md](../project/architecture.md) |
| Level 2: Mesh Engine | P2P, CRDT, Content Addressing, Anti-entropy | [architecture.md](../project/architecture.md) |
| Level 3: Display Server | WebGPU Renderer, Layout Engine, Universal Shell, Effects | [architecture.md](../project/architecture.md) |
| Level 4: Intent API | Semantic Kernel, Intent Map, Generative UI, Voice | [architecture.md](../project/architecture.md) |

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

**Принцип:** Shell может падать, но окна продолжают рендериться. Shadow State Recovery (см. [architecture.md](../project/architecture.md)).

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
---

## 4. App Runtime

Модель приложений — 5 уровней интеграции: от «набрал URL» до «полный натив на WebGPU». Манифест `core.json`, app-scoped SQLite, `@core/*` API, App Registry, безопасность — подробно в [Layer Apps](layer-apps.md).

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

**Где:** Level 1 (Micro-Kernel, Bun + V8).

### 4.2 App Registry

**Что:** Реестр всех доступных приложений. Установка = адрес. Модель приложений (5 уровней, манифест, хранение, магазин) — в [Layer Apps, секция App Registry](layer-apps.md).

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

**Где:** Level 0 (аудио через Host Shim) + Level 4 (Whisper model, Intent Parser).

### 7.2 Intent Parser

**Что:** Текст/голос -> типизированная команда.

**Как:**

```
"Запиши: купить краску для ванной"
  |
  v
Intent Parser (Level 4):
  +-- Первое слово "запиши" -> Intent: create_note
  +-- Параметры: content = "купить краску для ванной"
  +-- Контекст: текущий проект = "Ремонт"
  |
  v
Intent Map -> находит Core.Notes.create_note()
  |
  v
Вызов: Core.Notes.create_note({ content, project: "Ремонт" })
  |
  v
Заметка создана. TTS: "Записал в проект Ремонт"
```

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

### 9.2 CRDT-синхронизация

**Что:** Конфликт-free репликация всех данных между устройствами.

**Как:**

```
Изменение данных (любое):
  |
  v
CRDT Layer (Level 2):
  |
  +-- Causal Trees: для текста и упорядоченных данных
  +-- LWW-Element-Set: для простых UI-состояний (last write wins)
  +-- Hybrid Logical Clocks: для порядка событий
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
  +-- Локальный Fork -> данные сохраняются локально
  +-- При восстановлении связи -> автоматический Merge
```

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

---

## 10. Security Layer

Безопасность — кросс-слойная тема, подробно описана в [Layer Security](layer-security.md).

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

Подробно: аутентификация, шифрование, алгоритмы, RBAC, аудит, User Directory, изоляция, векторы атак — в [Layer Security](layer-security.md).

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

---

## 14. RBAC (Role-Based Access Control)

Три встроенные роли (Owner/Member/Guest), кастомные роли, группы, два уровня назначения (Space/проект). RBAC Engine на Level 1 (Micro-Kernel, SQLite). Проверка при каждом API-запросе.

Управление: контекстное меню (Level 3), Command Bar (Level 1 + Level 4), Core.Hardcore (CLI).

Подробно: роли, наследование, группы, техническая реализация — в [Layer Security, секция 3](layer-security.md).

---

## 15. Audit

Бэк логирует действия пользователей. 13 категорий, include/exclude фильтры, настраиваемый retention. Audit Engine на Level 1 (Micro-Kernel, отдельная SQLite-таблица). Owner видит всё, Member/Guest — не видят.

Просмотр: Core.Hardcore (CLI), Command Bar (Intent Parser), контекстное меню (история доступа).

Подробно: 13 категорий, настройки, JSON-конфиг, техническая реализация — в [Layer Security, секция 4](layer-security.md).

---

## 16. User Directory & Backoffice Profiles

### 16.1 User Directory

Абстракция `UserDirectory` с четырьмя провайдерами: Local (default), LDAP (AD, FreeIPA), OAuth (Google, Microsoft, GitHub), Custom (плагин). Маппинг групп → роли CORE. Настройка через Core.Hardcore (CLI) или Command Bar (Owner).

**Где:** Level 1 (Micro-Kernel, UserDirectory interface + implementations).

Подробно: провайдеры, настройка, техническая реализация — в [Layer Security, секция 5](layer-security.md).

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
| Безопасность — ключи, прокси, шифрование, аутентификация | → [Layer Security](layer-security.md) | Level 0 + Level 1 + Level 2 |
| Space — размещение | Space Manager | Level 0 + Level 3 |
| Space — состояния | Space Manager (state machine) | Level 1 + Level 2 |
| Space — визуальное отличие | Space Manager (bg + метка) | Level 1 + Level 3 |
| Space — оффлайн | Space Manager + CRDT + Data Cache | Level 1 + Level 2 + Level 3 |
| Space — подключение | Connection Protocol (токен, ECDH) | Level 1 + Level 2 |
| Уведомления | Notification Manager (единый поток, приоритеты) | Level 1 + Level 2 + Level 3 |
| Буфер обмена | Clipboard Manager (мультибуфер, политики) | Level 1 + Level 2 + Level 3 |
| Копирование между Space | P2P direct transfer (Бэк-контролируемая) | Level 2 |
| Роли и права, Аудит, User Directory | → [Layer Security](layer-security.md) | Level 1 |
| Профили Бэка | Component Manager (ядро + опциональные) | Level 1 |
| Носители — импорт/экспорт | Storage Manager (Import/Export Engine, Dedup) | Level 0 + Level 1 + Level 3 |

---

## Следующий слой

Layer 5 описывает **API-контракты** — конкретные интерфейсы между подсистемами, форматы данных, протоколы взаимодействия. Переход от "как работает" к "как подключиться".
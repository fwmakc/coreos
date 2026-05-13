# Layer 11 — Developer Reference | Справочник разработчика

> **Назначение:** Единый технический справочник для разработки Workspace. Содержит все сущности, API, конфигурации, потоки данных, горячие клавиши, системные команды и описание интерфейса.
>
> **Это слой** — агрегирует данные из layer-1, layer-2, layer-3, layer-6, layer-8 в формате, пригодном для ежедневной работы разработчика.

---

## 1. Command Bar (Строка)

### 1.1 Архитектура

| Компонент | Уровень | Язык / Runtime | Ответственность |
|-----------|---------|----------------|-----------------|
| Input Router | Level 1 | Bun (TypeScript) | Определение режима по паттерну ввода |
| Suggestion Engine | Level 1 | Bun + SQLite FTS | Ранжирование подсказок |
| Mode Handlers (8 шт.) | Level 1–4 | Bun / V8 Isolate | Исполнение команд |
| Settings Store | Level 1 | SQLite | Хранение настроек строки |
| Display Renderer | Level 3 | Rust + WebGPU | Рендеринг строки и выпадающих списков |

### 1.2 Input Router

**Триггеры режимов (регулярные выражения):**

| Паттерн | Режим | Пример |
|---------|-------|--------|
| `/^@/` | Мессенджер | `@ivan` |
| `/@[\w]+\.[\w]+/` | Email | `user@domain.com` |
| `/^\+\d/` | Мессенджер (звонок) | `+79991234567` |
| `/^(\$|>)\s*/` | Терминал | `$ cargo build` |
| `/\.(com\|ru\|org)\b/i` | Браузер | `youtube.com` |
| `/^[\d+\-*/^().\s]+$/` | Калькулятор | `sin(45) + 2^10` |
| `/(напомни\|завтра\|через\s+\d+)/i` | Напоминание | `напомни завтра позвонить` |
| `fallback` | Поиск + ИИ-агент | `фото со стройки` |

**Ручной выбор режима:** клик по иконке или горячая клавиша (`Tab` / `1`–`8`).

**При неуверенности:** Router выдаёт top-3 варианта с confidence score. Выбор пользователя записывается в персональную историю (`shell_input_history` SQLite).

### 1.3 Suggestion Engine

**Источники (5 штук):**
1. App Registry — совпадение по имени/тегам приложений
2. Search Index — файлы, проекты (FTS5 + OCR)
3. Contact Book — люди, email, номера
4. History — недавние действия пользователя
5. Intent Map — зарегистрированные команды приложений (`@workspace/intents`)

**Ранжирование:**
1. Точное совпадение имени приложения
2. Контекст текущего проекта
3. Частота использования
4. Давность

**Техника:** Level 1 (SQLite + FTS5) + Level 3 (рендеринг списка).

### 1.4 Восемь режимов — технические потоки

#### 1. Поиск
```
Ввод → Search Engine (SQLite FTS5 + граф связей)
     → Результаты: файлы, проекты, теги, контакты, история
     → Enter → открытие файла/проекта/контакта
```

#### 2. Заметка
```
Ввод → App: Notes (V8 Isolate)
     → Создание записи в SQLite: notes(id, content, project_id, tags, created_at)
     → Автоизвлечение тегов: #хештеги → Tag Engine
     → Обновление графа связей проекта
```

#### 3. Напоминание
```
Ввод → Intent Parser (Level 4, Semantic Kernel)
     → Извлечение datetime (duckling / rule-based)
     → Scheduler (Level 1) → SQLite: events(id, title, trigger_at, project_id)
     → В момент trigger → Push Notification (Level 3, Display Server)
```

#### 4. Калькулятор
```
Ввод → Math Parser (Level 1, встроенный модуль)
     → Результат: строка с красивой нотацией (MathML → WebGPU текст)
     → История: SQLite calculations(project_id, expression, result)
     → "Построить график" → App: Calculator (V8 Isolate, @workspace/graphics)
```

#### 5. Терминал
```
Ввод ($ или >) → Host Shim (Level 0, Rust)
               → PTY (псевдотерминал) → вывод в выпадающем списке
               → Или: создание окна терминала в проекте (Window Manager)
```

#### 6. Браузер
```
Ввод (URL) → Island Mode (Level 2, Chromium sandbox)
           → Создание web-окна в текущем проекте
           → Window Manager размещает как обычное окно
```

#### 7. Мессенджер
```
@ivan      → Contact Book (SQLite) → поиск
           → Выпадающий список: контакт + платформы + последние сообщения
           → Enter → Chat Engine (Level 1) → окно чата в проекте

email@domain → Email Engine (Level 1) → форма письма

+7999...   → Contact Book → мессенджер по умолчанию
           → Опция "Позвонить" (VoIP, WebRTC)
```

#### 8. ИИ-агент
```
Ввод → Intent API (Level 4)
     → Base Tier: rule-based NLP (16 MB RAM, всегда работает)
     → Full Tier: Semantic Kernel + embeddings (nomic-embed-text / bge-m3)
     → Intent Map → Action Executor
     → Результат: текст в строке / Generative UI / действие / TTS
```

### 1.5 Настройки строки

**Хранение:** SQLite → таблица `shell_settings` (Level 1).

```typescript
interface ShellSettings {
  bar_position: 'bottom' | 'top';
  bar_halign: 'center' | 'left' | 'right';
  bar_width: 'adaptive' | 'full' | number; // px
  bar_height: { min: number; max: number };
  bar_padding: { top: number; right: number; bottom: number; left: number };
  bar_margin: { top: number; right: number; bottom: number; left: number };
  bar_radius: number | { tl: number; tr: number; bl: number; br: number };
  bar_bg_color: string;
  bar_bg_image: string; // путь
  bar_opacity: number; // 0..1
}
```

**Рендеринг:** Universal Shell читает настройки при запуске и применяет к layout (Level 3, Display Server).

---

## 2. Intent API (Level 4)

### 2.1 Pipeline

```
[Whisper / Keyboard] → Intent Parser → Intent Resolver → Action Executor → Response Formatter
                                                            ↓
                                                    [Generative UI / Cloud Bridge / System Call]
```

### 2.2 Intent Parser

**Два тира:**

| Тир | Технология | Ресурсы | Когда работает |
|-----|-----------|---------|----------------|
| Base Tier | Rule-based + lightweight NLP + FTS5 | 16 MB RAM, no GPU | Всегда |
| Full Tier | Semantic Kernel + векторный поиск (embeddings) | GPU/NPU | Workspace.Mind включён |

**Словари Intents:** `"открой"`, `"найди"`, `"переключи"`, `"создай"`, `"отправь"`, `"покажи"`.

### 2.3 Intent Resolver

**Логика сопоставления:**
- Сопоставление с зарегистрированными Intents приложений (`os.mind.registerIntent`)
- Системные Intents: яркость, громкость, переключение профиля, скриншот, переключение Space
- **Неоднозначность:** если 2+ приложения регистрируют один Intent → выбор по приоритету (последнее использование, владелец проекта) или уточнение в Command Bar

### 2.4 Action Executor

**Каналы исполнения:**
1. **Intent API вызов:** `Workspace.Notes.create_note({ content, project })`
2. **Системный вызов:** изменение настроек, управление окнами
3. **Generative UI:** генерация JS-виджета в Level 5 sandbox → отрисовка через `@workspace/graphics`
4. **Cloud Bridge:** прокси к облачным LLM (OpenAI-compatible API)

### 2.5 Response Formatter

**Форматы ответа:**
- Текст в Command Bar
- TTS (Piper / Coqui, <100 мс)
- UI-виджет (временный, через `@workspace/graphics`)
- Безмолвное действие

### 2.6 Generative UI Engine

```
Intent → Semantic Kernel собирает данные
       → Генерация JS-кода в Level 5 sandbox
       → Отрисовка временного виджета через @workspace/graphics
       → Пример: "Покажи график трат на кофе за год"
```

### 2.7 Cloud Bridge

- **Протокол:** OpenAI-compatible API (OpenAI, Anthropic, GLM, Kimi, Gemini)
- **Explicit consent per request** (по умолчанию)
- **Prompt filtering:** DLP, удаление чувствительных данных перед отправкой
- **Аудит:** все облачные запросы логируются

### 2.8 Smart Scheduler

- Динамическое распределение AI-нагрузки между GPU / NPU / CPU
- Если запущена игра → снижение приоритета Whisper / SLM
- Переключение между CPU и NPU в зависимости от доступности

### 2.9 AI Event Bus

**События:**
- `ai.speech.recognized`
- `ai.intent.resolved`
- `ai.action.executed`
- `ai.cloud.requested`

### 2.10 Pipeline Hooks (точки расширения)

- `preIntentParse` — модификация текста перед парсингом
- `postIntentResolve` — обработка после определения Intent
- `preActionExecute` — проверка перед выполнением

### 2.11 Model Sandbox

- Whisper, SLM, TTS, Ollama — каждый в отдельном процессе или V8 Isolate
- Нет прямого доступа к VFS, SQLite, сети (кроме Cloud Bridge через прокси)
- Доступ только через `@workspace/*` wrappers

---

## 3. Voice Engine

### 3.1 Whisper (ASR)

- **Модель:** Whisper small / medium (зависит от железа)
- **Работает всегда**, даже в играх
- **WORKSPACE Pinning:** отдельное ядро CPU или NPU
- **Wake word:** `"WORKSPACE"` / `"Компьютер"` / настраиваемый (3–4 слога)
- **Exclusive Mode (игры):** 0% влияние на игру, результаты → только в наушники (TTS) или overlay
- **Privacy:** локально, данные не уходят

### 3.2 TTS Engine

- **Модель:** Piper / Coqui
- **Задержка:** <100 мс
- **Опциональный облачный fallback**

### 3.3 Zero UI

**Примеры команд:**
- `"Сделай музыку тише"` → `Workspace.Audio.setVolume(0.5)`
- `"Поставь будильник на 7"` → `Scheduler.setAlarm(7:00)`
- `"Скинь кадр Ване"` → Screenshot → Chat → отправка
- `"Что по проекту?"` → ИИ сводка → TTS в наушники

### 3.4 Intent Queue

```
Пользователь нажимает действие при CPU задушен до 400 МГц
  → Intent Queue принимает Intent = "saveProject(project_id=42)"
  → ACK: «Принято» (Static UI Overlay, checkmark)
  → Подписка: при освобождении CPU — Intent исполнится
  → Если Intent > 5 сек — progress bar
  → Если невозможен — TTS: "Не удалось сохранить, нужен интернет"
```

### 3.5 Безопасность голосового ввода

- **LED-индикатор:** Host Shim активирует LED при захвате микрофона
- **Audio buffer zeroize:** сырые samples не сохраняются на диск
- **Integrity check Whisper:** BLAKE3-хеш при загрузке модели
- **Opt-out:** Settings → Privacy → Voice Input
- **Speaker identity (опционально):** локальная проверка голоса для sensitive-команд

---

## 4. Communication Layer

### 4.1 Мессенджер

- **Контакты:** SQLite Contact Book (имя, ник, номер, email, платформы)
- **WORKSPACE-контакт:** P2P, CRDT, мгновенно, end-to-end (WireGuard)
- **Внешний контакт:** bridge (Telegram/WhatsApp API, email)
- **Шифрование:** end-to-end через WireGuard tunnel

### 4.2 Почта

- **Email Engine:** Level 1, SQLite-индексация входящих
- **SMTP/IMAP/POP3:** стандартные протоколы
- **Индексация:** входящие попадают в Search Engine (FTS5)

### 4.3 VoIP

- **WORKSPACE → WORKSPACE:** P2P VoIP (WebRTC через WireGuard)
- **Внешний:** SIP bridge / GSM через провайдера
- **Аудио:** CPAL (Level 0, Host Shim)

### 4.4 ИИ-мост

```
"Скинь документ Ивану"
  → Intent Parser: action="отправить", object="документ", recipient="Иван"
  → Search Engine → последний документ в проекте
  → Contact Book → Иван
  → Подтверждение: "Отправить 'смета.pdf' Ивану?"
  → Подтверждено → Chat Engine → отправка
```

### 4.5 Техподдержка

**Архитектура:**
1. Пользователь: `"техподдержка"` (голос / Command Bar)
2. Бэк открывает окно приёма (TTL 10 мин) + код `SUPPORT-9284-6173`
3. Relay-сервер WORKSPACE Corp (NAT traversal) → WireGuard-туннель до Бэка
4. Бэк проверяет: код валиден? подпись WORKSPACE Corp (Ed25519, HSM) верна?
5. Подтверждение пользователя: `"Подтверждённый специалист. Разрешить?"`
6. Сессия TTL 30 мин, логируется в аудит (категория "system")

**Уровни доступа:** Read-only (по умолчанию) / Diagnose / Full (требует dual approval + видеозапись).

---

## 5. Модель приложений (5 уровней интеграции)

### 5.1 Сравнение уровней

| Уровень | Название | Усилия | Runtime | Данные | Сеть | Риск |
|---------|----------|--------|---------|--------|------|------|
| 1 | «Как есть» | 0 | Island Mode (Chromium) | Cookies / storage | Свободно | Как браузер |
| 2 | «Манифест» | 5 мин | Island Mode (Chromium) | Cookies / storage | Свободно | Браузер + push |
| 3 | «Бэк на месте» | 1 день | Island + V8 Isolate (Bun) | App-scoped SQLite + VFS | Whitelist | Изолированный бэкенд |
| 4 | «@workspace/*» | 1 неделя | Island Mode | Через `@workspace/*` (capability) | Через `@workspace/network` | Только разрешённое |
| 5 | «Полный натив» | 1 месяц | V8 Isolate (Bun) | Через fs shim (capability) | Через `@workspace/network` | Только разрешённое |

### 5.2 Манифест `workspace.json` — полная схема

```json
{
  "name": "string (required)",
  "short_name": "string",
  "description": "string",
  "icon": "string (path or URL)",
  "version": "string (semver, required for levels 3-5)",
  "min_core_version": "string (semver range)",
  "update_url": "string (URL or git repo, required for levels 3-5)",
  "changelog_url": "string (URL)",

  "url": "string (level 1-2: URL сайта)",
  "display": "standalone | windowed (default: windowed)",
  "permissions": ["notifications", "fullscreen"],

  "frontend": "string (level 3+: путь к index.html или .bun)",
  "frontend_format": "static | bytecode (default: static)",
  "backend": "string (level 3: путь к server.js или server.bun)",
  "backend_format": "script | bytecode (default: script)",
  "port": "number (level 3: порт для бэкенда)",

  "entry": "string (level 5: точка входа нативного приложения или .bun)",
  "entry_format": "script | bytecode (default: script)",
  "mode": "island | native (default: island)",

  "bun_version": "string (semver range, для bytecode)",

  "network_whitelist": ["string[] (level 3: разрешённые домены. Пустой = доступ запрещён)"],

  "error_reporting_url": "string (URL, куда Workspace отправляет отчёты об ошибках)"
}
```

### 5.3 Permissions (уровни 2–5)

| Permission | Что даёт | Уровни | Спрашивает пользователя? |
|-----------|---------|--------|------------------------|
| `notifications` | Push-уведомления Workspace | 2–5 | Да, при первом запуске |
| `fullscreen` | Полноэкранный режим | 2–5 | Нет |
| `fs` | Доступ к файлам через `@workspace/fs` | 4–5 | Да |
| `contacts` | Доступ к контактам | 4–5 | Да |
| `network` | Сетевые запросы через `@workspace/network` | 4–5 | Да |
| `voice` | Голосовые команды | 5 | Да |
| `graphics` | Прямой доступ к WebGPU | 5 | Да |
| `secure_sign` | Биометрическая подпись | 4–5 | Да, каждый раз |

### 5.4 `@workspace/*` API (уровни 4–5)

```typescript
// Данные
import { readFile, writeFile, readdir } from 'fs'      // перехвачено → Back API
import { listNotes, createNote, updateNote } from '@workspace/notes'
import { addTag, removeTag, getTags } from '@workspace/tags'
import { search } from '@workspace/search'

// Контакты и коммуникации
import { findContact, listContacts } from '@workspace/contacts'
import { sendMessage } from '@workspace/messenger'

// Проекты и структура
import { getProject, listProjects } from '@workspace/projects'
import { createTask, listTasks } from '@workspace/tasks'

// Уведомления
import { notify } from '@workspace/notifications'

// App-scoped хранилище
import { db } from '@workspace/db'   // SQLite, CRDT-синхронизация

// Интеграция с системой
import { registerIntent } from '@workspace/intents'

// Уровень 5 — WebGPU + UI
import { drawRect, drawText, createTexture } from '@workspace/graphics'
import { pushLayout, flexColumn, flexRow } from '@workspace/layout'
import { Button, TextInput, List } from '@workspace/ui'
import { showDialog, showToast } from '@workspace/ui'
import { registerVoiceCommand } from '@workspace/voice'
import { onFrame } from '@workspace/loop'
import { getMetrics } from '@workspace/performance'
```

### 5.5 UI-фреймворк (уровень 5)

| Уровень | API | Назначение |
|---------|-----|-----------|
| A | `@workspace/graphics` | Прямой WebGPU. VR, 3D-редакторы, игры |
| B | `@workspace/shell` | Управление окнами, фокусом, слоями |
| C | `@workspace/ui` | Готовые компоненты (Button, TextInput, Card). Автоматическая адаптация под тему |

### 5.6 App Registry

**Хранение:**
```
~/.WORKSPACE/apps/
├── system/                        ← системные приложения (встроены)
├── installed/                     ← установленные приложения
│   ├── com.todoapp@1.2.0/
│   │   ├── workspace.json
│   │   ├── frontend/
│   │   ├── backend/               ← если level 3
│   │   ├── data.db                ← app-scoped SQLite
│   │   └── cache/                 ← кэш кода
├── catalog/                       ← кэш каталога приложений
└── pinned/                        ← закреплённые (level 1-2)
```

**Установка по уровням:**
| Уровень | Как устанавливается |
|---------|-------------------|
| 1 | Набрал URL → закрепил. Файлы не скачиваются |
| 2 | Набрал URL → Workspace нашёл `workspace.json` → добавил в каталог |
| 3–5 | Скачал из каталога / по адресу → код в `installed/` |

**Магазин:** `pkg.workspace.app` — подпись Ed25519, обновления автоматические.

### 5.7 App-scoped SQLite

```typescript
import { Database } from 'bun:sqlite'
const db = new Database('data.db')  // автоматически app-scoped
// Путь: ~/.WORKSPACE/apps/<app_id>/data.db
// Изолирована, синхронизируется через CRDT, бэкапится
```

### 5.8 Sandboxing — Capability-based Security

```typescript
const context = {
  fs: { read: ["/project/abc/**"], write: ["/project/abc/**"] },
  network: { domains: ["api.example.com"] },
  graphics: true,
  mind: true,
  contacts: false,
};
```

**Уровень 3 — особенности:**
- Сеть только по `network_whitelist`. Пустой массив = доступ запрещён.
- Firewall: только frontend Island может обращаться к `localhost:PORT` бэкенда.
- OAuth внешних сервисов — через системный Auth Proxy, не напрямую.
- Все HTTP-запросы логируются в аудит (категория «apps»).

### 5.9 V8 Bytecode (защита кода)

| Уровень | Что в bytecode |
|---------|---------------|
| 3 | Бэкенд (`server.bun`) |
| 4 | Весь код (`app.bun`) |
| 5 | Весь код (`app.bun`) |

```bash
bun build ./server.ts --bytecode --outfile server.bun
```

- Платформонезависимый
- Исходники не извлекаемы
- Поле `bun_version` в манифесте для проверки совместимости

### 5.10 Lifecycle приложения

```
Не запущено → Кэш (код на диске, ~0 RAM)
      ↓ Запуск
Активно → V8 Isolate, окно в проекте, полный доступ к квоте
      ↓ Сворачивание / потеря фокуса
Приостановлено → Isolate заморожен, память удерживается
      ↓ Закрытие
Уничтожено → Isolate убит, память освобождена, кэш кода остаётся
      ↓ Checkpoint каждые 5 сек
Warm Recovery → восстановление из checkpoint за <100 мс
```

### 5.11 Warm Recovery

**Checkpoint:**
- Каждые 5 сек (async, не блокирует UI)
- Сериализация JS state (user data, не DOM/V8 internals)
- Запись в SQLite: `app_id, checkpoint_blob, timestamp`
- Размер: типично 10–500 KB

**При сбое:**
- Host Shim обнаруживает crash (segfault, OOM kill)
- Micro-Kernel читает последний checkpoint
- Новый Isolate с тем же `app_id`
- Display Server: Static UI Overlay + progress (<100 мс)

**Что НЕ сохраняется:** WebGPU textures/buffers, network connections, таймеры/анимации.

### 5.12 Native Process Monitor

**Watchdog'и:**

| Тип | Условие | Действие |
|-----|---------|----------|
| Memory | >85% RAM хост-ОС или >95% RAM WORKSPACE | Graceful suspend → checkpoint → kill |
| CPU | Нет heartbeat 500 мс (Game Mode: 50 мс) или CPU >95% на ядре >5 сек | Kill + Static UI Overlay «Не отвечает» |
| GPU | GPU fence не возвращается >2 сек | GPU reset (TDR) + recovery |
| User | Command Bar → «задачи» → «завершить» или Panic Gesture | Мгновенный kill |

### 5.13 Secure Transaction API

```
Приложение запрашивает capability "secure_sign"
  → Пользователь подтверждает в Permissions UI
  → Workspace.Security.signWithBiometry({ data, key_id, biometry, confirmation_ui })
  → Micro-Kernel: проверка capability и key_id
  → Display Server: WYSIWYS overlay (рендерит система, не приложение)
  → Пользователь: biometric prompt
  → Host Shim (Secure Enclave / TPM): sign(data, private_key) → signature
  → Возвращается только signature (Ed25519)
```

**Rate limiting:** max 10 подписей/мин на приложение, max 100/час на профиль.

### 5.14 Security Hooks

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

### 5.15 Session Management

```rust
struct SessionConfig {
    ttl_minutes: u32,           // 30 по умолчанию
    auto_lock_after_idle: u32,  // 5 минут простоя
    require_biometry: bool,     // true для уровня «Повышенный»
}
```

- Проверка каждую минуту: idle > auto_lock → lock screen; total > ttl → logout
- Owner может принудительно завершить сессию через Workspace.Hardcore или другой Фронт
- Remote wipe: Бэк отзывает session token → push-уведомление → lock screen + очистка кэша

### 5.16 `workspace-dev` — Developer CLI

```bash
workspace-dev install <url|path>        # установка из Git / локальной директории
workspace-dev run <appId> --dev         # dev-режим (auto-reload)
workspace-dev logs <appId> --follow     # логи
workspace-dev build <appId> --output ./dist/   # сборка bytecode
workspace-dev validate workspace.json        # проверка манифеста
workspace-dev publish ./dist/ --registry pkg.workspace.app
workspace-dev check-updates / update / rollback
```

### 5.17 `@workspace/mock`

- npm-пакет `npm install @workspace/mock --save-dev`
- In-memory реализации `@workspace/*` API
- Тестирование вне Workspace (браузер / Node.js)
- Ограничения: нет CRDT, P2P, WebGPU, реальной безопасности

### 5.18 Window Injection & Env Injection

**Window Injection (`window.__WORKSPACE__`):**
- Inject-ится в Island Mode до загрузки страницы
- Поля: `version`, `level`, `appId`, `backend`, `db`, `theme`, `locale`

**Env Injection (`process.env.WORKSPACE*`):**
- `CORE_OS = 'true'`
- `workspace_version`, `CORE_OS_LEVEL`, `CORE_OS_APP_ID`
- `CORE_OS_DB_PATH` — путь к app-scoped SQLite
- `CORE_OS_VFS_PATH` — путь к app-scoped VFS
- `CORE_OS_PROFILE_ID`, `CORE_OS_SPACE_ID`

---

## 6. Project Manager

### 6.1 Жизненный цикл

**Создание:**
- Пользователь → "Новый проект" / кнопка ⊕ / голос
- SQLite: `projects(id, name, created_at, tags, icon, color)`
- Создаётся пустой layout → проект становится активным

**Переключение:**
- Текущий проект → заморозка (save state)
- Новый проект → загрузка state → рендеринг

**Удаление:**
- Мягкое удаление → архив
- Через 30 дней → физическое удаление (или мгновенное по подтверждению)

### 6.2 Session Persistence

```
Каждое изменение layout → Layout Store (SQLite):
  project_id, window_id, app_id, x, y, w, h, z_index, state, updated_at
  → CRDT Layer → синхронизация на другие устройства
```

**Чистая сессия (`ephemeral: true`):**
- При закрытии проекта все записи layout удаляются
- Приложения получают уведомление → очищают свои данные

### 6.3 Home Project

- Нулевой проект `id = "home"`. Всегда существует, не удаляется.
- Layout = пустой (только строка).
- Закреплённые проекты отображаются как иконки.
- Контекст по умолчанию для Command Bar.

### 6.4 Layout Engine

**Один экран:**
- Project Layout = tree of containers
- Split (horizontal/vertical) → Window (app instance)
- Tab Group → Window A, Window B

**Несколько экранов:**
- Screen Manager (Level 0, Host Shim)
- Monitor 1 → Project A, Monitor 2 → Project B
- Окно за край экрана → переброска на другой монитор

---

## 7. Window Manager

### 7.1 Z-индекс, фокус, перетаскивание

- **Z-стек:** список окон по порядку (фоновые → активное)
- **Фокус:** только одно окно получает keyboard input
- **Примагничивание (snap):** к краям, к другим окнам, к половинам экрана
- **Shadow State Recovery:** Shell может падать, но окна продолжают рендериться

### 7.2 Web-окна (Island Mode)

```
URL → App Runtime создаёт Island Process
    +-- Chromium sandbox (один на веб-окно)
    +-- Tabs: внутри веб-окна свои вкладки
    +-- Incognito: флаг при создании Island Process
    +-- DevTools: F12
```

### 7.3 Нативные окна (WebGPU)

```
Приложение (V8 Isolate) → Workspace.Graphics API (Level 3)
                        → WebGPU Pipeline
                        → Display Server композитит
                        → Effects: blur, transparency, shadows
```

### 7.4 Remote Renderer

```
Бэк (V8 Isolate + WebGPU) → Рендеринг в offscreen texture
                          → Кодирование в video stream (H.264/AV1)
                          → TLS-шифрованный bitmap-стрим → Фронт
Фронт → Декодирование → отображение как обычное окно
      → Ввод → обратно на Бэк
```

### 7.5 Static UI Overlay

```
Пользователь нажимает кнопку
  → Display Server проверяет: V8 Isolate готов? CPU > порога?
    +-- Да → живой рендеринг через WebGPU
    +-- Нет → Static UI Overlay:
        +-- Последний snapshot интерфейса (bitmap в GPU memory)
        +-- Активные зоны (hotspots) для тач/клика
        +-- Действие → Intent Queue
        +-- Как только V8 «продышится» — очередь исполняется
```

### 7.6 Game Mode API

**Условия активации:**
- Приложение level >= 4 (verified publisher)
- Пользователь подтвердил в Permissions UI

**Что активируется:**

| Компонент | Что происходит |
|-----------|---------------|
| Direct GPU Context | Display Server отдаёт swapchain напрямую приложению (без композитинга) |
| Raw Input | Host Shim перенаправляет HID events напрямую в Isolate (без Input Router). Исключение: Panic Gesture |
| Low-latency Audio | CPAL REALTIME (минимальный буфер, exclusive stream) |
| CPU Affinity | Game thread pinned на dedicated WORKSPACE. SCHED_RR + high priority |
| Network Overlay | UDP-based real-time socket. DTLS шифрование |

**Frame Pacing:** target FPS (60/120/144/240). FIFO_RELAXED для VRR. Auto resolution scaling (0.5–1.0).

**Переключение контекста (Alt+Tab):**
- Display Server → shadow state (framebuffer в dedicated GPU memory)
- Isolate приостанавливается (freeze), checkpoint сохраняется
- Обратное переключение: resume isolate (<100 мс)

**Время:** Shell → Game Mode: 16–33 мс. Game Mode → Shell: мгновенно. Обратно: <100 мс.

**Выход:** Esc (2 сек), Panic Gesture, Game crashed → Shadow State Recovery → Static UI Overlay.

### 7.7 Optimistic Rendering

```
Пользователь ввёл текст / переместил окно / изменил файл
  → Display Server применяет локально и мгновенно (0ms feedback)
  → Отправляет мутацию в CRDT Layer → Sync Engine
Если прилетает remote winner (Hash-based Ordering):
  → UI НЕ откатывает состояние назад
  → Показывает индикатор: «Обновлено на другом устройстве»
  → Плавно применяет winner через 300ms fade / blink
```

---

## 8. Система настроек

### 8.1 Хранилища и приоритет

| Хранилище | Что там | Кто управляет | Формат |
|-----------|---------|---------------|--------|
| **SQLite (Фронт)** | Command Bar, темы, горячие клавиши, приоритеты уведомлений, accessibility | Пользователь | SQLite |
| **SQLite (Бэк)** | VFS (passport, tags, versions), RBAC, Audit Store, User Directory, Device Registry | Owner / система | SQLite |
| **JSON (Бэк)** | AI-модели, Cloud Bridge, backup targets, политики безопасности, компоненты Бэка | Owner / админ | JSON |
| **JSON (манифест приложения)** | `workspace.json`: permissions, network_whitelist, update_url | Разработчик | JSON |
| **TPM / Secure Enclave** | Master key, device key, biometric data | Система (Key Manager) | Аппаратный |
| **RAM (не сохраняется)** | Анонимный профиль, аудио-буфер Whisper | Система | RAM |

**Приоритет настроек (input mapping, горячие клавиши):**
1. Корпоративный конфиг Бэка (переопределяет всё)
2. Пользовательский конфиг профиля
3. Значения по умолчанию

### 8.2 Command Bar — настройки

| Поле | Тип | Диапазон |
|------|-----|----------|
| `bar_position` | enum | `"bottom"` \| `"top"` |
| `bar_halign` | enum | `"center"` \| `"left"` \| `"right"` |
| `bar_width` | union | `"adaptive"` \| `"full"` \| `number` (px) |
| `bar_height` | object | `{ min: number; max: number }` |
| `bar_padding` | object | `{ top, right, bottom, left: number }` |
| `bar_margin` | object | `{ top, right, bottom, left: number }` |
| `bar_radius` | union | `number` \| `{ tl, tr, bl, br: number }` |
| `bar_bg_color` | string | CSS color |
| `bar_bg_image` | string | путь к изображению |
| `bar_opacity` | number | `0..1` |

**Интерфейс:** GUI → Настройки → Command Bar.

### 8.3 Space — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Создание / подключение / отключение | Бэк (SQLite + mesh-реестр) | Command Bar / QR-сканер / GUI |
| Размещение (плитки, сплит, мониторы) | SQLite (layout) | GUI (drag-and-drop) |
| Фон (цвет, градиент, паттерн, изображение) | SQLite (настройки Space) | GUI → Настройки Space |
| Приоритет уведомлений (прерывающие / тихие / молчащие) | SQLite (пользовательские приоритеты) | GUI → Настройки уведомлений |

### 8.4 Профили — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Переключение между профилями | Бэк (Profile Manager) | Экран входа / GUI |
| Создание профиля (имя, recovery-фраза BIP-39) | Бэк (Key Manager + Profile Manager) | GUI → Настройки → Пользователи |
| Анонимный профиль | RAM (не сохраняется) | Экран входа → «Анонимный» / Command Bar |
| Auto-lock (блокировка после N минут простоя) | SQLite (настройки пользователя) | GUI → Настройки → Безопасность |
| Screen lock + биометрия/PIN | SQLite / TPM / Secure Enclave | Экран входа |
| Remote wipe (удалённое уничтожение кэша) | Бэк (mesh-реестр device keys) | Workspace.Backoffice / Workspace.Hardcore |
| TTL сессии (30 мин бездействия → logout) | Политика Бэка | Workspace.Hardcore |

### 8.5 Приложения — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Permissions (файлы, сеть, контакты, уведомления, микрофон, камера, `secure_sign`) | Манифест `workspace.json` + Capability Store (Бэк) | Permissions UI (всплывающее окно) / ПКМ на окно / Command Bar |
| Блокировка bytecode-приложений (`allow_bytecode_apps`) | Политика Бэка | Workspace.Backoffice / Workspace.Hardcore |
| Автообновление приложений | SQLite (настройки пользователя) | GUI → Настройки приложений |
| Отчёты об ошибках (разрешить/запретить `message`) | SQLite (настройки пользователя) | Диалог ошибки / Command Bar |

### 8.6 Уведомления — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Приоритеты Space (прерывающие / тихие / молчащие) | SQLite (Фронт) | GUI → Настройки → Уведомления |
| Игровой режим (молчащие уведомления) | Runtime (Game Mode API) | Command Bar → «игровой режим» / горячая клавиша |
| История уведомлений | SQLite (local) | Command Bar → «уведомления» / жест свайп сверху |

### 8.7 Безопасность — настройки

| Что настраивается | Где хранится | Кто меняет | Интерфейс |
|-------------------|--------------|------------|-----------|
| Уровень безопасности (Базовый / Повышенный / Максимальный) | Бэк (Key Manager + конфиг) | Owner | Workspace.Backoffice / Workspace.Hardcore |
| `external_media_allow_export` (запрет экспорта на USB) | Политика Бэка (JSON/SQLite) | Owner | Workspace.Backoffice / Workspace.Hardcore |
| `require_encrypt_export` (шифрование при экспорте) | Политика Бэка | Owner | Workspace.Backoffice / Workspace.Hardcore |
| `allow_import` / `max_import_files` / `max_import_file_size` | Политика Бэка | Owner | Workspace.Hardcore |
| `clipboard_allow_export` (`none` / `text` / `all`) | Политика Бэка | Owner | Workspace.Backoffice / Workspace.Hardcore |
| `clipboard_life_time`, `clipboard_persist` | Политика Бэка | Owner | Workspace.Backoffice / Workspace.Hardcore |
| Recovery-фраза (24 слова BIP-39) | Бумага / YubiKey (вне системы) | Пользователь (Owner) | GUI при первом запуске |

### 8.8 Синхронизация и бэкап — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| P2P Mesh (включение/отключение компонентов) | Бэк (JSON-конфиг) | Workspace.Hardcore / Command Bar |
| Seeding (смена primary Бэка) | CRDT + mesh-реестр | Command Bar → «Бэки» → «Сделать primary» |
| Бэкап: targets (USB, S3, SFTP, другой Бэк, Custom) | JSON-конфиг Бэка | Workspace.Backoffice / Workspace.Hardcore / Command Bar |
| Бэкап: расписание (`hourly` / `daily` / `weekly`) | JSON-конфиг Бэка | GUI / CLI |
| Бэкап: retention (сколько копий, default 7) | JSON-конфиг Бэка | GUI / CLI |
| Бэкап: warnings (`no_backup_days`, `low_space_mb`) | JSON-конфиг Бэка | GUI / CLI |
| Шифрование бэкапа (backup key из recovery-фразы) | JSON-конфиг | GUI / CLI |
| Verified Content Seeding (opt-in, max 10% канала) | SQLite (пользовательские настройки) | GUI → Статус сети |

### 8.9 Темы и оболочки — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Системная тема / стиль (магазин стилей) | SQLite / App Registry | GUI → Настройки → Темы |
| Альтернативная оболочка (Shell) | App Registry | GUI → Настройки → Оболочки |
| Высокий контраст | SQLite (доступность) | GUI → Доступность / горячая клавиша |
| Крупный текст | SQLite (доступность) | GUI → Доступность |
| Уменьшенная анимация | SQLite (доступность) | GUI → Доступность |

### 8.10 Голос и AI — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Workspace.Mind: включён / выключен | SQLite / JSON (`workspace.json`) | GUI → System Settings → AI |
| Режим работы (`local` / `hybrid-consent` / `hybrid-auto` / `cloud-only`) | SQLite / JSON | GUI → AI |
| ASR-модель (Whisper base / medium / large) | JSON (`ai.asr.model`) | GUI → AI |
| NLU-модель (Llama 3.1 8B, Phi-4 и т.д.) | JSON (`ai.nlu.model`) | GUI → AI |
| Embeddings-модель (nomic, bge-m3) | JSON | GUI → AI |
| TTS (Piper ru_RU / en_US / zh_CN) | JSON (`ai.tts.voice`) | GUI → AI |
| Cloud LLM провайдер (OpenAI, Anthropic, GLM, Kimi) | JSON (`ai.cloud_bridge.providers`) | GUI → AI |
| Ограничение RAM/VRAM для AI | JSON (`ai.scheduler`) | GUI → AI |
| Язык голосового ввода | SQLite / JSON | GUI → AI |
| Wake-word («WORKSPACE» или кастомное) | SQLite / JSON | GUI → AI |
| Чувствительность wake-word | SQLite / JSON | GUI → AI |
| Push-to-talk (клавиша / геймпад / наушники) | SQLite / JSON | GUI → AI |
| Отключение AI по приложению (`os.mind.optOut()`) | Runtime (V8 Isolate) | Код приложения |
| RBAC для AI (`ai:use_local`, `ai:use_cloud`, `ai:configure`, `ai:load_custom_model`) | RBAC Engine (Бэк) | Workspace.Backoffice / Workspace.Hardcore |

### 8.11 Устройства — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| USB: автоматические правила по метке тома (импорт / бэкап / игнорировать) | SQLite (`Device Registry` → `rules`) | GUI / Command Bar |
| Дедупликация при импорте | SQLite (пользовательские настройки) | GUI |
| Автоматические теги при импорте | SQLite (пользовательские настройки) | GUI |
| Индексация после импорта | SQLite (пользовательские настройки) | GUI |
| Уведомление о носителе | SQLite (пользовательские настройки) | GUI |
| Mirror Folders (привязка папки хост-ОС к проекту) | SQLite (`Mirror Engine` → `watched_paths`) | ПКМ на проект → «Подключить папку хост-ОС» |
| Печать / сканирование | Хост-ОС (CUPS/TWAIN) | Command Bar → «напечатать» / «отсканировать» |

### 8.12 Energy Manager — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Пороги автоэкономии (<20% — фоновый sync off, <10% — только приоритетные) | Политика Бэка (Energy Manager) | Статус-бар → диалог |
| Ночной режим (фоновые задачи: 23:00–07:00) | Политика Бэка (Scheduler) | Автоматически |
| Background sync только по Wi-Fi | SQLite (настройки пользователя) | GUI |

### 8.13 Accessibility — настройки

| Что настраивается | Где хранится | Интерфейс |
|-------------------|--------------|-----------|
| Screen reader | SQLite (настройки) | GUI → Доступность |
| Высокий контраст | SQLite (настройки) | GUI → Доступность / горячая клавиша |
| Крупный текст | SQLite (настройки) | GUI → Доступность |
| Уменьшенная анимация | SQLite (настройки) | GUI → Доступность |
| Полная навигация с клавиатуры | SQLite (настройки) | GUI → Доступность |
| Голосовое управление | AI-настройки | GUI → AI |

### 8.14 Input Mapping (горячие клавиши, жесты)

| Что настраивается | Где хранится | Кто меняет | Интерфейс |
|-------------------|--------------|------------|-----------|
| Горячие клавиши (любое сочетание для любого действия) | SQLite (пользовательский конфиг профиля) | Пользователь | GUI → Настройки → Горячие клавиши |
| Жесты (тачпад, тачскрин) | SQLite (пользовательский конфиг профиля) | Пользователь | GUI → Настройки → Горячие клавиши (запись жеста) |
| Корпоративный input mapping | JSON / SQLite (конфиг Бэка) | Админ | Workspace.Backoffice → политики → input mapping / Workspace.Hardcore: `workspace-cli settings input set --action ... --key ...` |
| Game Mode: горячая клавиша входа/выхода | SQLite (пользовательский конфиг) | Пользователь | GUI |

**Действия по умолчанию:**

| Действие | Дефолт | Настраивается? |
|----------|--------|----------------|
| Переключение окон (внутри проекта) | Alt+Tab / Cmd+Tab | Да |
| Переключение проектов | Ctrl+~ | Да |
| Переключение Space | Ctrl+Shift+~ | Да |
| Сворачивание окна | Свайп вниз от верхнего края | Да |
| Закрытие окна | Свайп вверх от нижнего края / крестик | Да |
| Panic Gesture | Тройное касание в углу / Ctrl+Shift+Esc | Нет (железо) |
| Game Mode | Command Bar / горячая клавиша | Да |

---

## 9. Администрирование

### 9.1 Workspace.Backoffice (GUI)

- GUI-приложение внутри WORKSPACE. Работает через Фронт, подключённый к Бэку.
- Предустановлено в домашней конфигурации.
- **Корпоративный режим:** приложение не устанавливается. Флаг `allow_gui_admin: false` — Бэк отказывается запускать даже при ручной загрузке.

**Доступно в режимах:**

| Режим | Workspace.Backoffice | Workspace.Hardcore |
|-------|-----------------|---------------|
| Домашний | Да | Да |
| Малая команда | Да | Да |
| Корпоративный | Заблокирован | Единственный способ |

### 9.2 Workspace.Hardcore (TUI + CLI)

- TUI-приложение прямо на Бэке. Подключение по SSH.
- Без Фронта, без GUI, без WebGPU.
- Для: корпоративного сегмента, первичной настройки сервера, аварийного восстановления, автоматизации (SSH + CLI = скрипты, CI/CD).

**Примеры команд:**
```bash
ssh admin@WORKSPACE-server -- workspace-cli user add --name "Иван" --role developer
ssh admin@WORKSPACE-server -- workspace-cli app deploy --file crm.pkg --role all
ssh admin@WORKSPACE-server -- workspace-cli backup --full --output /mnt/backup/
ssh admin@WORKSPACE-server -- workspace-cli settings input set --action "switch-space" --key "Ctrl+Shift+Tab"
ssh admin@WORKSPACE-server -- workspace-cli directory set --type ldap --url ldap://corp.local
ssh admin@WORKSPACE-server -- workspace-cli directory map --ldap-group "Engineers" --role "developer"
```

### 9.3 Бэк-конфигурация

**Профиль Бэка (при установке):**
| Профиль | Компоненты |
|---------|-----------|
| Минимальный | SQLite, CRDT, Key Manager, Auth Proxy |
| Сбалансированный | + Chat, Scheduler, Search, P2P-клиент |
| Полный | + VoIP-сервер, App Registry, Tag Engine, Sync Engine, Dev Tools |

**JSON-конфиг Бэка:**
```json
{
  "profile": "balanced",
  "components": {
    "chat": true,
    "scheduler": true,
    "search": true,
    "p2p_server": false,
    "voip": true,
    "app_registry": true,
    "tag_engine": true,
    "sync_engine": true,
    "auth_proxy": true,
    "dev_tools": false
  },
  "allow_gui_admin": true,
  "allow_anonymous_connect": false,
  "security_level": "enhanced",
  "audit": {
    "categories": ["auth", "roles", "projects", "files", "apps", "system"],
    "retention_days": 90,
    "max_size_mb": 1024
  },
  "backup": {
    "targets": ["usb", "s3"],
    "schedule": "daily",
    "retention": 7,
    "warnings": {
      "no_backup_days": 3,
      "low_space_mb": 1024
    }
  },
  "ai": {
    "asr": { "model": "whisper-medium" },
    "nlu": { "model": "llama-3.1-8b" },
    "tts": { "voice": "piper-ru_RU" },
    "cloud_bridge": {
      "providers": ["openai", "anthropic"],
      "explicit_consent": true
    },
    "scheduler": { "max_ram_mb": 2048, "max_vram_mb": 4096 }
  }
}
```

### 9.4 RBAC

**Встроенные роли:**
| Роль | Права |
|------|-------|
| Owner (root) | Полный контроль |
| Member | Чтение/запись |
| Guest | Только чтение |

**Кастомные роли:** Owner создаёт через Workspace.Backoffice / Workspace.Hardcore.

**Категории аудита (13 штук):**
`auth`, `roles`, `projects`, `files`, `notes`, `tags`, `messenger`, `search`, `apps`, `browser`, `profiles`, `system`, `multi-back`.

---

## 10. Горячие клавиши и системные команды

### 10.1 Навигация

| Действие | Десктоп (Windows/Linux) | macOS | Тачпад | Телефон |
|----------|------------------------|-------|--------|---------|
| Переключение окон (внутри проекта) | `Alt+Tab` | `Cmd+Tab` | 3 пальца влево/вправо | Свайп по нижней грани |
| Переключение проектов | `Ctrl+~` | `Cmd+~` | 4 пальца влево/вправо | Жест «два пальца вверх» |
| Переключение Space | `Ctrl+Shift+~` | `Cmd+Shift+~` | 4 пальца вверх (обзор) | Жест «два пальца вбок» |
| Home Project (главный экран) | `Ctrl+Home` | `Cmd+Home` | 5 пальцев схватить | Кнопка «Домой» |
| Обзор всех Space | `Ctrl+Shift+Up` | `Cmd+Shift+Up` | 4 пальца вверх | Свайп вверх и удержать |

### 10.2 Управление окнами

| Действие | Десктоп | macOS | Тачпад | Телефон |
|----------|---------|-------|--------|---------|
| Сворачивание окна | `Ctrl+M` | `Cmd+M` | Свайп вниз от верхнего края | Свайп вниз от верхнего края |
| Закрытие окна | `Ctrl+W` | `Cmd+W` | Свайп вверх от нижнего края | Свайп вверх от нижнего края |
| Полноэкранный режим | `F11` | `Ctrl+Cmd+F` | Двойное касание заголовка | Поворот устройства |
| Выход из полноэкрана | `Esc` | `Esc` | Жест «назад» | Кнопка «Назад» |
| Snap (примагничивание) | `Win+←/→/↑/↓` | нет | Перетаскивание к краю | Нет |

### 10.3 Command Bar

| Действие | Десктоп / macOS |
|----------|-----------------|
| Фокус на строку | `Ctrl+Space` / `Cmd+Space` |
| Переключение режима вручную | `Tab` или `1`–`8` |
| Очистить строку | `Esc` (когда строка в фокусе) |
| Выполнить (Enter) | `Enter` |
| Выбрать подсказку (стрелки) | `↑` / `↓` |
| Открыть настройки строки | `Ctrl+,` (когда строка в фокусе) |

### 10.4 Game Mode

| Действие | Десктоп / macOS |
|----------|-----------------|
| Включить/выключить Game Mode | Настраиваемая клавиша (дефолт: `Ctrl+Shift+G`) |
| Выход из Game Mode | `Esc` (удержание 2 сек) |
| Panic Gesture | `Ctrl+Shift+Esc` или тройное касание в углу |

### 10.5 Системные команды (голос / Command Bar)

| Команда | Действие | Компонент |
|---------|----------|-----------|
| `"техподдержка"` | Открыть окно приёма, сгенерировать код | Tech Support Engine |
| `"проверь почту"` | Показать непрочитанные в строке | Email Engine |
| `"скинь [файл] [контакту]"` | ИИ-мост: найти файл → найти контакт → отправить | Intent API + Chat Engine |
| `"покажи график [данных]"` | Генеративный UI: построить график | Generative UI Engine |
| `"сделай музыку тише"` | `Workspace.Audio.setVolume(0.5)` | Zero UI |
| `"поставь будильник на [время]"` | `Scheduler.setAlarm(time)` | Zero UI |
| `"что по проекту?"` | ИИ-сводка → TTS в наушники | Zero UI + TTS |
| `"screenshot"` / `"скриншот"` | Сделать скриншот текущего окна | System Intent |
| `"lock"` / `"заблокируй"` | Блокировка экрана | Session Manager |
| `"logout"` / `"выйди"` | Завершение сессии | Session Manager |

### 10.6 Accessibility

| Действие | Клавиша |
|----------|---------|
| Включить/выключить screen reader | `Ctrl+Alt+S` |
| Высокий контраст | `Ctrl+Alt+C` |
| Крупный текст | `Ctrl+Alt++` (увеличить), `Ctrl+Alt+-` (уменьшить) |
| Уменьшенная анимация | `Ctrl+Alt+A` |
| Полная навигация табом | Всегда активна |

### 10.7 Panic Gesture (аварийный выход)

| Условие | Действие |
|---------|----------|
| Тройное касание любого угла экрана | Мгновенный выход из WORKSPACE в хост-ОС |
| `Ctrl+Shift+Esc` (дефолт) | То же самое |
| Длинное нажатие питания (>3 сек) | Меню хост-ОС |

Panic Gesture ловится на уровне Host Shim (Level 0) — не блокируется зависшими приложениями.

---

## 11. Описание интерфейса

### 11.1 Экран входа (Lock Screen)

**Элементы:**
- Список профилей (аватар, имя, последняя активность)
- «Анонимный» — временный профиль без сохранения
- Поле биометрии / PIN (для уровней «Повышенный» и «Максимальный»)
- Recovery-фраза (при первом запуске или восстановлении)
- Статус сети (онлайн / оффлайн / синхронизация)

**Состояния:**
- Auto-lock после N минут простоя
- Remote wipe: при получении команды от Бэка — экран блокируется, кэш очищается

### 11.2 Рабочий стол проекта

**Структура:**
```
+----------------------------------------------------------+
| [Space A] [Space B]            [Статус] [Батарея] [Время] |
+----------------------------------------------------------+
|                                                          |
|  +-------------------+  +-------------------+            |
|  | Окно приложения A |  | Окно приложения B |            |
|  |                   |  |                   |            |
|  +-------------------+  +-------------------+            |
|                                                          |
|  [Плашки свёрнутых окон]                                 |
|                                                          |
|  +----------------------------------------------------+  |
|  | [🔍] [📝] [⏰] ...  Command Bar  ...  [микрофон]   |  |
|  +----------------------------------------------------+  |
+----------------------------------------------------------+
```

**Элементы:**
- **Space Tabs** — переключение между Space (цветные индикаторы)
- **Status Bar** — сеть, синхронизация, батарея, режим питания
- **Window Area** — окна приложений в текущем layout (тайловый или плавающий)
- **Minimized Bar** — плашки свёрнутых окон внизу проекта
- **Command Bar** — строка ввода (позиция настраивается: bottom/top, center/left/right)

### 11.3 Окно приложения

**Элементы:**
- **Заголовок** — имя приложения, иконка, кнопки (свернуть, закрыть)
- **Рамка** — для resize (drag edges/corners)
- **Контент** — Island Mode (Chromium) или Native (WebGPU)
- **Контекстное меню** — ПКМ: «Права доступа», «Закрепить», «Полноэкранный», «Game Mode»

**Состояния:**
- Активное (фокус, полная яркость)
- Неактивное (без фокуса, слегка приглушено)
- Свернутое (плашка в Minimized Bar)
- Полноэкранное (без рамок, Command Bar свёрнута в индикатор)
- Game Mode (direct GPU, raw input, без композиции)

### 11.4 Центр уведомлений

**Вызов:** Command Bar → «уведомления» или свайп с верхнего края.

**Структура:**
- Группировка по Space (цветные маркеры)
- Группировка по времени (сегодня, вчера, ранее)
- Действия: очистить по Space, очистить все, перейти к источнику
- Непрочитанные — точка, исчезает после перехода

### 11.5 Permissions UI

**Вызов:** ПКМ на окно → «Права доступа» или Command Bar → «права приложений».

**Структура:**
- Список приложений с иконками
- Для каждого приложения — список capabilities (переключатели)
- `fs` — раскрывается в дерево папок/файлов (scope)
- `network` — раскрывается в список доменов (whitelist)
- Запрос нового разрешения — модальное окно: «Разрешить / Запретить / Разрешить один раз»

### 11.6 Настройки (GUI)

**Структура:**
```
Настройки
├── Command Bar
│   ├── Позиция, размер, цвет, прозрачность
│   └── Шрифт, скругление, фон
├── Space
│   ├── Уведомления (приоритеты по Space)
│   └── Внешний вид (фон, разделители)
├── Профиль
│   ├── Безопасность (auto-lock, биометрия)
│   └── Сессии (активные устройства, remote wipe)
├── Приложения
│   ├── Список установленных
│   ├── Права доступа
│   └── Автообновление
├── AI
│   ├── Workspace.Mind (вкл/выкл)
│   ├── Режим (local / hybrid / cloud)
│   ├── Модели (ASR, NLU, TTS)
│   └── Cloud Bridge (провайдеры, consent)
├── Устройства
│   ├── USB-правила
│   ├── Mirror Folders
│   └── Печать
├── Доступность
│   ├── Screen reader, контраст, крупный текст
│   └── Уменьшенная анимация, клавиатурная навигация
├── Горячие клавиши
│   ├── Список всех действий
│   └── Запись нового сочетания / жеста
└── О системе
    ├── Версия Workspace
    ├── Статус синхронизации
    └── Бэкап (восстановление, расписание)
```

### 11.7 Workspace.Backoffice (GUI администрирования)

**Доступ:** Command Bar → «администрирование» (только для Owner / админов).

**Структура:**
```
Backoffice
├── Пользователи
│   ├── Список (создание, редактирование, удаление)
│   ├── Роли (Owner / Member / Guest / кастомные)
│   └── Группы
├── Space
│   ├── Подключённые устройства
│   └── Политики (anonymous connect, clipboard, import/export)
├── Приложения
│   ├── App Registry (корпоративные приложения)
│   └── Развёртывание (deploy, rollback)
├── Безопасность
│   ├── Уровень безопасности
│   ├── Аудит (13 категорий, фильтры, экспорт)
│   └── Input Mapping (корпоративные горячие клавиши)
├── Бэкап
│   ├── Targets (USB, S3, SFTP, другой Бэк)
│   ├── Расписание и retention
│   └── Восстановление
├── AI
│   ├── Разрешённые модели
│   ├── RBAC для AI (use_local, use_cloud, configure)
│   └── Cloud Bridge (провайдеры, лимиты)
└── Техподдержка
    ├── Уровни доступа (Read-only / Diagnose / Full)
    ├── Whitelist IP
    └── История сессий
```

### 11.8 Workspace.Hardcore (TUI)

**Подключение:** `ssh admin@WORKSPACE-server`.

**Интерфейс:** TUI на базе `ratatui` (Rust) или аналог. Меню навигации стрелками, формы ввода, таблицы.

**Разделы:**
- `users` — управление пользователями
- `apps` — развёртывание приложений
- `backup` — запуск и восстановление
- `settings` — все JSON-конфиги Бэка
- `logs` — просмотр audit log
- `network` — P2P, WireGuard, мосты
- `update` — обновление Бэка

---

## 12. Cross-Reference к слоям

| Раздел справочника | Основной слой | Дополнительные слои |
|--------------------|---------------|---------------------|
| Command Bar | layer-8 §1 | layer-1 (UX), layer-2 (AI) |
| Intent API | layer-8 §7.5–7.15 | layer-2 (AI), layer-3 (System Split) |
| Voice Engine | layer-8 §7 | layer-1 (UX), layer-7 (Security) |
| Communication Layer | layer-8 §6 | layer-1 (UX), layer-7 (Security) |
| Модель приложений | layer-6 | layer-8 §4, layer-7 (Security) |
| Project Manager | layer-8 §2 | layer-1 (UX) |
| Window Manager | layer-8 §3 | layer-1 (UX), layer-7 (Security) |
| Система настроек | layer-1, layer-8 | layer-3 (System Split), layer-4 (Install) |
| Администрирование | layer-3, layer-8 | layer-4 (Install), layer-7 (Security) |

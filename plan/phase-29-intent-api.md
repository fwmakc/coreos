# Этап 29 — Intent API & AI WORKSPACE

## Цель
Создать систему Intent — обработку намерений пользователя: парсинг текста/голоса, разрешение в действия, Generative UI, Smart Scheduler, и Cloud Bridge для внешних AI. После этого этапа Workspace понимает, чего хочет пользователь, и выполняет или генерирует интерфейс для этого.

## Язык и стек
- **Язык:** TypeScript
- **Runtime:** Bun
- **Ключевые зависимости:** `fuse.js` (fuzzy matching для Intent resolution), ONNX Runtime (для локального SLM, опционально), OpenAI-compatible API client (для Cloud Bridge)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 12** — Micro-Kernel: WORKSPACE (SQLite, event loop).
- **Этап 15** — Command Bar (текстовый ввод, режим Ask).
- **Этап 28** — Voice Pipeline (ASR output → текст для Intent).
- **Этап 23** — V8 Isolate Runtime (Generative UI рендерится через `@workspace/ui`).

## Часть системы
**Level 4 — Intent API** [См. layer-8 §7.1–7.12, layer-2 §2, layer-1 §2.4]

## Требования

### 25.1 Intent Pipeline
- **Input:** текст из Command Bar, Voice Pipeline, или приложения.
- **Parser:**
  - Rule-based: регулярные выражения и шаблоны для типовых команд ("открой X", "найди Y", "создай Z").
  - SLM (Small Language Model): локальная модель (Llama 3.1 8B, Phi-3, или Qwen2 7B через ONNX) для сложных запросов.
  - **Fallback:** если SLM не справляется — Cloud Bridge (этап 25.6).
- **Intent Structure:**
  ```
  {
    action: "app.open" | "fs.search" | "project.create" | "ui.generate" | ...,
    target: "notes" | "file.txt" | null,
    params: { ... },
    confidence: 0.0–1.0,
    source: "text" | "voice" | "ai"
  }
  ```
- **Resolver:** Intent сопоставляется с зарегистрированными обработчиками (handler registry).
  - Каждое приложение и системный модуль регистрирует свои handlers через `IntentRegistry.register(action, handler)`.
  - Если несколько handlers — выбор по приоритету и контексту.
- **Executor:** вызов handler с params и context. Handler может быть:
  - Синхронный (мгновенный, < 10 мс).
  - Асинхронный (запускает долгую операцию, возвращает promise).
  - Генеративный (создаёт UI).

### 25.2 Command Bar Integration
- **Режим Ask (`?`):** пользователь вводит вопрос → Parser → Intent → Executor.
  - "какая погода?" → `web.search: "погода"` → результат в Command Bar suggestions.
  - "напиши письмо Ивану о встрече" → `email.compose: { to: "Иван", subject: "Встреча", body: "..." }`.
- **Режим Script (`$`):** JavaScript код исполняется в sandbox. Код имеет доступ к `WORKSPACE.intent.emit()` для системных вызовов.

### 25.3 Generative UI
- **Сценарий:** пользователь просит "создай форму для регистрации" — AI генерирует React-подобный компонент, который рендерится через `@workspace/ui`.
- **Codegen:** SLM генерирует код на TypeScript (подмножество `@workspace/ui` API).
- **Sandbox:** сгенерированный код выполняется в отдельном V8 Isolate с ограниченными capabilities (только `ui:render`, нет `fs:write` или `network:http`).
- **Preview:** сгенерированный UI отображается в окне с кнопками "Применить" (добавить в проект) и "Изменить" (редактировать промпт).
- **Human-in-the-loop:** пользователь может редактировать сгенерированный код перед применением.

### 25.4 Smart Scheduler
- **Задача:** AI-распределение ресурсов CPU/GPU/времени.
- **Входные данные:**
  - Список запущенных приложений и их приоритеты.
  - CPU/GPU load (от Host Shim).
  - Батарея (от Energy Manager, этап 27).
  - Предстоящие события (календарь, reminders).
- **Выход:** план выполнения задач.
  - Например: "Рендеринг видео — фоновая задача, запустить когда CPU < 30%".
  - "Backup — ночью, когда устройство на зарядке".
- **API:** `WORKSPACE.scheduler.schedule(task, constraints)`.

### 25.5 Semantic Search (Local)
- **Embeddings:** локальная модель `nomic-embed-text` или `bge-m3` (ONNX, ~100 МБ).
- **Index:** векторный индекс для файлов, заметок, сообщений. Хранится в SQLite ( через `sqlite-vec` extension или кастомная таблица).
- **Query:** пользователь вводит "документы про бюджет" — Semantic Search находит релевантные файлы, даже если в них нет слова "бюджет" (но есть "финансы", "расходы").
- **Hybrid search:** комбинация BM25 (FTS5) и semantic search. Ранжирование: 60% semantic + 40% keyword.

### 25.6 Cloud Bridge
- **Внешние AI:** OpenAI GPT-4o, Anthropic Claude, локальный Ollama.
- **API:** OpenAI-compatible (`/v1/chat/completions`).
- **Data policy:**
  - По умолчанию — локальные модели (SLM, embeddings).
  - Cloud Bridge — opt-in, пользователь явно включает в настройках.
  - Системные данные (файлы, сообщения) не отправляются в Cloud без explicit разрешения на файл.
- **Circuit breaker:** если Cloud API недоступен — fallback на локальные модели или текстовый ответ "Нужен интернет".
- **Cost tracking:** подсчёт токенов/стоимости запросов (для пользовательского контроля).

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Parse "open notes" | Rule-based | Ввод → Intent {action: "app.open", target: "notes"} |
| Parse complex query | SLM | "найди документы про бюджет" → semantic search |
| Generative UI | Codegen | "создай кнопку" → UI отображается в preview |
| Smart Scheduler | Планирование | Запланировать backup ночью → выполнено в 3:00 |
| Semantic search | Векторный | "финансы" → находит файл с "бюджет" |
| Cloud Bridge | Внешний AI | Включить → GPT отвечает на вопрос |
| Fallback | Offline | Отключить интернет → локальные модели |

## Интеграция с будущими этапами
- **Вход:** этап 13 (Command Bar) — текстовый ввод, режим Ask.
- **Вход:** этап 24 (Voice) — распознанный текст.
- **Выход:** resolved Intent → этап 20 (V8 Isolate) для app execution.
- **Выход:** Generative UI → этап 20 (`@workspace/ui` render).
- **Выход:** Smart Scheduler → этап 27 (Energy Manager) для constraints.
- **Выход:** Semantic Search → этап 12 (VFS) для file retrieval.

## Критерии приёмки
- [ ] Rule-based parser: 50 типовых команд → accuracy > 95%.
- [ ] SLM: сложные запросы → релевантный Intent (субъективная оценка, > 80% удовлетворённость).
- [ ] Generative UI: код компилируется, отображается, безопасен (sandbox).
- [ ] Smart Scheduler: задача выполнена в заданном временном окне.
- [ ] Semantic search: запрос "финансы" находит файл с "бюджет" (cosine similarity > 0.7).
- [ ] Cloud Bridge: ответ получен < 3 сек (при хорошем интернете).
- [ ] Fallback: при отсутствии интернета — локальные модели или понятная ошибка.

## Ссылки
- [layer-2-ai.md](../layers/layer-2-ai.md) — Intent API, Semantic Kernel, Generative UI, Smart Scheduler
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Command Bar, AI Agent
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Intent API §7.1–7.12

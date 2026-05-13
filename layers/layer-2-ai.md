# Layer 2 — ИИ-слой и Intent API (Core.Mind)

Как искусственный интеллект работает в Workspace: от голосового управления и генеративного UI до локального инференса, облачного моста и безопасности AI-операций.

---

## Принцип: ИИ как невидимый системный оператор

ИИ в CORE — не чат-бот, не помощник, не «Сири». Это автоматика, которая связывает приложения и систему через типизированные команды. Пользователь говорит «сделай» — система делает, не задавая лишних вопросов.

---

## Модели и их размещение

### Локальные модели (On-Device)

| Модель | Задача | Размер | Размещение |
|--------|--------|--------|------------|
| Whisper (OpenAI) | Распознавание речи (ASR) | 75 MB – 1.5 GB | Локально, CPU/NPU |
| SLM: Llama 3.x, Phi-4, Qwen 2.5, Mistral, GLM-4-9B | Понимание языка, Intent resolution | 2 GB – 8 GB | Локально, GPU/CPU |
| Embedding (nomic-embed-text, bge-m3) | Семантический поиск | 100 MB – 500 MB | Локально, CPU/GPU |
| Piper / Coqui TTS | Синтез речи | 50 MB – 200 MB | Локально, CPU |
| Multimodal (LLaVA, BakLLaVA) | Распознавание экрана, изображений | 4 GB – 8 GB | Локально, GPU (опционально) |

### Облачные модели (Cloud Bridge)

| Провайдер | Модели | Использование |
|-----------|--------|---------------|
| OpenAI | GPT-4o, GPT-4o-mini, o1 | Сложные рассуждения, генерация кода |
| Anthropic | Claude 3.5 Sonnet, Claude 3 Opus | Длинный контекст, анализ документов |
| GLM (Zhipu AI) | GLM-4, GLM-4-Flash | Китайский язык, локальная оптимизация |
| Kimi K2 (Moonshot AI) | Kimi K2, Kimi K2-lite | Длинный контекст (2M токенов) |
| Google | Gemini 1.5 Pro, Flash | Мультимодальность, поиск |
| Произвольный | Любой OpenAI-compatible API | Настраиваемый endpoint |

Облачные модели используются **только с явного согласия пользователя** (см. «Безопасность ИИ-слоя»).

### Гибридный режим (по умолчанию)

```
Простой запрос ("открой калькулятор")
  → Локальный SLM (миллисекунды)

Средний запрос ("найди договоры с неустойкой > 5%")
  → Локальный SLM + Embedding search + локальный анализ

Сложный запрос ("напиши эссе на 5 страниц")
  → Система спрашивает: "Отправить в [Провайдер]?"
  → Пользователь решает: Разрешить / Разрешить один раз / Нет
```

---

## Технические требования

### Минимальные (CPU-only)

| Компонент | Требование |
|-----------|------------|
| Процессор | x86_64 или ARM64 с AVX2 (2015+) |
| RAM | 8 GB (Whisper base + SLM 3B) |
| Диск | 5 GB свободного места для моделей |
| GPU | Не требуется |
| Сеть | Не требуется для локального режима |

### Рекомендуемые (GPU/NPU)

| Компонент | Требование |
|-----------|------------|
| Процессор | Apple Silicon M1+ / Intel Core i7+ / AMD Ryzen 7+ |
| RAM | 16 GB |
| GPU / NPU | 4+ GB VRAM (WebGPU) или Apple Neural Engine |
| Диск | 15 GB (Whisper medium + SLM 8B + embeddings + TTS) |
| Сеть | Для Cloud Bridge и обновления моделей |

### Максимальные (Local Workstation / Server)

| Компонент | Требование |
|-----------|------------|
| Процессор | Серверный CPU с большим кэшем |
| RAM | 32+ GB |
| GPU | 12+ GB VRAM (NVIDIA RTX 3060+, AMD RX 6800+) |
| Диск | 50+ GB (множественные модели, кэш) |
| Сеть | Стабильное соединение для Cloud Bridge |

---

## Устройства и аппаратное ускорение

| Задача | Устройство | Примеры железа |
|--------|------------|----------------|
| Распознавание речи (ASR) | NPU / CPU | Apple ANE, Qualcomm Hexagon, Intel NPU, AMD Ryzen AI |
| LLM inference | GPU / NPU | WebGPU (Vulkan/Metal/DX12), Apple Silicon, CUDA-through-WGPU |
| Embeddings | NPU / GPU | ONNX Runtime Web, WebGPU compute shaders |
| TTS | CPU / NPU | Piper (ONNX), Coqui (TFLite) |
| Генерация UI / изображений | GPU | WebGPU compute shaders, WGSL |

**Smart Scheduler** (см. ниже) распределяет нагрузку: если GPU занят игрой, Whisper переходит на NPU, а SLM — на CPU с пониженным приоритетом.

---

## Включение, отключение, настройка

### System Settings → AI (Core.Mind)

**Главный переключатель:**
- `Core.Mind: Включён / Выключен`
- При выключении Core.Mind: Voice Engine не загружается, облачные модели и генеративный UI отключены. **Intent API (базовый) остаётся активным** — Command Bar понимает текстовые команды через локальный rule-based парсер.

**Режим работы:**
- `Только локально` — Cloud Bridge полностью отключён, данные не покидают устройство.
- `Локально + Облако (с запросом)` — гибрид, explicit consent для каждого облачного запроса (по умолчанию).
- `Локально + Облако (авто)` — система сама решает, но логирует все облачные вызовы.
- `Только облако` — для тонких клиентов и устройств без GPU.

**Выбор моделей по задачам:**
- ASR (распознавание речи): Whisper base / medium / large
- NLU (понимание): Llama 3.1 8B / Phi-4 / Qwen 2.5 7B / GLM-4-9B
- Embeddings: nomic-embed-text / bge-m3
- TTS (синтез): Piper ru_RU / en_US / zh_CN
- Cloud LLM: GPT-4o / Claude 3.5 / GLM-4 / Kimi K2

**Ограничения ресурсов:**
- Максимум RAM для AI: 2 GB / 4 GB / 8 GB / без ограничений
- Максимум VRAM для AI: 2 GB / 4 GB / 8 GB / без ограничений
- Приоритет: Низкий (фон) / Средний / Высокий (отзывчивость)

**Голосовой ввод:**
- Язык: автоопределение / русский / английский / китайский
- Чувствительность wake-word: Низкая / Средняя / Высокая
- Wake-word: «Core» / кастомное (3-4 слога)
- Push-to-talk: клавиша / геймпад-кнопка / наушники

### Отключение по приложению

Приложение может отказаться от AI-интеграции:
```typescript
os.mind.optOut(); // Приложение не регистрирует Intents, игнорируется Semantic Kernel
```

### Отключение по профилю / роли

- Owner решает, может ли Member использовать Cloud Bridge.
- Guest: AI отключён по умолчанию (нет доступа к файлам, нет смысла в Semantic Kernel).

---

## Конвейер обработки запроса

```
[Голос / Текст / Жест / Контекст]
  ↓
Voice Engine (ASR: Whisper / Vosk)
  ↓
Intent Parser (NLU: локальный SLM или Regex + Embedding matching)
  ↓
Intent Resolver (выбор: встроенные Intents / приложения / системные команды)
  ↓
Context Enricher (Semantic Kernel: векторный поиск по файлам, контактам, истории)
  ↓
Action Executor (Intent API / системный вызов / генерация UI / Cloud Bridge)
  ↓
Response Formatter (текст / TTS / UI-виджет / безмолвное действие)
  ↓
Пользователь
```

### 1. Распознавание речи (ASR)

- **Локально:** Whisper base/medium/large. Работает на NPU/CPU с низкой задержкой (< 200 мс).
- **Облачно:** Если Whisper не справляется с акцентом или шумом — fallback на облачный ASR (с explicit consent).
- **Язык:** Автоопределение или фиксированный. Мультиязычные команды поддерживаются.
- **Wake-word:** «Core» активирует прослушивание. В Exclusive Mode — push-to-talk или постоянное прослушивание с шумоподавлением.

### 2. Парсинг запроса (NLU)

- **Локальный SLM** (3B – 8B параметров) извлекает entities и intent из текста.
- **Fallback:** Эвристический парсер для простых команд («открой X», «найди Y», «увеличь громкость»).
- **Контекст диалога:** SLM помнит 3-5 последних оборотов для уточнений («а теперь отсортируй по дате»).

### 3. Выбор смыслов (Intent Resolution)

- Сопоставление с зарегистрированными Intents приложений (см. Intent API).
- Системные Intents: яркость, громкость, переключение профиля, скриншот, запись экрана.
- **Неоднозначность:** «Открой терминал» → если есть 2 приложения с Intent `open_terminal`, система выбирает по приоритету (последнее использование, владелец проекта) или уточняет в Command Bar.

### 4. Поиск и контекст (Semantic Kernel)

- **Embeddings:** Локальная модель превращает файлы, заметки, контакты в векторы.
- **Индексирование:** Происходит в фоне через Smart Scheduler. Новые файлы индексируются в течение минуты.
- **Поиск:** По запросу «договор с неустойкой» система находит релевантные документы и передаёт их как context window в LLM.
- **Privacy:** Индекс хранится в app-scoped SQLite, зашифрован profile key.

### 5. Выполнение действия (Action Executor)

- **Intent API:** Вызов зарегистрированного приложения.
- **Системный вызов:** Изменение настроек, управление окнами.
- **Generative UI:** Если готового приложения нет — генерация временного виджета (см. ниже).
- **Cloud Bridge:** Если запрос требует сложных рассуждений и пользователь разрешил — отправка в облако.

### 6. Синтез речи (TTS) и ответ пользователю

- **Локальный TTS:** Piper / Coqui — офлайн, низкая задержка, подходит для коротких ответов.
- **Облачный TTS:** Высокое качество, естественная интонация (опционально).
- **Форматы ответа:**
  - **Безмолвный:** Действие выполнено, UI не изменился («сделай музыку тише»).
  - **Текстовый:** Результат в Command Bar.
  - **Голосовой:** TTS-ответ в наушники.
  - **UI-виджет:** Временный график, таблица, карта.

---

## Взаимодействие со стороны пользователя

### Голос

1. Wake-word («Core») или push-to-talk.
2. Команда естественным языком.
3. Система подтверждает распознавание (краткая вибрация / звук / текст в Command Bar).
4. Выполнение. Если требуется подтверждение — система спрашивает («Отправить запрос в OpenAI?»).

### Текст

- Command Bar → естественный язык.
- «Калькулятор: сколько будет 150 умножить на 43» → Intent API вызывает калькулятор.
- «Найди все фото с морем за прошлый год» → Semantic Kernel + векторный поиск.

### Жест / Контекст экрана

- «Вот это» (указание на объект) + голос: «перенеси в таблицу расходов».
- AI видит дерево компонентов Display Server (а не пиксели), понимает, какой элемент выделен.
- Мультитач: выделил текст → зажал → «переведи» → перевод появился рядом.

### Exclusive Mode (игры, 3D)

- Руки и глаза заняты — единственный канал голос.
- «Скинь этот кадр Ване» → скриншот → мессенджер.
- «Поставь будильник на 7» → системный вызов.
- «Что там по проекту?» → ИИ читает сводку в наушники.

---

## Взаимодействие со стороны системы

### Регистрация Intents (приложения)

```typescript
os.mind.registerIntent({
  name: "create_note",
  description: "Создает новую текстовую заметку",
  parameters: { content: "string", tags: "string[]?" },
  action: (data) => { /* логика создания */ }
});
```

Результат: пользователь говорит «Запиши, что я забыл купить молоко» → система сама вызывает `create_note` из приложения заметок, даже если оно закрыто.

### Системные события (Event Bus)

| Событие | Когда | Подписчики |
|---------|-------|------------|
| `ai.speech.recognized` | Whisper завершил распознавание | Command Bar, Intent Parser |
| `ai.intent.resolved` | Выбран Intent | Action Executor, Audit Engine |
| `ai.intent.ambiguous` | Неоднозначность | UI (уточнение в Command Bar) |
| `ai.action.executed` | Действие выполнено | Notification Manager, Audit Engine |
| `ai.action.failed` | Ошибка выполнения | UI (toast / TTS), приложение |
| `ai.context.updated` | Обновлён семантический индекс | Smart Folders, Search |
| `ai.cloud.requested` | Требуется облачный вызов | UI (запрос разрешения), Cloud Bridge |
| `ai.cloud.completed` | Облачный вызов завершён | Response Formatter, Audit Engine |

### Обратная связь от приложений

Приложение может сообщить AI о результате действия:
```typescript
os.mind.reportResult({ intent: "create_note", status: "success", data: { id: "note_123" } });
// Или:
os.mind.reportResult({ intent: "send_email", status: "failure", error: "Нет соединения" });
```

Это используется для:
- Уточнения ответа пользователю («Отправлено» / «Не удалось, нет сети»).
- Обучения приоритетов Intent Resolution (если приложение часто отказывает, его Intents понижаются в приоритете).

---

## Взаимодействие с технической стороны

### API для разработчиков

```typescript
import { mind, voice, ai } from '@core/ai';

// Регистрация Intent
mind.registerIntent({ name: "convert_currency", ... });

// Доступ к голосу
voice.onWakeWord(() => { ... });
voice.synthesize("Готово", { voice: "ru_RU-medium" });

// Прямой доступ к LLM (только Level 5 — нативные приложения)
const result = await ai.prompt({
  model: "local:llama3:8b",
  system: "Ты помощник по бухгалтерии",
  user: "Рассчитай НДС 20% от 15000 рублей"
});
```

### Pipeline Hooks

Разработчик может вмешаться в конвейер AI:

```typescript
// Предобработка распознанного текста
mind.on("preIntentParse", (text) => {
  return text.replace(/баксов/g, "USD"); // нормализация сленга
});

// Переопределение выбранного Intent
mind.on("postIntentResolve", (intent, context) => {
  if (intent.name === "open_browser" && context.project === "Работа") {
    return { ...intent, parameters: { ...intent.parameters, profile: "work" } };
  }
  return intent;
});

// Валидация перед выполнением
mind.on("preActionExecute", (intent) => {
  if (intent.name === "delete_file" && !intent.parameters.confirm) {
    throw new Error("Требуется подтверждение удаления");
  }
});
```

### Конфигурация моделей (core.json)

```json
{
  "ai": {
    "mode": "hybrid",
    "asr": { "provider": "whisper", "model": "base", "device": "npu", "language": "auto" },
    "nlu": { "provider": "local", "model": "llama3.1:8b", "device": "gpu", "context_window": 4096 },
    "embedding": { "provider": "local", "model": "nomic-embed-text", "device": "gpu" },
    "tts": { "provider": "piper", "voice": "ru_RU-medium", "speed": 1.0 },
    "cloud_bridge": {
      "enabled": true,
      "default_provider": "anthropic",
      "providers": {
        "openai": { "endpoint": "https://api.openai.com/v1", "model": "gpt-4o" },
        "anthropic": { "endpoint": "https://api.anthropic.com/v1", "model": "claude-3-5-sonnet" },
        "glm": { "endpoint": "https://open.bigmodel.cn/api/paas/v4", "model": "glm-4" },
        "kimi": { "endpoint": "https://api.moonshot.cn/v1", "model": "kimi-k2" }
      }
    },
    "scheduler": { "max_gpu_percent": 30, "max_ram_gb": 4 }
  }
}
```

---

## Сторонние AI (Cloud Bridge)

### Поддерживаемые провайдеры

Workspace поддерживает любой провайдер с OpenAI-compatible API:

- **OpenAI:** ChatGPT, GPT-4o, GPT-4o-mini, o1
- **Anthropic:** Claude 3.5 Sonnet, Claude 3 Opus, Claude 3 Haiku
- **GLM (Zhipu AI):** GLM-4, GLM-4-Flash, GLM-4V
- **Kimi K2 (Moonshot AI):** Kimi K2, Kimi K2-lite (до 2M контекста)
- **Google:** Gemini 1.5 Pro, Gemini 1.5 Flash
- **Произвольный:** Любой self-hosted или региональный endpoint

### Политика использования

1. **Explicit consent:** Данные отправляются только с явного согласия пользователя. Для каждого запроса: «Отправить в [Провайдер]?» → «Разрешить один раз / Всегда для этого приложения / Нет».
2. **Owner control:** Owner может заблокировать Cloud Bridge полностью (`policy: ai_cloud_bridge = false`). В этом режиме система не предлагает облачные вызовы.
3. **Аудит:** Все облачные запросы логируются: provider, endpoint, timestamp, размер prompt, не содержимое ответа.
4. **Prompt filtering:** Перед отправкой prompt проходит фильтрацию: удаление потенциально чувствительных данных (номера карт, пароли — если они попали в голосовой ввод), ограничение длины.
5. **No training:** Cloud Bridge явно указывает в запросах `X-No-Training: 1` (где провайдер поддерживает), запрещая использование данных для обучения моделей.

---

## Локальные модели (Ollama, llama.cpp, vLLM)

### Ollama (рекомендуемый способ)

```bash
# Установка модели
core-cli ai model pull llama3.1:8b
core-cli ai model pull phi4:14b
core-cli ai model pull qwen2.5:7b

# Workspace автоматически интегрирует Ollama как backend
# Ollama запускается как отдельный процесс (Level 0), общается через localhost API
```

- Поддержка GGUF-квантования: Q4_K_M (экономия VRAM), Q5_K_M (баланс), Q8_0 (качество).
- Автоматическое переключение: если Ollama не запущен, Workspace стартует его при первом AI-запросе.

### llama.cpp / vLLM (продвинутый способ)

```bash
# Для GPU-серверов и энтузиастов
core-cli ai backend set vllm --model /path/to/model --gpu-memory-utilization 0.8
```

- **vLLM:** оптимален для серверных GPU (batch inference, PagedAttention).
- **llama.cpp:** оптимален для CPU (ARM NEON, AVX2, AMX).

### Собственные модели

Пользователь или компания может загрузить собственную GGUF / ONNX-модель:

```bash
core-cli ai model register \
  --name my-company-llm \
  --path /vfs/models/company-llm-q4.gguf \
  --type llama \
  --context 8192
```

Модель появляется в списке выбора наряду с Llama, Phi и др.

---

## Безопасность ИИ-слоя

### Изоляция моделей

- AI-процессы (Whisper, SLM, Ollama, TTS) запускаются в отдельных sandbox'ах (V8 Isolate для JS-обёрток, отдельные процессы для нативных бэкендов).
- **Нет прямого доступа** к VFS, SQLite, сети (кроме Cloud Bridge через прокси).
- Доступ только через `@core/*` wrappers и Intent API.

### Защита данных

| Сценарий | Защита |
|----------|--------|
| Локальные модели | Данные никогда не покидают устройство. Model weights хранятся в зашифрованном виде (profile key). |
| Облачные модели | Explicit consent + audit log. Prompt filtering удаляет чувствительные данные перед отправкой. |
| Semantic Index | Embeddings и индекс хранятся в app-scoped SQLite, зашифрованы profile key. |
| Voice History | Аудиозаписи голосовых команд не сохраняются (только распознанный текст, опционально). |
| Whisper Integrity | BLAKE3-хеш модели проверяется при загрузке. Подмена → отказ загрузки. |
| LED Indicator | Аппаратный LED активируется при захвате микрофона. Пользователь видит, что система слушает. |
| Audio Buffer Wipe | Сырые audio samples обнуляются после распознавания, не сохраняются на диск. |

### Защита от prompt injection

- **Input sanitization:** Передача пользовательского ввода в LLM через parametrized templates, а не конкатенацию строк.
- **System prompt isolation:** System instructions недоступны для переопределения пользовательским prompt.
- **Output validation:** Результат LLM (если он интерпретируется как системная команда) проходит sandbox-валидацию перед выполнением.
- **Intent Whitelist:** Даже если LLM предложит опасное действие, оно не выполнится, если нет зарегистрированного Intent с соответствующими capabilities.

### RBAC для AI

| Право | Кто | Описание |
|-------|-----|----------|
| `ai:use_local` | Member+ | Использование локальных моделей |
| `ai:use_cloud` | Owner / Member (по решению Owner) | Использование облачных моделей |
| `ai:register_intent` | Developer (Level 5 apps) | Регистрация Intents приложением |
| `ai:load_custom_model` | Owner | Загрузка собственных GGUF/ONNX-моделей |
| `ai:configure` | Owner | Изменение настроек Core.Mind |
| `ai:view_audit` | Owner | Просмотр истории AI-действий |

- **Guest:** AI отключён по умолчанию. Нет доступа к Semantic Kernel (нет своих данных).

### Аудит

Все AI-действия логируются в Audit Engine:

```
timestamp | user_id | action | details
----------|---------|--------|--------
2026-05-09T14:32:01 | user_42 | ai.speech.recognized | "открой калькулятор", confidence: 0.97
2026-05-09T14:32:02 | user_42 | ai.intent.resolved | intent: "open_app", target: "calculator"
2026-05-09T14:35:15 | user_42 | ai.cloud.requested | provider: "anthropic", prompt_size: 2048 bytes, approved: true
2026-05-09T14:35:18 | user_42 | ai.cloud.completed | provider: "anthropic", latency: 2.3s
```

Owner просматривает историю через Command Bar → «История AI» или Core.Backoffice → Audit → Category: AI.

### Защита от эксфильтрации через AI

- **DLP (Data Loss Prevention):** Если пользовательский prompt содержит паттерны, похожие на номера кредитных карт, паспортов, API-ключи — система предупреждает: «В вашем запросе обнаружены потенциально чувствительные данные. Отправить всё равно?»
- **Max prompt size:** Ограничение на размер prompt для Cloud Bridge (например, 32 KB), предотвращающее случайную отправку больших файлов.
- **No file upload to cloud:** Через Cloud Bridge отправляется только текст prompt. Файлы анализируются локально (SLM + embeddings), облачный LLM получает только извлечённую структуру.

---

## ИИ как "Мост данных"

### Связывание приложений без участия юзера
- «Запомни этот момент» (в игре) → скриншот + распознавание объектов → заметка-черновик в рабочем пространстве.
- «Найди все договоры подряда за прошлый месяц, где неустойка выше 5%» → ИИ лезет в файлы, индексирует нативно → список за секунду.

### Multi-modal
ИИ «видит» экран не как пиксели, а как структурное дерево компонентов:
- «Перенеси число из этого чека в мою таблицу расходов»
- Прямой доступ к объектам на холсте → выцепление данных → переброска

---

## Фоновое выполнение (Contextual Actions)

ИИ дергает методы приложений напрямую, не разворачивая окна:

| Команда | Действие |
|---------|----------|
| «Сделай музыку тише» | Сигнал в Core.Audio, плеер не открывается |
| «Запиши последние 30 секунд экрана» | Display Server выгружает буфер → файл с тегом |
| «Найди где в этой игре лежит артефакт X» | ИИ парсит гайд в замороженном браузере → зачитывает в наушники |

---

## Smart Scheduler (Управление ресурсами)

ИИ следит, чтобы фоновые задачи не просаживали FPS:
- Видит нагрузку на GPU через WebGPU timestamps.
- Динамически ограничивает аппетиты фоновых задач.
- «Отрендери это видео, пока я играю» → «Сделаю, но на 30% мощности, чтобы игра не лагала».
- Если GPU занят игрой, Whisper переходит на NPU, SLM — на CPU с пониженным приоритетом.

---

## Generative UI (On-the-fly)

Если готового приложения нет:
1. ИИ собирает данные из разных источников (Semantic Kernel).
2. Пишет JS-код визуализации на лету (Level 5 sandbox).
3. Отрисовывает временный виджет в пространстве через `@core/graphics`.

Пример: «Покажи график моих трат на кофе за год» → ИИ генерирует виджет из данных чеков.

---

## ИИ НЕ находится в ядре

Важное разделение:
- **Ядро (Kernel, VFS):** Только чистая математика (CRDT). Детерминировано.
  - Текст и конфиги → Causal Trees (merge без потерь).
  - Простые состояния → LWW-Element-Set (Last Write Wins по Hybrid Logical Clock).
  - True concurrent conflict на регистре → **Hash-based Ordering**: сравниваем BLAKE3 хэши значений, winner выбирается детерминированно (например, меньший хэш). Никаких веток, никакого ручного разрешения.
- **ИИ (User Space):** Не разрешает конфликты данных — winner уже выбран ядром. ИИ может помочь приложению показать историю изменений или diff, но решение принято математически и не обсуждается.

---

## ИИ-диспетчер (The Guided Experience)

ИИ не болтает, он действует:
- «Я оптимизировал твоё пространство»
- «Я нашёл ту фотку из 2015-го"
- Автоматический переход в Exclusive при запуске тяжёлого 3D-редактора
- Гашение уведомлений при обнаружении фокусировки (через Intent API + Display Server)

---

## Связь с другими слоями

| Слой | Что описано |
|------|-------------|
| [Layer: UX](layer-1-user-experience.md) | Голосовое управление, Command Bar, Generative UI — пользовательский опыт |
| [Layer: Фронт/Бэк](layer-3-system-split.md) | Intent API как компонент Бэка, AI-мост между приложениями |
| [Layer: Устройства](layer-5-devices.md) | Whisper на NPU, локальный инференс на GPU, Core-Plug как hardware target |
| [Layer: Приложения](layer-6-apps.md) | Регистрация Intents приложениями, mind capability, Design System для Generative UI |
| [Layer: Безопасность](layer-7-security.md) | Cloud Bridge — политики отправки данных, аудит AI-действий, RBAC для AI |
| [Layer: Подсистемы](layer-8-technical-decomposition.md) | Intent API, Voice Engine, Semantic Kernel — техническая реализация |
| [Layer: Business](layer-10-business-model.md) | AI как киллер-фича для входа, B2B SaaS, обучение и сертификация |

---

## Предыдущий слой

Layer 1 описывает пользовательский опыт — Space, Command Bar, проекты и приложения. [См. layer-1-user-experience.md](layer-1-user-experience.md).

---

## Следующий слой

Layer 3 описывает **разделение системы на Фронт и Бэк** — как Space технически реализуется через подключение к одному или нескольким Бэкам. [См. layer-3-system-split.md](layer-3-system-split.md).

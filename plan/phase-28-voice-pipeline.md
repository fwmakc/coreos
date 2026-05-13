# Этап 28 — Voice Pipeline

## Цель
Создать голосовой интерфейс: распознавание речи (ASR через Whisper), синтез речи (TTS), Zero UI (голосовые команды без экрана), и Intent Queue (обработка при перегрузке CPU). После этого этапа пользователь может управлять Workspace голосом.

## Язык и стек
- **Язык:** TypeScript (оркестрация), Rust (Whisper инференс через FFI)
- **Runtime:** Bun + whisper.cpp (через `bun:ffi` или child process)
- **Ключевые зависимости:** `whisper.cpp` (C++ inference), `piper` (TTS, ONNX), `cpal` (audio, уже в этапе 4)
- **Целевые ОС:** Windows, macOS, Linux, Android

## Зависимости
- **Этап 6** — Host Shim: Audio (capture/playback, ring buffers).
- **Этап 12** — Micro-Kernel: WORKSPACE (event loop, IPC).
- **Этап 15** — Command Bar (Input Router для voice input).
- **Этап 29** — Intent API (voice → Intent → Action, placeholder на этом этапе).

## Часть системы
**Level 4 — Voice Engine** [См. layer-8 §7, layer-2 §3, layer-1 §7]

## Требования

### 24.1 Whisper ASR
- **Модель:** `whisper-base` (74 МБ, 1 ГБ RAM, real-time на CPU) — дефолт. `whisper-tiny` (39 МБ) для устройств с < 2 ГБ RAM. `whisper-small` (244 МБ) для лучшей точности.
- **Инференс:** whisper.cpp через FFI (`bun:ffi`) или child process с SharedArrayBuffer.
- **Аудио pipeline:**
  - Host Shim Audio (этап 4) захватывает 16 kHz mono f32.
  - Ring buffer на 30 секунд.
  - VAD (Voice Activity Detection): пороговый детектор или tiny модель. Если silence > 2 сек — считаем, что фраза закончилась.
  - Chunk extraction: извлечение 30-секундного сегмента (или меньше, если фраза короче).
  - Mel spectrogram + inference.
  - Результат: `text` + `confidence`.
- **Wake word (опционально):** "WORKSPACE" или "Компьютер". Если включено — система слушает постоянно (с низким энергопотреблением), и активируется при wake word.
- **Языки:** русский и английский (автоопределение). Мультиязычность — post-release.

### 24.2 TTS Engine
- **Модель:** Piper (ONNX, ~20–50 МБ на язык).
- **Язык:** русский голос (по умолчанию). Английский — post-release.
- **Synthesis:** phonemization → ONNX inference → PCM f32.
- **Playback:** через Host Shim Audio (этап 4).
- **Latency:** < 100 мс для фраз < 10 слов.
- **Параметры:** скорость (0.5–2.0), громкость (0.0–1.0).

### 24.3 Zero UI
- **Команды без экрана:** когда пользователь говорит команду, система выполняет её без открытия UI.
- **Handler mapping:**
  - "сделай музыку тише" → `audio.set_volume(level)`.
  - "поставь будильник на 7" → `scheduler.set_alarm(time)`.
  - "отправь скриншот Ивану" → `messenger.send_screenshot(contact)`.
  - "покажи сводку проекта" → `project.get_summary()`.
  - "заблокируй экран" → `system.lock_screen()`.
- **Ответ:** TTS feedback ("Громкость 50 процентов"). Короткий звуковой сигнал (success/error beep) если TTS занят.
- **Push-to-talk:** активация голосового ввода по клавише (настраивается), геймпад-кнопке, или кнопке наушников.

### 24.4 Intent Queue
- **Сценарий:** пользователь даёт голосовую команду при 100% CPU load (например, идёт компиляция).
- **Поведение:**
  - Команда распознаётся, Intent создаётся и ставится в очередь.
  - Отображается Static UI Overlay: "Принято" → "Выполняется...".
  - Когда CPU load < 70% — Intent извлекается из очереди и выполняется.
  - TTS: "Команда выполнена".
- **Таймаут:** если Intent не выполнен за 5 секунд — отмена, TTS: "Не удалось выполнить команду".

### 24.5 Voice Security
- **LED индикатор:** при захвате микрофона Host Shim (этап 4) включает LED (Scroll Lock или системный индикатор).
- **Audio buffer zeroize:** после распознавания аудио-буфер очищается (zeroize).
- **Model integrity:** BLAKE3 хеш модели Whisper проверяется при загрузке. Подмена модели → отказ загрузки.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| ASR accuracy | Распознавание | 100 тестовых фраз → accuracy > 90% |
| TTS latency | Синтез | Фраза < 10 слов → < 100 мс |
| Zero UI command | Без экрана | "сделай тише" → громкость меняется, TTS ответ |
| Intent Queue | Перегрузка CPU | Команда при 100% CPU → выполнение при освобождении |
| Push-to-talk | Активация | Нажать клавишу → голосовой ввод активен |
| Wake word | Постоянное прослушивание | Сказать "WORKSPACE" → система слушает |
| LED indicator | Индикация | Захват микрофона → LED включается |

## Интеграция с будущими этапами
- **Вход:** этап 4 (Audio) — capture/playback.
- **Вход:** этап 13 (Command Bar) — Input Router (voice как источник ввода).
- **Выход:** распознанный текст → этап 25 (Intent API) для парсинга.
- **Выход:** TTS → этап 4 (Audio) для playback.
- **Выход:** Zero UI handlers → этап 15 (Window Manager, volume), 14 (Project Manager), 22 (Messenger).
- **Вход:** этап 25 (Intent API) — resolved Intent для выполнения.

## Критерии приёмки
- [ ] Whisper accuracy > 90% на 100 тестовых фразах (русский, чистая речь).
- [ ] Whisper real-time на CPU (base model, < 2 ГБ RAM).
- [ ] TTS latency < 100 мс для фраз < 10 слов.
- [ ] Zero UI: 10 команд работают без открытия UI.
- [ ] Intent Queue: команда при 100% CPU выполняется позже.
- [ ] LED включается при захвате микрофона.
- [ ] Audio buffer zeroize после распознавания.
- [ ] Model integrity: подмена модели → отказ загрузки.

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Голосовое управление, Zero UI
- [layer-2-ai.md](../layers/layer-2-ai.md) — Whisper, TTS
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Voice Engine §7

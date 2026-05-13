# Этап 6 — Host Shim: Audio

## Цель
Добавить в Host Shim кроссплатформенную аудио-подсистему: захват (capture) с микрофона и воспроизведение (playback) на динамики/наушники. После этого этапа Workspace умеет записывать и воспроизводить аудио на всех трёх платформах.

## Язык и стек
- **Язык:** Rust
- **Ключевые зависимости:** `cpal` (cross-platform audio library), `symphonia` (декодирование аудио, опционально), `rubato` (ресемплинг, если нужен)
- **Целевые ОС:** Windows (WASAPI Shared Mode, fallback DirectSound), macOS (CoreAudio), Linux (PulseAudio, PipeWire через ALSA/JACK), Android (AAudio, OpenSL ES fallback)

## Зависимости
- **Этап 1** — Host Shim: Windows (event loop для audio callback).
- **Этап 2** — Host Shim: macOS.
- **Этап 3** — Host Shim: Linux.

## Часть системы
**Level 0 — Host Shim** [См. layer-8 §4.1.3, layer-7 §21.10, layer-1 §7]

## Требования

### 4.1 Аудио-абстракция
- Определение trait `AudioBackend`:
  - `list_devices() -> Vec<AudioDevice>` (входные и выходные).
  - `open_output(device_id, config) -> AudioOutputStream`
  - `open_input(device_id, config) -> AudioInputStream`
  - `get_default_output() -> AudioDevice`
  - `get_default_input() -> AudioDevice`
- Конфигурация потока: sample rate (48 kHz по умолчанию, 16 kHz для ASR), channels (mono/stereo), format (f32le).

### 4.2 Воспроизведение (Playback)
- Ring buffer между Rust-стороной и callback thread `cpal`.
- Метод `write_samples(data)` — неблокирующий, данные пишутся в ring buffer.
- Callback thread вызывается ОС с фиксированным размером буфера (10 мс).
- Если ring buffer underrun — заполняется нулями (silence), событие `AudioUnderrun` логируется.

### 4.3 Захват (Capture)
- Ring buffer для входных данных.
- Callback thread читает из `cpal` input callback и пишет в ring buffer.
- Потребитель (например, Whisper на этапе 24) читает из ring buffer через `read_samples(n)`.
- Если ring buffer overrun — старые данные перезаписываются (circular), событие `AudioOverrun` логируется.

### 4.4 Безопасность аудио
- **LED индикатор записи:** при активном `AudioInputStream` Host Shim должен сигнализировать о записи (например, через keyboard LED Scroll Lock или системный API индикатора). Если аппаратный LED недоступен — отображается UI-индикатор (но UI ещё не готов, поэтому резервируется API).
- **Zeroize буфера:** после остановки захвата аудио-буфер заполняется нулями через `zeroize` crate.

### 4.5 Платформенные особенности
- **Windows:** WASAPI Shared Mode (event-driven). Fallback на DirectSound если WASAPI недоступен.
- **macOS:** CoreAudio через `cpal` (AVAudioEngine не используется, низкоуровневый доступ).
- **Linux:** ALSA/PulseAudio через `cpal`. PipeWire работает через PulseAudio-совместимый слой.
- **Android:** AAudio (API 26+), OpenSL ES fallback.

## Ключевые функции

| Функция | Описание | Тест |
|---------|----------|------|
| Список устройств | Вывод всех input/output | Запрос → список содержит микрофон и динамики |
| Воспроизведение sine | Генерация синусоиды 440 Гц | Запуск → слышен звук |
| Захват 5 секунд | Запись с микрофона | Запуск → файл содержит аудио |
| 16 kHz для ASR | Ресемплинг | Запрос 16 kHz → callback выдаёт 16 kHz |
| LED при захвате | Индикация | Запуск input → LED включается |
| Zeroize | Очистка | Остановка → буфер заполнен нулями |

## Интеграция с будущими этапами
- **Выход:** `AudioInputStream` → этап 24 (Voice Pipeline / Whisper) для ASR.
- **Вход:** этап 24 (TTS) → `AudioOutputStream` для воспроизведения речи.
- **Вход:** этап 23 (VoIP) → duplex audio (одновременный input/output).

## Критерии приёмки
- [ ] Компилируется на Windows, macOS, Linux, Android.
- [ ] Список устройств корректен на всех платформах.
- [ ] Воспроизведение sine wave работает (слышимый звук).
- [ ] Захват 5 секунд записывает аудио в файл.
- [ ] 16 kHz конфигурация работает.
- [ ] LED/индикатор включается при захвате.
- [ ] Буфер zeroize после остановки (проверяется через memory dump).

## Ссылки
- [layer-1-user-experience.md](../layers/layer-1-user-experience.md) — Голосовое управление
- [layer-7-security.md](../layers/layer-7-security.md) — Безопасность аудио
- [layer-8-technical-decomposition.md](../layers/layer-8-technical-decomposition.md) — Audio §4.1.3

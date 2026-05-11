# Этап 7 — Voice Engine (Голосовой ввод)

> **Цель:** Система распознаёт голосовые команды локально (Whisper), синтезирует речь (TTS), работает Zero UI (команды без экрана), Intent Queue (обработка при перегрузке CPU).

**После этого этапа:** пользователь говорит "открой заметки" — система распознаёт, определяет Intent, выполняет. Говорит "поставь будильник на 7" — Scheduler создаёт задачу. В игре: "сделай музыку тише" — работает через Zero UI без отвлечения.

---

## Зависимости

- **Этап 1 (Фундамент):** Host Shim (аудио capture/playback)
- **Этап 2 (Command Bar):** Input Router, Intent API (base tier)
- **Этап 3 (Проекты):** Scheduler (для напоминаний)
- **Этап 6 (Коммуникации):** Messenger (для команд "позвонить", "отправить")

---

## Компоненты

### 7.1 Whisper (ASR)

```rust
// voice/src/whisper.rs
pub struct WhisperEngine {
    model: WhisperModel,
    audio_buffer: RingBuffer<f32>,
    state: RecognitionState,
    core_pinning: CpuAffinity,
}

impl WhisperEngine {
    pub fn new(model_path: &str, config: WhisperConfig) -> Result<Self, WhisperError> {
        let model = WhisperModel::load(model_path)?;
        Ok(Self {
            model,
            audio_buffer: RingBuffer::new(config.sample_rate * 30), // 30 сек буфер
            state: RecognitionState::Idle,
            core_pinning: CpuAffinity::new(config.dedicated_core),
        })
    }
    
    pub fn feed_audio(&mut self, samples: &[f32]) {
        self.audio_buffer.write(samples);
        
        // Wake word detection (опционально)
        if self.config.wake_word_enabled && self.state == RecognitionState::Idle {
            if self.detect_wake_word(samples) {
                self.state = RecognitionState::Listening;
                self.callback(WakeWordDetected);
            }
        }
    }
    
    pub fn process(&mut self) -> Option<RecognitionResult> {
        if self.state != RecognitionState::Listening {
            return None;
        }
        
        // Извлечение 30-секундного chunk'а
        let chunk = self.audio_buffer.read_last(30 * self.config.sample_rate);
        
        // Mel spectrogram
        let spectrogram = self.compute_mel_spectrogram(chunk);
        
        // Inference
        let tokens = self.model.decode(&spectrogram)?;
        let text = self.tokenizer.decode(&tokens);
        
        // VAD (Voice Activity Detection) — проверяем, закончил ли пользователь говорить
        if self.is_silence(&chunk) {
            self.state = RecognitionState::Idle;
            self.audio_buffer.clear();
            Some(RecognitionResult { text, confidence: self.model.confidence() })
        } else {
            None
        }
    }
    
    fn detect_wake_word(&self, samples: &[f32]) -> bool {
        // Простой пороговый детектор или маленькая нейросеть
        // Поддерживаемые wake words: "CORE", "Компьютер", кастомные
        self.wake_word_detector.process(samples)
    }
}

struct WhisperConfig {
    model_size: WhisperSize, // Tiny, Base, Small, Medium, Large
    sample_rate: u32, // 16000
    language: String, // "auto" или конкретный
    wake_word_enabled: bool,
    wake_word: String,
    dedicated_core: Option<usize>, // Core Pinning
}

enum WhisperSize {
    Tiny,   // 39 MB, 1 GB RAM, CPU real-time
    Base,   // 74 MB, 1 GB RAM, CPU real-time
    Small,  // 244 MB, 2 GB RAM, CPU real-time
    Medium, // 769 MB, 5 GB RAM, GPU recommended
    Large,  // 1550 MB, 10 GB RAM, GPU required
}
```

**Интеграция с Bun:**
```typescript
// micro-kernel/src/voice/whisper-host.ts
export class WhisperHost {
    private whisper: WhisperEngine; // Rust через FFI
    
    async start(): Promise<void> {
        // Запуск в отдельном потоке (Worker или child process)
        this.whisper = await WhisperFFI.load({
            model: 'whisper-base',
            language: 'ru',
            wakeWord: 'CORE',
        });
        
        // Аудио callback
        hostShim.audio.onInput((samples: Float32Array) => {
            this.whisper.feedAudio(samples);
        });
        
        // Периодическая обработка (каждые 100 мс)
        setInterval(() => {
            const result = this.whisper.process();
            if (result) {
                this.onRecognition(result.text);
            }
        }, 100);
    }
    
    private onRecognition(text: string): void {
        // Отправка в Input Router как обычный текст
        inputRouter.handleInput(text, { source: 'voice' });
    }
}
```

### 7.2 TTS Engine

```rust
// voice/src/tts.rs
pub struct TTSEngine {
    model: PiperModel, // или Coqui
    audio_output: AudioHost,
}

impl TTSEngine {
    pub fn new(model_path: &str, config: TTSConfig) -> Result<Self, TTSError> {
        let model = PiperModel::load(model_path)?;
        Ok(Self { model, audio_output: AudioHost::new()? })
    }
    
    pub fn synthesize(&mut self, text: &str) -> Result<Vec<f32>, TTSError> {
        // Phonemization
        let phonemes = self.phonemize(text);
        
        // Inference
        let audio = self.model.synthesize(&phonemes)?;
        
        Ok(audio)
    }
    
    pub fn speak(&mut self, text: &str) -> Result<(), TTSError> {
        let audio = self.synthesize(text)?;
        self.audio_output.play(&audio);
        Ok(())
    }
    
    pub async fn speak_async(&mut self, text: &str) -> Result<(), TTSError> {
        // Для долгих фраз — stream playback
        let stream = self.model.synthesize_stream(text)?;
        self.audio_output.play_stream(stream).await;
        Ok(())
    }
}

struct TTSConfig {
    voice: String, // "piper-ru_RU" или путь к модели
    speed: f32,    // 0.5 .. 2.0
    pitch: f32,    // -1.0 .. 1.0
    volume: f32,   // 0.0 .. 1.0
}
```

**Задержка:** < 100 мс для коротких фраз (< 10 слов).

### 7.3 Zero UI

```typescript
// micro-kernel/src/voice/zero-ui.ts
export class ZeroUI {
    private intentMap: Map<string, ZeroUIHandler>;
    
    constructor() {
        this.registerHandlers();
    }
    
    private registerHandlers(): void {
        this.intentMap.set('audio.set_volume', this.handleSetVolume);
        this.intentMap.set('scheduler.set_alarm', this.handleSetAlarm);
        this.intentMap.set('messenger.send_screenshot', this.handleSendScreenshot);
        this.intentMap.set('project.get_summary', this.handleProjectSummary);
        this.intentMap.set('system.lock_screen', this.handleLockScreen);
    }
    
    async execute(intent: Intent): Promise<void> {
        const handler = this.intentMap.get(intent.action);
        if (!handler) {
            // Fallback: TTS "Не понял команду"
            await tts.speak('Не понял команду');
            return;
        }
        
        const result = await handler(intent.params);
        
        // Ответ через TTS (без UI)
        if (result.speech) {
            await tts.speak(result.speech);
        }
        
        // Опционально: короткий звуковой сигнал
        if (result.success) {
            await audio.playBeep('success');
        }
    }
    
    private async handleSetVolume(params: { level: number }): Promise<ZeroUIResult> {
        await core.audio.setVolume(params.level);
        return { success: true, speech: `Громкость ${Math.round(params.level * 100)} процентов` };
    }
    
    private async handleSendScreenshot(params: { contact: string }): Promise<ZeroUIResult> {
        const screenshot = await displayServer.captureScreen();
        const contact = await contactBook.findByName(params.contact);
        await messenger.sendToContact(contact.id, { type: 'image', data: screenshot });
        return { success: true, speech: `Скриншот отправлен ${contact.name}` };
    }
}
```

### 7.4 Intent Queue

```typescript
// micro-kernel/src/voice/intent-queue.ts
export class IntentQueue {
    private queue: QueuedIntent[] = [];
    private isProcessing: boolean = false;
    
    async enqueue(intent: Intent): Promise<void> {
        this.queue.push({
            intent,
            enqueuedAt: Date.now(),
            status: 'pending',
        });
        
        // Показать "Принято" overlay
        displayServer.showOverlay({
            type: 'ack',
            message: 'Принято',
        });
        
        // Попытка обработки
        await this.tryProcess();
    }
    
    private async tryProcess(): Promise<void> {
        if (this.isProcessing) return;
        
        // Проверка CPU load
        const cpuLoad = await hostShim.getCpuLoad();
        if (cpuLoad > 0.9) {
            // CPU занят — ждём
            displayServer.showOverlay({
                type: 'progress',
                message: 'Выполняется...',
            });
            return;
        }
        
        this.isProcessing = true;
        
        while (this.queue.length > 0) {
            const item = this.queue.shift()!;
            item.status = 'processing';
            
            try {
                // Таймаут 5 секунд
                const result = await Promise.race([
                    actionExecutor.execute(item.intent),
                    new Promise((_, reject) => 
                        setTimeout(() => reject(new Error('Timeout')), 5000)
                    ),
                ]);
                
                item.status = 'completed';
            } catch (e) {
                item.status = 'failed';
                // TTS: "Не удалось выполнить, нужен интернет"
                await tts.speak('Не удалось выполнить команду');
            }
        }
        
        this.isProcessing = false;
        displayServer.hideOverlay();
    }
    
    // Вызывается при освобождении CPU
    onCpuFreed(): void {
        this.tryProcess();
    }
}
```

### 7.5 Безопасность голоса

```rust
// voice/src/security.rs
pub struct VoiceSecurity {
    led_controller: LedController,
    audio_wiper: AudioWiper,
    model_integrity: ModelIntegrity,
}

impl VoiceSecurity {
    pub fn onRecordingStart(&mut self) {
        // LED индикатор
        self.led_controller.set(true);
    }
    
    pub fn onRecordingEnd(&mut self, audio_buffer: &mut [f32]) {
        // LED выключение
        self.led_controller.set(false);
        
        // Zeroize audio buffer
        self.audio_wiper.zeroize(audio_buffer);
    }
    
    pub fn verifyModel(&self, model_path: &str) -> bool {
        // BLAKE3 хеш модели
        let expected_hash = self.get_expected_hash(model_path);
        let actual_hash = blake3::hash(&std::fs::read(model_path).unwrap());
        expected_hash == actual_hash.as_bytes()
    }
    
    pub fn verifySpeaker(&self, samples: &[f32]) -> f32 {
        // Опционально: speaker identification
        // Возвращает confidence score
        self.speaker_model.verify(samples)
    }
}
```

---

## Шаги реализации

### Шаг 7.1: Whisper (5 дней)

1. Загрузка whisper.cpp / whisper-rs
2. Модели: tiny (для тестов), base (дефолт)
3. Audio capture через CPAL (16 kHz, mono, f32)
4. Real-time inference (chunk-based)
5. Wake word detection (threshold или tiny model)
6. Тест: распознавание 100 фраз, accuracy > 90%

### Шаг 7.2: TTS (3 дня)

1. Piper model loading
2. Phonemizer (espeak-ng или встроенный)
3. Audio synthesis
4. Playback через CPAL
5. Тест: синтез 10 фраз, latency < 100 мс

### Шаг 7.3: Zero UI (3 дня)

1. Intent → ZeroUI handler mapping
2. System handlers (volume, alarm, screenshot, lock)
3. TTS feedback
4. Audio cues (success/error beeps)
5. Тест: "сделай музыку тише" → громкость меняется

### Шаг 7.4: Intent Queue (2 дня)

1. Queue structure
2. CPU load monitoring
3. Static UI Overlay (ack, progress)
4. Retry on CPU free
5. Тест: команда при 100% CPU → очередь → выполнение при освобождении

### Шаг 7.5: Voice Security (2 дня)

1. LED control (Host Shim GPIO/keyboard LED)
2. Audio buffer zeroize
3. Model integrity check (BLAKE3)
4. Тест: LED включается при записи, буфер очищается

### Шаг 7.6: Интеграция (3 дней)

1. Command Bar voice mode (микрофон иконка)
2. Push-to-talk (клавиша/геймпад/наушники)
3. Voice → Intent → Action полный цикл
4. Performance test: voice command → execution < 500 мс

---

## Критерии приёмки

- [ ] Whisper: accuracy > 90% на 100 тестовых фразах
- [ ] Whisper: real-time на CPU (base model, < 2 GB RAM)
- [ ] TTS: latency < 100 мс для фраз < 10 слов
- [ ] Zero UI: 10 команд работают без экрана
- [ ] Intent Queue: команда при 100% CPU → выполняется позже
- [ ] LED: включается при захвате микрофона
- [ ] Audio buffer: очищается после распознавания
- [ ] Model integrity: подмена модели → отказ загрузки
- [ ] Push-to-talk: работает с клавишей/геймпадом

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Whisper: только русский и английский | Post-release | Мультиязычность |
| TTS: только 1 голос | Post-release | Множественные голоса |
| Zero UI: 10 команд | Этап 9 | Полный Intent API |
| Нет speaker identification | Этап 8 | Биометрия голоса |
| Нет облачного fallback ASR | Этап 9 | Cloud Bridge для Whisper |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Whisper | layer-8 §7.1, layer-2 (ASR) |
| TTS | layer-8 §7.2, layer-2 (TTS) |
| Zero UI | layer-8 §7.3, layer-1 (голосовое управление) |
| Intent Queue | layer-8 §7.3.1, layer-1 (Graceful Degradation) |
| Voice Security | layer-8 §7.4, layer-7 (безопасность голоса) |

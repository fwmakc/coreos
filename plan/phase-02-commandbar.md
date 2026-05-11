# Этап 2 — Command Bar (Строка ввода)

> **Цель:** Работает Command Bar — строка ввода внизу (или вверху) экрана с 8 режимами, подсказками при вводе, настройками внешнего вида. Можно вводить текст, переключать режимы, получать подсказки.

**После этого этапа:** пользователь видит строку, набирает текст, система определяет режим, показывает подсказки. Работают режимы: Поиск (по файлам), Заметка (сохранение в SQLite), Калькулятор (встроенный math parser), Терминал (выполнение команд через Host Shim PTY). Остальные режимы — placeholder'ы.

---

## Зависимости

- **Этап 1 (Фундамент):** Display Server для рендеринга строки, Host Shim для ввода, SQLite (через Bun)
- **Bun runtime:** должен быть встроен в бинарник или запускаться как отдельный процесс, общающийся с Display Server через ABI

---

## Компоненты

### 2.1 Bun Runtime (Level 1)

На этом этапе Micro-Kernel — это Bun process, который общается с Display Server через FFI или IPC.

```rust
// ABI между Rust (Display Server) и Bun (Micro-Kernel)
#[repr(C)]
pub struct CoreAbi {
    // Display Server → Micro-Kernel
    pub on_input: extern "C" fn(event: InputEvent),
    pub on_resize: extern "C" fn(width: u32, height: u32),
    
    // Micro-Kernel → Display Server
    pub render_command_bar: extern "C" fn(commands: *const RenderCommand, count: usize),
    pub render_suggestions: extern "C" fn(items: *const SuggestionItem, count: usize),
}

#[repr(C)]
pub enum InputEvent {
    KeyDown { key: u32, modifiers: u32 },
    KeyUp { key: u32, modifiers: u32 },
    CharInput { codepoint: u32 },
    MouseDown { x: f32, y: f32, button: u8 },
    MouseUp { x: f32, y: f32, button: u8 },
    MouseMove { x: f32, y: f32 },
}

#[repr(C)]
pub struct RenderCommand {
    pub primitive_type: u8, // 0=rect, 1=text, 2=image, 3=border
    pub x: f32, pub y: f32, pub w: f32, pub h: f32,
    pub color: [f32; 4],
    pub text: *const u8, // UTF-8, null-terminated
    pub font_size: f32,
    pub radius: [f32; 4],
}

#[repr(C)]
pub struct SuggestionItem {
    pub icon: u32, // emoji/codepoint
    pub text: *const u8, // UTF-8
    pub description: *const u8,
    pub highlighted: bool, // первый элемент
}
```

**Zero-copy ABI:**
- `SharedArrayBuffer` между Rust и Bun для bulk данных (текстуры, large buffers)
- `std::sync::atomic` для сигнализации (новый кадр готов, новый ввод пришёл)
- `bun:ffi` для вызова Rust функций из TS

**Альтернатива (если FFI сложен):** IPC через Unix domain socket / named pipe:
```typescript
// Micro-Kernel (Bun)
const ipc = await Bun.connect({ unix: "/tmp/core-os.ipc" });
ipc.write(JSON.stringify({ type: "render_commands", commands: [...] }));
```

### 2.2 Input Router

```typescript
// micro-kernel/src/command-bar/input-router.ts
export class InputRouter {
    private patterns: Pattern[] = [
        { regex: /^@/, mode: Mode.Messenger },
        { regex: /^@\w+\.\w+/, mode: Mode.Messenger }, // email
        { regex: /^\+\d/, mode: Mode.Messenger },
        { regex: /^(\$|>)\s*/, mode: Mode.Terminal },
        { regex: /\.(com|ru|org|net|io)\b/i, mode: Mode.Browser },
        { regex: /^[\d+\-*/^().\s]+$/, mode: Mode.Calculator },
        { regex: /(напомни|завтра|через\s+\d+)/i, mode: Mode.Reminder },
    ];
    
    detectMode(input: string): Mode {
        for (const p of this.patterns) {
            if (p.regex.test(input)) return p.mode;
        }
        return Mode.Search; // fallback
    }
    
    getTop3Alternatives(input: string): { mode: Mode; confidence: number }[] {
        // Для неуверенных случаев
        return this.patterns
            .map(p => ({ mode: p.mode, confidence: this.score(input, p.regex) }))
            .sort((a, b) => b.confidence - a.confidence)
            .slice(0, 3);
    }
}

export enum Mode {
    Search = 0,
    Note = 1,
    Reminder = 2,
    Calculator = 3,
    Terminal = 4,
    Browser = 5,
    Messenger = 6,
    AiAgent = 7,
}
```

**Требования:**
- Router работает на каждый символ ввода (debounce 50 мс для performance)
- История выбора пользователя хранится в `shell_input_history` (SQLite)
- Обучение: если пользователь выбрал альтернативный режим 3+ раза для похожего ввода — confidence повышается

### 2.3 Suggestion Engine

```typescript
// micro-kernel/src/command-bar/suggestion-engine.ts
export class SuggestionEngine {
    private sources: SuggestionSource[] = [
        new AppRegistrySource(),     // совпадение по имени/тегам приложений
        new SearchIndexSource(),     // файлы, проекты (FTS5)
        new HistorySource(),         // недавние действия
    ];
    
    async getSuggestions(query: string, context: Context): Promise<Suggestion[]> {
        const results = await Promise.all(
            this.sources.map(s => s.search(query, context))
        );
        
        return this.rank(results.flat(), context);
    }
    
    private rank(suggestions: Suggestion[], context: Context): Suggestion[] {
        return suggestions.sort((a, b) => {
            if (a.exactMatch !== b.exactMatch) return a.exactMatch ? -1 : 1;
            if (a.projectContext !== b.projectContext) return a.projectContext ? -1 : 1;
            if (a.frequency !== b.frequency) return b.frequency - a.frequency;
            return b.lastUsed - a.lastUsed;
        });
    }
}

interface Suggestion {
    id: string;
    icon: string; // emoji или URL
    title: string;
    description: string;
    action: Action;
    exactMatch: boolean;
    projectContext: boolean;
    frequency: number;
    lastUsed: number; // timestamp
}
```

**Требования:**
- FTS5 поиск по SQLite (файлы, заметки, проекты)
- App Registry: список встроенных приложений (Notes, Calculator, Terminal, Files) + pinned URLs
- History: последние 1000 действий пользователя
- Ранжирование: exact match > project context > frequency > recency
- Задержка: < 16 мс (1 кадр) для query < 10 символов

### 2.4 SQLite схема

```sql
-- micro-kernel/schema.sql
CREATE TABLE shell_settings (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);

CREATE TABLE shell_input_history (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    input TEXT NOT NULL,
    mode INTEGER NOT NULL, -- Mode enum
    confidence REAL NOT NULL DEFAULT 1.0,
    created_at INTEGER NOT NULL -- unix timestamp
);

CREATE TABLE search_index (
    id INTEGER PRIMARY KEY,
    type TEXT NOT NULL, -- 'file', 'project', 'note', 'contact'
    title TEXT NOT NULL,
    content TEXT,
    tags TEXT, -- JSON array
    project_id TEXT,
    fts_tokens TEXT -- FTS5 virtual table
);

CREATE VIRTUAL TABLE search_fts USING fts5(
    title, content, tags,
    content='search_index',
    content_rowid='id'
);
```

### 2.5 Режимы (реализация на этапе 2)

#### Режим Поиск (Search)
```typescript
class SearchModeHandler implements ModeHandler {
    async execute(query: string): Promise<SearchResult[]> {
        const results = await db.query(`
            SELECT * FROM search_index
            WHERE id IN (
                SELECT rowid FROM search_fts
                WHERE search_fts MATCH ?
                ORDER BY rank
                LIMIT 10
            )
        `).all(query);
        return results;
    }
}
```

#### Режим Заметка (Note)
```typescript
class NoteModeHandler implements ModeHandler {
    async execute(content: string): Promise<void> {
        const tags = this.extractTags(content); // #хештеги
        await db.run(`
            INSERT INTO notes (id, content, project_id, tags, created_at)
            VALUES (?, ?, ?, ?, ?)
        `, [generateId(), content, currentProjectId, JSON.stringify(tags), Date.now()]);
    }
    
    private extractTags(content: string): string[] {
        return [...content.matchAll(/#(\w+)/g)].map(m => m[1]);
    }
}
```

#### Режим Калькулятор (Calculator)
```typescript
class CalculatorModeHandler implements ModeHandler {
    private parser = new MathParser();
    
    execute(expression: string): CalcResult {
        const result = this.parser.evaluate(expression);
        return {
            display: this.formatResult(result),
            value: result,
            history: this.saveToHistory(expression, result)
        };
    }
}

// MathParser: поддержка +, -, *, /, ^, sin, cos, tan, log, ln, sqrt, pi, e
// Префиксная / инфиксная нотация через shunting-yard algorithm
```

#### Режим Терминал (Terminal)
```typescript
class TerminalModeHandler implements ModeHandler {
    private pty: PtySession | null = null;
    
    async execute(command: string): Promise<string> {
        // Убираем префикс $ или >
        const cmd = command.replace(/^(\$|>)\s*/, '');
        
        // Через Host Shim PTY
        const result = await hostShim.exec(cmd);
        return result.stdout;
    }
}
```

#### Placeholder режимы (будут реализованы позже)
- **Browser** — открывает URL во внешнем браузере (этап 4: Island Mode)
- **Messenger** — показывает placeholder "Мессенджер в разработке" (этап 6)
- **Reminder** — сохраняет текстовую заметку с пометкой "напоминание" (этап 3: Scheduler)
- **AI Agent** — fallback к поиску (этап 9: Semantic Kernel)

### 2.6 Рендеринг Command Bar

```typescript
// micro-kernel/src/command-bar/renderer.ts
export class CommandBarRenderer {
    render(state: BarState): RenderCommand[] {
        const commands: RenderCommand[] = [];
        
        // Фон строки
        commands.push(this.drawRect(
            state.x, state.y, state.width, state.height,
            state.settings.bar_bg_color,
            state.settings.bar_radius
        ));
        
        // Иконка режима
        commands.push(this.drawText(
            state.x + 16, state.y + state.height / 2 + 6,
            state.modeIcon, 20
        ));
        
        // Текст ввода
        commands.push(this.drawText(
            state.x + 48, state.y + state.height / 2 + 6,
            state.input + '|', // курсор
            state.settings.font_size
        ));
        
        // Подсказки (выпадающий список)
        if (state.suggestions.length > 0) {
            const listY = state.y - state.suggestions.length * 44 - 8;
            commands.push(this.drawRect(
                state.x, listY, state.width, state.suggestions.length * 44 + 8,
                '#1a1a1a', [8, 8, 0, 0]
            ));
            
            state.suggestions.forEach((s, i) => {
                const y = listY + 8 + i * 44;
                if (i === state.selectedIndex) {
                    commands.push(this.drawRect(state.x + 4, y, state.width - 8, 40, '#333', [4; 4]));
                }
                commands.push(this.drawText(state.x + 16, y + 26, `${s.icon} ${s.title}`, 16));
            });
        }
        
        return commands;
    }
}
```

---

## Шаги реализации

### Шаг 2.1: Интеграция Bun в Rust (5 дней)

1. Статическая линковка Bun runtime или embedding через `bun:ffi`
2. Запуск Bun process из Rust с IPC channel
3. Протокол IPC: JSON messages через Unix socket / named pipe
4. Тест: Rust отправляет "ping", Bun отвечает "pong"

### Шаг 2.2: SQLite в Bun (2 дня)

1. `bun:sqlite` — создание БД `~/.core/shell.db`
2. Выполнение `schema.sql`
3. CRUD операции через TypeScript API
4. Тест: вставка/выборка 1000 записей < 10 мс

### Шаг 2.3: Input Router (3 дня)

1. Реализация всех regex паттернов
2. Тесты: 100 тестовых строк → правильный режим
3. История + обучение (confidence tracking)
4. Тест: после 3 выборов альтернативы — она становится приоритетной

### Шаг 2.4: Suggestion Engine (4 дня)

1. FTS5 виртуальная таблица
2. Источники: App Registry (in-memory список), Search Index (FTS5), History (SQLite)
3. Ранжирование (exact match, context, frequency, recency)
4. Тест: query "note" → Notes app, затем файлы с "note" в названии

### Шаг 2.5: Mode Handlers (5 дней)

1. Search: FTS5 query
2. Note: INSERT в SQLite + tag extraction
3. Calculator: MathParser (shunting-yard)
4. Terminal: Host Shim PTY exec
5. Placeholder handlers для Browser, Messenger, Reminder, AI Agent

### Шаг 2.6: Command Bar Renderer (4 дня)

1. Layout calculation (position, width, height из shell_settings)
2. RenderCommand generation
3. Suggestions dropdown (position, scroll, highlight)
4. Cursor blink animation (timer-based)

### Шаг 2.7: Интеграция и тестирование (4 дней)

1. Полная цепочка: ввод → Router → Mode → Render → Display Server
2. Интерактивный тест: пользователь набирает текст, видит подсказки, выбирает
3. Performance test: 60 FPS при наборе текста + подсказки
4. Кросс-платформенный тест: Windows, macOS, Linux

---

## Критерии приёмки

- [ ] Command Bar отображается внизу экрана (по умолчанию)
- [ ] Набор текста: мгновенное отображение символов (< 16 мс)
- [ ] Режим определяется автоматически по паттерну (8 тестов)
- [ ] Подсказки появляются за < 50 мс после остановки ввода
- [ ] Режимы работают: Search (FTS5), Note (SQLite), Calculator (eval), Terminal (exec)
- [ ] Placeholder режимы показывают понятное сообщение
- [ ] Настройки строки сохраняются в SQLite и применяются
- [ ] История ввода сохраняется и влияет на ранжирование
- [ ] 60 FPS при работе со строкой

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Browser открывает внешний браузер | Этап 4 | Island Mode (Chromium) |
| Messenger — заглушка | Этап 6 | P2P чат |
| Reminder — текстовая заметка | Этап 3 | Scheduler + Push Notification |
| AI Agent — fallback к поиску | Post-release | Semantic Kernel |
| Contact Book — пустой | Этап 6 | Загрузка контактов |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Input Router | layer-8 §1.1, layer-1 (8 режимов строки) |
| Suggestion Engine | layer-8 §1.2, layer-1 (подсказки при вводе) |
| Mode Handlers | layer-8 §1.3, layer-1 (калькулятор, терминал, браузер) |
| SQLite schema | layer-8 §1.4, layer-3 (Data Cache) |
| Bun Runtime | layer-3 (V8 Isolates), layer-8 §4.1 |

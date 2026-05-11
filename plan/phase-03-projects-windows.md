# Этап 3 — Проекты и окна (Project Manager + Window Manager)

> **Цель:** Система поддерживает проекты (рабочие столы) с окнами внутри. Есть Home Project, можно создавать проекты, переключаться между ними, размещать окна (тайловый и плавающий layout). Layout сохраняется в SQLite.

**После этого этапа:** пользователь видит Home Project с иконками проектов, может создать новый проект, открыть его, разместить окна (drag-and-drop, snap), переключаться между проектами. Есть базовые встроенные приложения: Notes, Calculator, Terminal, Files.

---

## Зависимости

- **Этап 1 (Фундамент):** Display Server, Host Shim, GPU
- **Этап 2 (Command Bar):** Command Bar, SQLite, Input Router, базовые mode handlers

---

## Компоненты

### 3.1 Project Manager (Level 1, Bun)

```typescript
// micro-kernel/src/project/manager.ts
export class ProjectManager {
    private projects: Map<string, Project>;
    private activeProjectId: string;
    private homeProjectId = "home";
    
    async createProject(name: string, options?: CreateOptions): Promise<Project>;
    async switchProject(projectId: string): Promise<void>;
    async archiveProject(projectId: string): Promise<void>;
    async deleteProject(projectId: string): Promise<void>; // после 30 дней архива
    
    getActiveProject(): Project;
    getHomeProject(): Project;
    listProjects(): Project[];
    
    private async persistLayout(projectId: string, layout: Layout): Promise<void>;
    private async loadLayout(projectId: string): Promise<Layout>;
}

interface Project {
    id: string;
    name: string;
    createdAt: number;
    tags: string[];
    icon: string; // emoji или path
    color: string;
    layout: Layout;
    isHome: boolean;
    isArchived: boolean;
}

interface CreateOptions {
    tags?: string[];
    icon?: string;
    color?: string;
    ephemeral?: boolean; // true = не сохранять layout
}
```

**Home Project:**
- `id = "home"`, создаётся автоматически при первом запуске
- Не удаляется, не архивируется
- Layout = сетка иконок закреплённых проектов + Command Bar
- Имя = имя пользователя (или "Home")

**Session Persistence:**
```typescript
// Автосохранение layout каждые 5 сек или при изменении
interface Layout {
    windows: WindowLayout[];
    splitTree?: SplitNode; // для тайлового layout
    timestamp: number;
}

interface WindowLayout {
    windowId: string;
    appId: string;
    x: number; y: number; w: number; h: number;
    zIndex: number;
    state: WindowState; // active, minimized, fullscreen
}
```

### 3.2 Window Manager (Level 1 + Level 3)

```typescript
// micro-kernel/src/window/manager.ts
export class WindowManager {
    private windows: Map<string, Window>;
    private zStack: string[]; // ordered by z-index
    private focusId: string | null;
    
    createWindow(appId: string, options?: WindowOptions): Window;
    closeWindow(windowId: string): void;
    focusWindow(windowId: string): void;
    minimizeWindow(windowId: string): void;
    restoreWindow(windowId: string): void;
    setFullscreen(windowId: string, fullscreen: boolean): void;
    
    // Layout
    moveWindow(windowId: string, x: number, y: number): void;
    resizeWindow(windowId: string, w: number, h: number): void;
    snapWindow(windowId: string, direction: SnapDirection): void;
    
    // Drag & Drop
    startDrag(windowId: string, x: number, y: number): void;
    updateDrag(x: number, y: number): void;
    endDrag(): void;
}

interface Window {
    id: string;
    appId: string;
    title: string;
    x: number; y: number; w: number; h: number;
    zIndex: number;
    state: WindowState;
    isFloating: boolean; // false = тайловое
}

enum WindowState {
    Active = 'active',
    Inactive = 'inactive',
    Minimized = 'minimized',
    Fullscreen = 'fullscreen',
}

type SnapDirection = 'left' | 'right' | 'top' | 'bottom' | 'center';
```

#### 3.2.1 Тайловый layout

```
+-----------+-----------+
|           |           |
|  Window A |  Window B |
|           |           |
+-----------+-----------+
|                       |
|      Window C         |
|                       |
+-----------------------+
```

```typescript
interface SplitNode {
    direction: 'horizontal' | 'vertical';
    ratio: number; // 0.0 .. 1.0
    first: SplitNode | WindowLeaf;
    second: SplitNode | WindowLeaf;
}

interface WindowLeaf {
    type: 'window';
    windowId: string;
}
```

**Алгоритм размещения:**
1. Новое окно → сплит активного окна (direction чередуется: h, v, h, v...)
2. Перетаскивание разделителя → пересчёт ratio
3. Drag window на край экрана → snap (преобразование в split)

#### 3.2.2 Плавающий layout

- Окна свободно перемещаются
- Z-index управляет наложением
- Snap: drag к краю → примагничивание (как в Windows)

#### 3.2.3 Drag & Drop

```typescript
// State machine
enum DragState {
    Idle,
    DraggingWindow,     // перетаскивание окна
    DraggingDivider,    // перетаскивание разделителя
    DraggingToSnap,     // перетаскивание к краю (snap preview)
}

// Snap preview: полупрозрачный overlay показывает, где окно встанет
```

### 3.3 SQLite схема

```sql
CREATE TABLE projects (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    created_at INTEGER NOT NULL,
    tags TEXT, -- JSON array
    icon TEXT,
    color TEXT,
    is_home INTEGER NOT NULL DEFAULT 0,
    is_archived INTEGER NOT NULL DEFAULT 0,
    archived_at INTEGER,
    ephemeral INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE layouts (
    project_id TEXT PRIMARY KEY,
    layout_json TEXT NOT NULL, -- JSON serialized Layout
    updated_at INTEGER NOT NULL,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

CREATE TABLE windows (
    id TEXT PRIMARY KEY,
    project_id TEXT NOT NULL,
    app_id TEXT NOT NULL,
    title TEXT,
    x REAL, y REAL, w REAL, h REAL,
    z_index INTEGER,
    state TEXT, -- 'active', 'inactive', 'minimized', 'fullscreen'
    is_floating INTEGER DEFAULT 0,
    created_at INTEGER,
    FOREIGN KEY (project_id) REFERENCES projects(id)
);

-- Встроенные приложения
CREATE TABLE notes (
    id TEXT PRIMARY KEY,
    content TEXT NOT NULL,
    project_id TEXT,
    tags TEXT, -- JSON array
    created_at INTEGER,
    updated_at INTEGER
);

CREATE TABLE calculations (
    id INTEGER PRIMARY KEY,
    project_id TEXT,
    expression TEXT,
    result REAL,
    created_at INTEGER
);
```

### 3.4 Встроенные приложения (базовые)

На этом этапе реализуем 4 базовых приложения как V8 Isolates (упрощённые, без полного sandboxing — он будет на этапе 4).

#### 3.4.1 Notes (Заметки)

```typescript
// apps/notes/app.ts
export class NotesApp {
    async create(content: string, projectId: string): Promise<Note> {
        const tags = extractTags(content);
        return db.insert('notes', { content, projectId, tags, createdAt: Date.now() });
    }
    
    async list(projectId: string): Promise<Note[]> {
        return db.query('SELECT * FROM notes WHERE project_id = ? ORDER BY updated_at DESC')
            .all(projectId);
    }
    
    async search(query: string): Promise<Note[]> {
        return db.query(`
            SELECT * FROM notes 
            WHERE content LIKE ? OR tags LIKE ?
            ORDER BY updated_at DESC
        `).all(`%${query}%`, `%${query}%`);
    }
}
```

**UI:** простой текстовый редактор (textarea-like) в окне.

#### 3.4.2 Calculator (Калькулятор)

```typescript
// apps/calculator/app.ts
export class CalculatorApp {
    evaluate(expression: string): number {
        const parser = new ShuntingYardParser();
        return parser.evaluate(expression);
    }
    
    getHistory(projectId: string): Calculation[] {
        return db.query('SELECT * FROM calculations WHERE project_id = ?')
            .all(projectId);
    }
}

// Поддержка: +, -, *, /, ^, sin, cos, tan, log, ln, sqrt, pi, e, скобки
```

**UI:** кнопки + дисплей. История вычислений внизу.

#### 3.4.3 Terminal (Терминал)

```typescript
// apps/terminal/app.ts
export class TerminalApp {
    private pty: PtySession;
    
    async execute(command: string): Promise<{ stdout: string; stderr: string; code: number }> {
        return hostShim.exec(command);
    }
    
    // Или полноценный PTY для интерактивных команд
    async startPty(shell: string = '/bin/bash'): Promise<PtySession> {
        this.pty = await hostShim.createPty(shell);
        return this.pty;
    }
}
```

**UI:** консольный вывод с цветами (ANSI escape codes → цветные прямоугольники).

#### 3.4.4 Files (Файловый менеджер)

```typescript
// apps/files/app.ts
export class FilesApp {
    async list(path: string): Promise<FileEntry[]> {
        return hostShim.fs.list(path);
    }
    
    async read(path: string): Promise<Uint8Array> {
        return hostShim.fs.read(path);
    }
    
    async preview(path: string): Promise<Preview> {
        // Текст, изображение, PDF
        const meta = await hostShim.fs.metadata(path);
        if (meta.mimeType.startsWith('image/')) {
            return { type: 'image', data: await this.read(path) };
        }
        if (meta.mimeType.startsWith('text/')) {
            return { type: 'text', content: new TextDecoder().decode(await this.read(path)) };
        }
        return { type: 'unknown' };
    }
}
```

**UI:** список файлов (иконки, имена), двойной клик → открытие.

### 3.5 Рендеринг проектов и окон

```typescript
// display_server/src/project_renderer.ts
export class ProjectRenderer {
    render(project: Project, windows: Window[]): RenderCommand[] {
        const commands: RenderCommand[] = [];
        
        // Фон проекта (цвет или изображение)
        commands.push(this.drawRect(0, 0, screenW, screenH, project.color));
        
        // Если Home Project — сетка иконок
        if (project.isHome) {
            commands.push(...this.renderHomeProject(project));
        }
        
        // Окна (отсортированы по z-index)
        for (const win of windows.sort((a, b) => a.zIndex - b.zIndex)) {
            if (win.state === 'minimized') continue;
            commands.push(...this.renderWindow(win));
        }
        
        // Minimized bar (плашки свёрнутых окон)
        const minimized = windows.filter(w => w.state === 'minimized');
        if (minimized.length > 0) {
            commands.push(...this.renderMinimizedBar(minimized));
        }
        
        return commands;
    }
    
    private renderWindow(win: Window): RenderCommand[] {
        const cmds: RenderCommand[] = [];
        
        // Тень (опционально)
        // cmds.push(this.drawShadow(win.x, win.y, win.w, win.h));
        
        // Фон окна
        cmds.push(this.drawRect(win.x, win.y, win.w, win.h, '#2a2a2a', [8, 8, 8, 8]));
        
        // Заголовок
        cmds.push(this.drawRect(win.x, win.y, win.w, 32, '#1a1a1a', [8, 8, 0, 0]));
        cmds.push(this.drawText(win.x + 12, win.y + 22, win.title, 14, '#fff'));
        
        // Кнопки заголовка (свернуть, закрыть)
        cmds.push(this.drawCircle(win.x + win.w - 40, win.y + 16, 6, '#ff5f57')); // close
        cmds.push(this.drawCircle(win.x + win.w - 60, win.y + 16, 6, '#ffbd2e')); // minimize
        
        // Контент окна (placeholder — заполненный цвет)
        cmds.push(this.drawRect(win.x + 4, win.y + 36, win.w - 8, win.h - 40, '#333', [0, 0, 4, 4]));
        
        // Рамка (если активное)
        // cmds.push(this.drawBorder(win.x, win.y, win.w, win.h, '#4a9eff', 2));
        
        return cmds;
    }
}
```

---

## Шаги реализации

### Шаг 3.1: Project Manager (4 дня)

1. SQLite schema (projects, layouts)
2. CRUD операции для проектов
3. Home Project (auto-create)
4. Session persistence (layout JSON → SQLite)
5. Тест: создание 10 проектов, переключение, архивация

### Шаг 3.2: Window Manager (5 дней)

1. Window model (state machine)
2. Z-index management
3. Focus management
4. Create / close / minimize / restore / fullscreen
5. Тест: создание 5 окон, переключение фокуса, z-order

### Шаг 3.3: Layout Engine (5 дней)

1. Тайловый layout (split tree)
2. Плавающий layout (x, y, w, h)
3. Drag & Drop (state machine)
4. Snap (примагничивание к краям)
5. Layout persistence (JSON → SQLite)
6. Тест: drag window к краю → snap, resize splitter

### Шаг 3.4: Встроенные приложения (7 дней)

1. Notes: CRUD, tag extraction, search
2. Calculator: MathParser (shunting-yard), history
3. Terminal: PTY через Host Shim, ANSI colors
4. Files: list, read, preview (text + image)
5. App Registry (in-memory список 4 приложений)
6. Тест: каждое приложение открывается в окне и работает

### Шаг 3.5: Рендеринг (4 дня)

1. ProjectRenderer (фон, окна, Home Project)
2. Window chrome (заголовок, кнопки, рамка)
3. Minimized bar
4. Home Project grid (иконки проектов)
5. Тест: 60 FPS при 10 окнах

### Шаг 3.6: Интеграция (4 дня)

1. Command Bar + Project Manager (строка внутри проекта)
2. Переключение проектов через Command Bar / горячие клавиши
3. Создание проекта через Command Bar («новый проект»)
4. Интерактивный тест: пользователь создаёт проекты, открывает приложения

---

## Критерии приёмки

- [ ] Home Project отображается при запуске
- [ ] Создание проекта через Command Bar
- [ ] Переключение между проектами (Command Bar / горячие клавиши)
- [ ] 4 базовых приложения работают: Notes, Calculator, Terminal, Files
- [ ] Окна: создание, закрытие, сворачивание, фокус, z-index
- [ ] Layout: тайловый и плавающий режимы
- [ ] Drag & Drop: перетаскивание окон, resize разделителей
- [ ] Snap: примагничивание к краям экрана
- [ ] Layout сохраняется при переключении проектов
- [ ] 60 FPS при 10 окнах на экране

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Нет sandboxing для приложений | Этап 4 | V8 Isolates, capability-based security |
| App Registry — in-memory список | Этап 4 | SQLite + catalog + установка |
| Files — только локальная ФС | Этап 5 | VFS + CRDT + lazy load |
| Notes — без AI-тегов | Post-release | Semantic Kernel для авто-тегов |
| Terminal — без истории команд | Этап 8 | Audit + history |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Project Manager | layer-8 §2, layer-1 (проекты, Home Project) |
| Window Manager | layer-8 §3.1, §3.4, §3.6, layer-1 (окна) |
| Layout Engine | layer-8 §2.4, §3.4, layer-1 (layout) |
| Встроенные приложения | layer-6 (уровни 1-4), layer-1 (приложения) |
| Session Persistence | layer-8 §2.2, §4.3.1 (Warm Recovery) |

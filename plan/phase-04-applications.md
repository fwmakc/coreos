# Этап 4 — App Runtime (Приложения)

> **Цель:** Система поддерживает 5 уровней интеграции приложений. Работают Island Mode (Chromium sandbox), V8 Isolates, App Registry, Permissions UI, Sandboxing (capability-based security). Можно устанавливать приложения из магазина или по адресу.

**После этого этапа:** пользователь устанавливает приложение из `pkg.core.app`, оно запускается в V8 Isolate или Island Mode, запрашивает права через Permissions UI, работает в sandbox с доступом только к разрешённым ресурсам.

---

## Зависимости

- **Этап 1 (Фундамент):** Display Server, Host Shim, GPU
- **Этап 2 (Command Bar):** SQLite, Command Bar (для запуска приложений)
- **Этап 3 (Проекты):** Window Manager, Project Manager, базовые приложения

---

## Компоненты

### 4.1 Island Mode (Level 2)

Island Mode — Chromium sandbox для веб-контента (уровни 1–2).

```rust
// app_runtime/src/island/mod.rs
pub struct IslandProcess {
    process_id: u64,
    webview: WebView, // Chromium Embedded Framework или аналог
    sandbox: SandboxConfig,
    window_id: String, // связь с Window Manager
}

impl IslandProcess {
    pub fn new(url: &str, window_id: &str, config: IslandConfig) -> Result<Self, IslandError>;
    pub fn navigate(&mut self, url: &str);
    pub fn inject_js(&mut self, code: &str); // window.__CORE_OS__ injection
    pub fn resize(&mut self, w: u32, h: u32);
    pub fn handle_input(&mut self, event: InputEvent);
    pub fn take_bitmap(&mut self) -> Vec<u8>; // RGBA для Display Server
    pub fn destroy(self);
}

struct IslandConfig {
    incognito: bool,
    devtools: bool,
    user_agent: String,
}
```

**Технические требования:**
- **Движок:** Chromium Embedded Framework (CEF) или WebKitGTK (Linux) / WebView2 (Windows) / WKWebView (macOS)
- **Sandbox:** отдельный OS process, seccomp (Linux), App Sandbox (macOS), LPAC (Windows)
- **Рендеринг:** offscreen rendering → bitmap → Display Server текстура
- **Ввод:** проброс событий от Host Shim → Island Process
- **Injection:** `window.__CORE_OS__` с полями `version`, `level`, `appId`, `theme`, `locale`

**Offscreen rendering:**
```rust
// CEF: CefWindowInfo::SetAsWindowless
// WebKitGTK: webkit_web_view_set_draw_background(false) + snapshot
// WebView2: ICoreWebView2Controller::put_Bounds + CapturePreview
// Result: RGBA bitmap (width x height x 4 bytes)
```

### 4.2 V8 Isolate Runtime (Level 2)

V8 Isolate — sandbox для нативных приложений CORE OS (уровни 3–5).

```rust
// app_runtime/src/isolate/mod.rs
pub struct V8Isolate {
    isolate_id: u64,
    isolate: v8::OwnedIsolate,
    context: v8::Global<v8::Context>,
    resource_constraints: ResourceConstraints,
    capabilities: CapabilityContext,
}

impl V8Isolate {
    pub fn new(app_id: &str, code: &[u8], capabilities: CapabilityContext) -> Result<Self, IsolateError>;
    pub fn execute(&mut self, entry_point: &str) -> Result<v8::Local<v8::Value>, IsolateError>;
    pub fn call_method(&mut self, object: &str, method: &str, args: &[v8::Local<v8::Value>]) -> Result<v8::Local<v8::Value>, IsolateError>;
    pub fn checkpoint(&mut self) -> Vec<u8>; // сериализация state
    pub fn restore(&mut self, checkpoint: &[u8]);
    pub fn terminate(&mut self);
    
    // Мониторинг
    pub fn heap_stats(&self) -> HeapStatistics;
    pub fn cpu_time_ms(&self) -> u64;
}

struct ResourceConstraints {
    max_old_generation_size: usize, // MB
    max_young_generation_size: usize, // MB
    cpu_quota: f32, // 0.0 .. 1.0
    external_memory_limit: usize, // MB
}
```

**Bun integration:**
```typescript
// micro-kernel/src/app-runtime/isolate-host.ts
export class IsolateHost {
    async createIsolate(appId: string, manifest: CoreJson): Promise<Isolate> {
        const isolate = await Bun. Isolate.create({
            memoryLimit: manifest.memory_limit || 128, // MB
            cpuQuota: manifest.cpu_quota || 0.5,
        });
        
        // Загрузка кода
        const code = await this.loadCode(appId, manifest);
        await isolate.evaluate(code);
        
        // Установка capability context
        await isolate.global.set('__CORE_OS_CONTEXT__', this.buildContext(manifest));
        
        return isolate;
    }
    
    private buildContext(manifest: CoreJson): CapabilityContext {
        return {
            fs: manifest.permissions.includes('fs') ? { read: ['/**'], write: ['/**'] } : null,
            network: manifest.permissions.includes('network') ? { domains: ['*'] } : null,
            graphics: manifest.permissions.includes('graphics'),
            contacts: manifest.permissions.includes('contacts'),
            notifications: manifest.permissions.includes('notifications'),
        };
    }
}
```

**Memory limits:**
- `max_old_generation_size` → при превышении `TerminateExecution()` + уведомление
- `max_young_generation_size` → форсированный minor GC
- `cpu_quota` → throttle до базового приоритета
- `external_memory_limit` → блокировка новых allocation

### 4.3 App Registry

```typescript
// micro-kernel/src/app-runtime/registry.ts
export class AppRegistry {
    private db: Database;
    
    async install(url: string): Promise<AppManifest> {
        // Скачивание
        const packageData = await fetch(url);
        
        // Валидация
        const manifest = await this.validateManifest(packageData);
        
        // Распаковка
        const appPath = `~/.core/apps/installed/${manifest.id}@${manifest.version}`;
        await this.extract(packageData, appPath);
        
        // Регистрация
        await this.db.run(`
            INSERT INTO app_registry (id, name, version, path, level, permissions)
            VALUES (?, ?, ?, ?, ?, ?)
        `, [manifest.id, manifest.name, manifest.version, appPath, manifest.level, JSON.stringify(manifest.permissions)]);
        
        return manifest;
    }
    
    async uninstall(appId: string): Promise<void> {
        // Удаление директории
        await hostShim.fs.remove(`~/.core/apps/installed/${appId}`);
        // Удаление из registry
        await this.db.run('DELETE FROM app_registry WHERE id = ?', [appId]);
    }
    
    async list(): Promise<AppEntry[]> {
        return this.db.query('SELECT * FROM app_registry').all();
    }
    
    async get(appId: string): Promise<AppEntry | null> {
        return this.db.query('SELECT * FROM app_registry WHERE id = ?').get(appId);
    }
    
    private async validateManifest(data: Uint8Array): Promise<AppManifest> {
        const manifest = JSON.parse(await this.extractFile(data, 'core.json'));
        
        // Обязательные поля
        if (!manifest.name || !manifest.version) {
            throw new Error('Invalid manifest: missing required fields');
        }
        
        // Проверка подписи (Ed25519)
        if (manifest.signature) {
            const valid = await crypto.verifyEd25519(manifest, manifest.signature);
            if (!valid) throw new Error('Invalid signature');
        }
        
        return manifest;
    }
}

interface AppManifest {
    id: string;
    name: string;
    version: string;
    level: number; // 1-5
    permissions: string[];
    entry?: string;
    frontend?: string;
    backend?: string;
    port?: number;
    network_whitelist?: string[];
}
```

**SQLite schema:**
```sql
CREATE TABLE app_registry (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    version TEXT NOT NULL,
    path TEXT NOT NULL,
    level INTEGER NOT NULL,
    permissions TEXT, -- JSON array
    installed_at INTEGER,
    updated_at INTEGER,
    signature TEXT
);

CREATE TABLE app_cache (
    app_id TEXT PRIMARY KEY,
    code_hash TEXT,
    last_used INTEGER,
    FOREIGN KEY (app_id) REFERENCES app_registry(id)
);
```

### 4.4 Permissions UI

```typescript
// micro-kernel/src/app-runtime/permissions.ts
export class PermissionsUI {
    async requestPermission(appId: string, permission: string, details?: PermissionDetails): Promise<PermissionResult> {
        // Проверка: уже разрешено?
        const existing = await this.db.query(
            'SELECT * FROM permissions WHERE app_id = ? AND permission = ?'
        ).get(appId, permission);
        
        if (existing) return existing.granted ? 'granted' : 'denied';
        
        // Показать модальное окно
        const result = await this.showModal({
            appName: await this.getAppName(appId),
            permission,
            details,
            options: ['Разрешить', 'Запретить', 'Разрешить один раз'],
        });
        
        // Сохранение
        await this.db.run(`
            INSERT INTO permissions (app_id, permission, granted, scope, created_at)
            VALUES (?, ?, ?, ?, ?)
        `, [app_id, permission, result.granted, JSON.stringify(result.scope), Date.now()]);
        
        return result;
    }
    
    private async showModal(config: ModalConfig): Promise<ModalResult> {
        // RenderCommand для модального окна
        const commands = this.renderModal(config);
        await displayServer.showOverlay(commands);
        
        // Ожидание выбора пользователя
        return new Promise((resolve) => {
            inputRouter.once('confirm', (choice) => resolve(choice));
        });
    }
}

enum PermissionResult {
    Granted = 'granted',
    Denied = 'denied',
    OneTime = 'one_time',
}
```

**SQLite schema:**
```sql
CREATE TABLE permissions (
    id INTEGER PRIMARY KEY,
    app_id TEXT NOT NULL,
    permission TEXT NOT NULL,
    granted INTEGER NOT NULL, -- 0/1
    scope TEXT, -- JSON: { read: [...], write: [...], domains: [...] }
    created_at INTEGER,
    UNIQUE(app_id, permission)
);
```

### 4.5 Sandboxing (Capability-based)

```typescript
// micro-kernel/src/app-runtime/sandbox.ts
export class Sandbox {
    constructor(private capabilities: CapabilityContext) {}
    
    // FS wrapper
    async readFile(path: string): Promise<Uint8Array> {
        if (!this.capabilities.fs?.read) {
            throw new PermissionError('fs.read not granted');
        }
        if (!this.matchScope(path, this.capabilities.fs.read)) {
            throw new PermissionError(`Path ${path} not in read scope`);
        }
        return hostShim.fs.read(path);
    }
    
    async writeFile(path: string, data: Uint8Array): Promise<void> {
        if (!this.capabilities.fs?.write) {
            throw new PermissionError('fs.write not granted');
        }
        if (!this.matchScope(path, this.capabilities.fs.write)) {
            throw new PermissionError(`Path ${path} not in write scope`);
        }
        return hostShim.fs.write(path, data);
    }
    
    // Network wrapper
    async fetch(url: string): Promise<Response> {
        if (!this.capabilities.network) {
            throw new PermissionError('network not granted');
        }
        const domain = new URL(url).hostname;
        if (!this.matchDomain(domain, this.capabilities.network.domains)) {
            throw new PermissionError(`Domain ${domain} not in whitelist`);
        }
        return globalThis.fetch(url);
    }
    
    private matchScope(path: string, patterns: string[]): boolean {
        return patterns.some(p => minimatch(path, p));
    }
    
    private matchDomain(domain: string, patterns: string[]): boolean {
        return patterns.some(p => domain.endsWith(p.replace('*.', '')));
    }
}
```

### 4.6 App-scoped SQLite (уровни 3–5)

```typescript
// micro-kernel/src/app-runtime/app-db.ts
export class AppDatabase {
    private db: Database;
    
    constructor(appId: string) {
        const dbPath = `~/.core/apps/installed/${appId}/data.db`;
        this.db = new Database(dbPath);
    }
    
    query(sql: string, params?: any[]): any[] {
        return this.db.query(sql).all(...(params || []));
    }
    
    run(sql: string, params?: any[]): void {
        this.db.run(sql, ...(params || []));
    }
    
    // CRDT-синхронизация (этап 5)
    async sync(): Promise<void> {
        // placeholder
    }
}
```

### 4.7 Warm Recovery (начало)

```typescript
// micro-kernel/src/app-runtime/checkpoint.ts
export class CheckpointManager {
    private interval: number = 5000; // 5 сек
    
    startCheckpointing(isolate: Isolate): void {
        setInterval(async () => {
            const state = await isolate.callMethod('global', '__getCheckpointState', []);
            const blob = JSON.stringify(state);
            await this.db.run(`
                INSERT INTO checkpoints (isolate_id, blob, timestamp)
                VALUES (?, ?, ?)
                ON CONFLICT(isolate_id) DO UPDATE SET blob=excluded.blob, timestamp=excluded.timestamp
            `, [isolate.id, blob, Date.now()]);
        }, this.interval);
    }
    
    async restore(isolate: Isolate): Promise<void> {
        const row = await this.db.query(
            'SELECT blob FROM checkpoints WHERE isolate_id = ? ORDER BY timestamp DESC LIMIT 1'
        ).get(isolate.id);
        
        if (row) {
            const state = JSON.parse(row.blob);
            await isolate.callMethod('global', '__restoreCheckpointState', [state]);
        }
    }
}
```

---

## Шаги реализации

### Шаг 4.1: Island Mode (CEF/WebView) (7 дней)

1. Интеграция CEF / WebView2 / WKWebView
2. Offscreen rendering (bitmap capture)
3. Input injection (keyboard, mouse)
4. `window.__CORE_OS__` injection
5. Sandbox (отдельный OS process)
6. Тест: открыть `example.com`, увидеть страницу в окне CORE OS

### Шаг 4.2: V8 Isolate Runtime (5 дней)

1. Bun Isolate API (или встроенный V8)
2. Resource constraints (memory, CPU)
3. Загрузка и выполнение JS кода
4. Межпроцессное взаимодействие (Isolate → Display Server)
5. Тест: простое приложение "Hello World" в окне

### Шаг 4.3: App Registry (4 дня)

1. SQLite schema
2. Скачивание и валидация `core.json`
3. Распаковка в `~/.core/apps/installed/`
4. Подпись (Ed25519) — placeholder, проверка в будущем
5. Тест: установка приложения, запуск, удаление

### Шаг 4.4: Permissions UI (3 дня)

1. Модальное окно (RenderCommand)
2. Запрос разрешений при первом запуске
3. Сохранение в SQLite
4. Проверка перед API-вызовами
5. Тест: приложение запрашивает fs → модальное окно → работает/не работает

### Шаг 4.5: Sandboxing (4 дня)

1. Capability context builder
2. FS wrapper (read/write scope)
3. Network wrapper (domain whitelist)
4. Security Hooks (BeforeApiCall, BeforeNetworkRequest)
5. Тест: приложение без прав → все вызовы падают с PermissionError

### Шаг 4.6: App-scoped DB + Checkpoint (2 дня)

1. `~/.core/apps/<appId>/data.db`
2. Checkpoint каждые 5 сек
3. Restore при перезапуске
4. Тест: приложение падает → восстановление из checkpoint

### Шаг 4.7: Интеграция (4 дней)

1. Command Bar → `установить <url>` → App Registry
2. Command Bar → `запустить <app>` → Window Manager + Isolate/Island
3. Все 5 уровней: тестовые приложения каждого уровня
4. Performance test: 10 приложений одновременно, 60 FPS

---

## Критерии приёмки

- [ ] Уровень 1: любой URL открывается в Island Mode
- [ ] Уровень 2: сайт с `core.json` → standalone окно, иконка, push (placeholder)
- [ ] Уровень 3: приложение с backend запускается (V8 Isolate + Island)
- [ ] Уровень 4: `@core/*` API работает (fs, network через sandbox)
- [ ] Уровень 5: нативное приложение на WebGPU (примитивы через `@core/graphics`)
- [ ] App Registry: установка, список, удаление (без следов)
- [ ] Permissions UI: модальное окно, сохранение, проверка
- [ ] Sandboxing: приложение без прав не может читать файлы/сеть
- [ ] Warm Recovery: checkpoint → restore за <100 мс
- [ ] 60 FPS при 5 приложениях одновременно

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| CEF — только базовый рендеринг | Post-release | DevTools, WebGL в Island Mode |
| Push notifications (level 2) | Этап 6 | Notification Engine |
| CRDT-синхронизация app DB | Этап 5 | Sync Engine |
| Ed25519 подпись пакетов | Этап 8 | Key Manager + Supply Chain |
| `@core/graphics` WebGPU | Этап 1 + Post-release | Полный API для level 5 |
| `@core/ui` компоненты | Post-release | Button, TextInput, Card |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Island Mode | layer-8 §3.2, §4.1, layer-6 (уровни 1-2) |
| V8 Isolates | layer-8 §4.1, layer-6 (уровни 3-5) |
| App Registry | layer-8 §4.2, layer-6 §1.9 |
| Permissions UI | layer-8 §4.5, layer-6 §1.8, layer-1 (разрешения) |
| Sandboxing | layer-8 §4.4, layer-6 §1.10, layer-7 (security) |
| Warm Recovery | layer-8 §4.3.1, layer-1 (восстановление) |

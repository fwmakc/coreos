# Этап 9 — Релизная полировка (Game Mode, Energy, Migration, Docs)

> **Цель:** Система готова к публичному релизу. Game Mode, Energy Manager, публичный P2P CDN, Store (`pkg.core.app`), migration tools, документация, стресс-тесты.

**После этого этапа:** CORE OS — полнофункциональная система, которую можно установить и использовать. Game Mode работает, батарея контролируется, Store устанавливает приложения, документация помогает пользователям и разработчикам.

---

## Зависимости

- Все предыдущие этапы (1–8)

---

## Компоненты

### 9.1 Game Mode

```rust
// display/src/game_mode.rs
pub struct GameMode {
    direct_surface: DirectSurface,
    shadow_framebuffer: ShadowFramebuffer,
    input_exclusivity: InputExclusivity,
}

impl GameMode {
    pub fn enter(&mut self, game_app: &AppInstance) -> Result<(), GameModeError> {
        // 1. Создать Direct Surface (exclusive или borderless fullscreen)
        self.direct_surface = DirectSurface::new(
            DisplayMode::BorderlessFullscreen,
            game_app.graphics_api(), // Vulkan/DirectX/Metal
        )?;
        
        // 2. Capture framebuffer для переключения назад
        self.shadow_framebuffer.capture();
        
        // 3. Передать input напрямую в isolate
        self.input_exclusivity.grab(game_app.isolate_id());
        
        // 4. Pause Shell rendering
        display_server.pause_shell_rendering();
        
        // 5. Redirect GPU context
        gpu.redirect(game_app.gpu_context());
        
        Ok(())
    }
    
    pub fn exit(&mut self) -> Result<(), GameModeError> {
        // 1. Release GPU context
        gpu.redirect_to_shell();
        
        // 2. Resume Shell rendering
        display_server.resume_shell_rendering();
        
        // 3. Restore framebuffer from shadow
        display_server.restore_framebuffer(&self.shadow_framebuffer);
        
        // 4. Release input
        self.input_exclusivity.release();
        
        // 5. Resume background apps (со сниженным приоритетом)
        app_scheduler.resume_background(min_fps: 30);
        
        Ok(())
    }
    
    pub fn handle_panic_gesture(&mut self) -> Result<(), GameModeError> {
        // Panic Gesture всегда работает — выход из Game Mode
        self.exit()?;
        Ok(())
    }
    
    pub fn alt_tab(&mut self, direction: AltTabDirection) {
        // Shadow framebuffer отображается поверх игры
        // Пользователь выбирает приложение
        // Возврат в Shell или другое приложение
        display_server.show_switcher(
            &self.shadow_framebuffer,
            direction,
        );
    }
}
```

**Переключение контекста (Alt+Tab в Game Mode):**
```rust
// display/src/game_mode.rs
fn alt_tab_shadow(&self) {
    // 1. Создать shadow framebuffer из текущего кадра
    let shadow = self.capture_current_framebuffer();
    
    // 2. Render window switcher поверх shadow
    let switcher = WindowSwitcher::new(shadow);
    
    // 3. Handle input (arrow keys / mouse)
    let selected = switcher.wait_for_selection();
    
    // 4. Если выбран Shell — exit Game Mode
    if selected == "shell" {
        self.exit();
    } else {
        // Переключиться на другое приложение
        self.switch_to_app(selected);
    }
}
```

### 9.2 Energy Manager

```rust
// system/src/energy.rs
pub struct EnergyManager {
    battery: BatteryMonitor,
    policy: EnergyPolicy,
}

impl EnergyManager {
    pub fn on_battery_level_change(&mut self, level: f32) {
        match level {
            0.0..0.05 => self.emergency_shutdown(),
            0.05..0.10 => self.enter_critical_mode(),
            0.10..0.20 => self.enter_power_save(),
            0.20..0.50 => self.reduce_brightness(50),
            _ => self.normal_mode(),
        }
    }
    
    fn enter_power_save(&mut self) {
        // Снижение FPS до 30
        display_server.set_target_fps(30);
        
        // Отключение тяжёлых эффектов
        display_server.disable_effects(["blur", "shadow", "animation"]);
        
        // Приостановка background sync
        p2p_mesh.pause_background_sync();
        
        // Задержка P2P announce
        p2p_mesh.set_announce_interval(300); // 5 минут
        
        // Таймаут auto-lock 60 сек
        session_manager.set_auto_lock_timeout(60);
        
        // TTS: "Включён режим энергосбережения"
        tts.speak("Включён режим энергосбережения");
    }
    
    fn enter_critical_mode(&mut self) {
        // Сохранение всех checkpoint'ов
        project_manager.save_all_checkpoints();
        
        // Закрытие всех background apps
        app_scheduler.kill_background();
        
        // Остановка P2P
        p2p_mesh.stop();
        
        // Минимальная яркость
        display_server.set_brightness(10);
        
        // Только emergency уведомления
        notification_manager.filter_priority("emergency");
        
        // TTS: "Критический уровень заряда"
        tts.speak("Критический уровень заряда. Сохраняю данные.");
    }
}
```

### 9.3 Public P2P CDN

```rust
// p2p/src/cdn.rs
pub struct PublicCDN {
    swarm: Swarm,
    content_store: ContentStore,
}

impl PublicCDN {
    pub fn publish_package(&mut self, pkg: &Package) -> Result<ContentId, CDError> {
        let content = pkg.serialize()?;
        let cid = blake3::hash(&content);
        
        // Store in DHT
        self.content_store.put(cid, content)?;
        
        // Announce to swarm
        self.swarm.announce(cid);
        
        Ok(cid)
    }
    
    pub fn resolve_package(&mut self, url: &str) -> Result<Package, CDError> {
        // URL: pkg://<cid>/package.core
        let cid = parse_pkg_url(url)?;
        
        // Try local store
        if let Some(content) = self.content_store.get(&cid) {
            return Package::deserialize(&content);
        }
        
        // Query DHT
        let providers = self.swarm.find_providers(&cid)?;
        
        // Download from nearest provider
        for provider in providers {
            if let Ok(content) = self.swarm.download(provider, &cid) {
                // Verify hash
                if blake3::hash(&content) == cid {
                    self.content_store.put(cid, content.clone())?;
                    return Package::deserialize(&content);
                }
            }
        }
        
        Err(CDError::PackageNotFound)
    }
}
```

### 9.4 Store (`pkg.core.app`)

```typescript
// store/src/app.tsx
export function StoreApp() {
    const [category, setCategory] = useState('featured');
    const [search, setSearch] = useState('');
    
    return (
        <Window title="CORE Store" icon="🏪">
            <SearchBar value={search} onChange={setSearch} />
            <Categories onSelect={setCategory} />
            <PackageList 
                category={category} 
                search={search}
                onInstall={installPackage}
            />
        </Window>
    );
}

async function installPackage(pkg: Package) {
    // 1. Проверить зависимости
    const deps = await dependencyResolver.resolve(pkg.dependencies);
    
    // 2. Проверить права
    const permissions = pkg.requiredCapabilities;
    const result = await permissionManager.request(permissions, {
        appName: pkg.name,
        appIcon: pkg.icon,
    });
    
    if (!result.granted) {
        throw new Error('Permissions denied');
    }
    
    // 3. Скачать
    const data = await p2pCDN.resolvePackage(pkg.url);
    
    // 4. Проверить подпись
    if (!signatureManager.verify(data, pkg.publisher_key)) {
        throw new Error('Invalid signature');
    }
    
    // 5. Зарегистрировать
    await appRegistry.register(pkg, result.capabilities);
    
    // 6. Показать уведомление
    notificationManager.show({
        title: `${pkg.name} установлен`,
        body: `Нажмите, чтобы открыть`,
        action: () => projectManager.openApp(pkg.id),
    });
}
```

### 9.5 Migration Tools

```rust
// migration/src/lib.rs
pub struct MigrationEngine {
    source: MigrationSource,
    target: MigrationTarget,
}

impl MigrationEngine {
    pub fn export_data(&self, filter: ExportFilter) -> Result<ExportData, MigrationError> {
        let mut data = ExportData::new();
        
        // Projects
        if filter.projects {
            data.projects = project_manager.export_all()?;
        }
        
        // Settings
        if filter.settings {
            data.settings = settings_manager.export()?;
        }
        
        // Apps
        if filter.apps {
            data.apps = app_registry.export_installed()?;
        }
        
        // Contacts
        if filter.contacts {
            data.contacts = contact_book.export()?;
        }
        
        // Notes
        if filter.notes {
            data.notes = note_app.export_all()?;
        }
        
        // P2P Keys
        if filter.p2p_keys {
            data.p2p_keys = p2p_mesh.export_keys()?;
        }
        
        Ok(data)
    }
    
    pub fn import_data(&mut self, data: &ExportData) -> Result<(), MigrationError> {
        // Validate format version
        if data.version > CURRENT_VERSION {
            return Err(MigrationError::UnsupportedVersion);
        }
        
        // Import projects
        for project in &data.projects {
            project_manager.import(project)?;
        }
        
        // Import settings
        settings_manager.import(&data.settings)?;
        
        // Re-install apps
        for app in &data.apps {
            store.install(app)?;
        }
        
        // Import contacts
        contact_book.import(&data.contacts)?;
        
        // Import P2P keys
        p2p_mesh.import_keys(&data.p2p_keys)?;
        
        Ok(())
    }
}
```

**Формат экспорта:**
```json
{
    "version": 1,
    "exported_at": "2025-01-20T10:00:00Z",
    "device_id": "...",
    "projects": [...],
    "settings": {...},
    "apps": [...],
    "contacts": [...],
    "notes": [...],
    "p2p_keys": {...}
}
```

### 9.6 Документация

```
docs/
├── user/
│   ├── getting-started.md
│   ├── projects.md
│   ├── voice-control.md
│   ├── security.md
│   ├── troubleshooting.md
│   └── faq.md
├── developer/
│   ├── architecture.md
│   ├── app-model.md
│   ├── intent-api.md
│   ├── p2p-protocol.md
│   └── contributing.md
├── admin/
│   ├── installation.md
│   ├── backoffice.md
│   ├── hardcore.md
│   └── security-hardening.md
└── api/
    ├── intent-api-reference.md
    ├── app-manifest.md
    └── p2p-rpc.md
```

---

## Шаги реализации

### Шаг 9.1: Game Mode (5 дней)

1. Direct Surface (exclusive fullscreen)
2. Shadow framebuffer capture/restore
3. Input exclusivity
4. GPU context redirect
5. Panic Gesture в Game Mode
6. Alt+Tab shadow switcher
7. Тест: Game Mode → Shell < 100 мс, Shell → Game Mode < 33 мс

### Шаг 9.2: Energy Manager (3 дня)

1. Battery monitoring (cross-platform)
2. Power save mode (30 FPS, отключение эффектов)
3. Critical mode (сохранение, закрытие background)
4. Auto-lock timeout
5. Тест: 10% батареи → power save включается

### Шаг 9.3: Public P2P CDN (3 дня)

1. Content addressing (BLAKE3)
2. Package publishing
3. Package resolution
4. Signature verification
5. Тест: публикация → скачивание → проверка подписи

### Шаг 9.4: Store (4 дня)

1. Store UI
2. Package browsing/search
3. Install flow (deps, permissions, download, verify, register)
4. Update mechanism
5. Тест: установка приложения из Store

### Шаг 9.5: Migration Tools (3 дня)

1. Export format
2. Import format
3. Version compatibility
4. CLI: `core-cli backup --export`, `core-cli restore --import`
5. Тест: экспорт → импорт → данные на месте

### Шаг 9.6: Документация (4 дня)

1. User docs (getting started, voice, security)
2. Developer docs (architecture, app model, Intent API)
3. Admin docs (installation, Backoffice, Hardcore)
4. API reference
5. Тест: новый разработчик разворачивает систему по docs

### Шаг 9.7: Стресс-тесты (3 дня)

1. CPU load 100% → responsiveness
2. RAM pressure 95% → behavior
3. Network latency 500 мс → P2P behavior
4. Battery critical → graceful shutdown
5. 1000 users → Backoffice performance

---

## Критерии приёмки

- [ ] Game Mode: работает, переключение < 100 мс
- [ ] Energy Manager: 3 режима (normal, power save, critical)
- [ ] Public P2P CDN: публикация и скачивание пакетов
- [ ] Store: установка приложений с проверкой подписи
- [ ] Migration: экспорт/импорт всех данных
- [ ] Docs: пользователь, разработчик, админ, API reference
- [ ] Stress tests: система устойчива при нагрузке
- [ ] Full system test: установка → настройка → работа → backup → restore

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Store: только P2P CDN | Post-release | Центральный сервер для рекомендаций |
| Game Mode: только WebGPU games | Post-release | Native games (Vulkan/DirectX passthrough) |
| Migration: только full export | Post-release | Selective export, cloud backup |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Game Mode | layer-8 §3.7, layer-1 (игровой режим) |
| Energy Manager | layer-8 §4.10, layer-1 (энергосбережение) |
| Public P2P CDN | layer-8 §11, layer-5 (P2P) |
| Store | layer-8 §6.2, layer-6 (модель приложений) |
| Migration | layer-8 §4.9, layer-4 (миграция) |
| Docs | layer-11 (developer reference) |

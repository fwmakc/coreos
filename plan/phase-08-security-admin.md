# Этап 8 — Безопасность и администрирование

> **Цель:** Система защищена на всех уровнях: RBAC, аудит (13 категорий), шифрование (3 уровня), Key Manager (Ed25519, TPM/Secure Enclave), Session Management (TTL, auto-lock, remote wipe). Core.Backoffice и Core.Hardcore предоставляют полное управление.

**После этого этапа:** каждое действие логируется, Owner контролирует доступ, данные зашифрованы, устройство можно заблокировать удалённо. Администратор управляет через GUI или SSH.

---

## Зависимости

- **Этап 2 (Command Bar):** SQLite, Settings
- **Этап 4 (Приложения):** App Registry, Permissions UI, Sandboxing
- **Этап 5 (P2P):** P2P Mesh (для remote wipe, key distribution)
- **Этап 6 (Коммуникации):** Messenger (для security alerts)

---

## Компоненты

### 8.1 RBAC (Role-Based Access Control)

```rust
// security/src/rbac.rs
pub struct RBACEngine {
    roles: HashMap<String, Role>,
    user_roles: HashMap<(UserId, ResourceId), String>,
    groups: HashMap<String, Group>,
}

impl RBACEngine {
    pub fn check(&self, user_id: &UserId, resource: &Resource, action: Action) -> Result<(), AuthError> {
        let role_name = self.user_roles.get(&(user_id.clone(), resource.id.clone()))
            .or_else(|| self.get_default_role(resource));
        
        let role = self.roles.get(role_name.unwrap())
            .ok_or(AuthError::RoleNotFound)?;
        
        if !role.capabilities.contains(&action.to_capability()) {
            return Err(AuthError::InsufficientPermissions);
        }
        
        Ok(())
    }
    
    pub fn create_role(&mut self, name: &str, capabilities: Vec<Capability>) -> Role;
    pub fn assign_role(&mut self, user_id: &UserId, resource_id: &ResourceId, role: &str);
    pub fn create_group(&mut self, name: &str, members: Vec<UserId>) -> Group;
}

struct Role {
    name: String,
    capabilities: Vec<Capability>,
    inherits: Option<String>, // наследование
}

enum Capability {
    FsRead, FsWrite, FsDelete,
    NetworkHttp, NetworkTcp, NetworkUdp,
    GraphicsRender, GraphicsCapture,
    MindLocal, MindCloud, MindConfigure,
    ContactsRead, ContactsWrite,
    NotificationsSend,
    AdminUsers, AdminRoles, AdminAudit, AdminBackup,
}
```

**Встроенные роли:**
| Роль | Capabilities |
|------|-------------|
| Owner | Все |
| Member | FsRead, FsWrite, NetworkHttp, GraphicsRender, ContactsRead, NotificationsSend |
| Guest | FsRead (только свои), NetworkHttp, GraphicsRender |

**Наследование:** роль проекта ограничивает роль Space. Роль группы добавляется к роли пользователя.

### 8.2 Audit

```rust
// security/src/audit.rs
pub struct AuditEngine {
    db: Database,
    config: AuditConfig,
}

impl AuditEngine {
    pub fn log(&mut self, entry: AuditEntry) {
        if !self.config.categories.contains(&entry.category) {
            return;
        }
        
        if self.should_exclude(&entry) {
            return;
        }
        
        self.db.execute("INSERT INTO audit (...) VALUES (...)", entry).unwrap();
        
        // Проверка переполнения
        if self.db.size() > self.config.max_size_mb * 1024 * 1024 {
            self.rotate_log();
        }
    }
    
    pub fn query(&self, filter: AuditFilter) -> Vec<AuditEntry>;
    pub fn export(&self, format: ExportFormat) -> Vec<u8>;
}

struct AuditEntry {
    id: u64,
    timestamp: u64,
    category: AuditCategory,
    user_id: UserId,
    action: String,
    resource: String,
    result: ActionResult,
    details: String, // JSON
}

enum AuditCategory {
    Auth, Roles, Projects, Files, Notes, Tags,
    Messenger, Search, Apps, Browser, Profiles, System, MultiBack,
}
```

**SQLite schema:**
```sql
CREATE TABLE audit (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    timestamp INTEGER NOT NULL,
    category TEXT NOT NULL,
    user_id TEXT NOT NULL,
    action TEXT NOT NULL,
    resource TEXT,
    result TEXT, -- 'success', 'denied', 'error'
    details TEXT, -- JSON
    ip_address TEXT,
    device_id TEXT
);

CREATE INDEX idx_audit_time ON audit(timestamp);
CREATE INDEX idx_audit_category ON audit(category);
CREATE INDEX idx_audit_user ON audit(user_id);
```

### 8.3 Key Manager

```rust
// security/src/keys.rs
pub struct KeyManager {
    master_key: DerivedKey,     // from recovery phrase (BIP-39)
    device_key: KeyPair,        // Ed25519, generated on first boot
    profile_keys: HashMap<ProfileId, DerivedKey>,
    storage: KeyStorage,        // TPM / Secure Enclave / Keychain
}

impl KeyManager {
    pub fn generate_recovery_phrase() -> String {
        // BIP-39: 24 слова
        bip39::Mnemonic::generate(24).unwrap().to_string()
    }
    
    pub fn derive_master_key(phrase: &str) -> DerivedKey {
        let mnemonic = bip39::Mnemonic::from_phrase(phrase).unwrap();
        let seed = mnemonic.to_seed("");
        DerivedKey::from_seed(&seed)
    }
    
    pub fn generate_device_key(&mut self) -> KeyPair {
        let keypair = ed25519_dalek::Keypair::generate(&mut OsRng);
        self.storage.store("device_key", &keypair.to_bytes());
        keypair
    }
    
    pub fn derive_profile_key(&self, profile_id: &str) -> DerivedKey {
        // HKDF: master_key + profile_id → profile_key
        hkdf::Hkdf::<sha2::Sha256>::new(None, &self.master_key.as_bytes())
            .expand(profile_id.as_bytes(), &mut [0u8; 32])
            .unwrap();
        DerivedKey::from_bytes(&[0u8; 32]) // placeholder for actual expansion
    }
    
    pub fn sign(&self, data: &[u8], key_id: &str) -> Signature {
        let keypair = self.storage.load(key_id);
        keypair.sign(data)
    }
    
    pub fn encrypt(&self, data: &[u8], key_id: &str) -> Vec<u8> {
        // XChaCha20-Poly1305
        let key = self.storage.load(key_id);
        let nonce = XNonce::from_slice(&rand::random::<[u8; 24]>());
        let ciphertext = chacha20poly1305::XChaCha20Poly1305::new(key.into())
            .encrypt(nonce, data)
            .unwrap();
        [nonce.as_slice(), &ciphertext].concat()
    }
}
```

**Хранение ключей:**
| Платформа | Хранилище |
|-----------|-----------|
| Windows | TPM 2.0 или Credential Guard |
| macOS | Secure Enclave (Keychain) |
| Linux | TPM 2.0 (tss2) или software keyring |
| Android | Android Keystore |
| iOS | Secure Enclave |

### 8.4 Session Management

```rust
// security/src/session.rs
pub struct SessionManager {
    sessions: HashMap<SessionId, Session>,
    config: SessionConfig,
}

impl SessionManager {
    pub fn create_session(&mut self, user_id: &UserId, device_id: &DeviceId) -> Session {
        let session = Session {
            id: generate_id(),
            user_id: user_id.clone(),
            device_id: device_id.clone(),
            token: generate_token(),
            created_at: now(),
            last_activity: now(),
            status: SessionStatus::Active,
        };
        self.sessions.insert(session.id.clone(), session.clone());
        session
    }
    
    pub fn check_session(&mut self, token: &Token) -> Result<Session, AuthError> {
        let session = self.sessions.values_mut()
            .find(|s| s.token == *token)
            .ok_or(AuthError::InvalidToken)?;
        
        // TTL check
        if now() - session.created_at > self.config.ttl_minutes * 60 {
            session.status = SessionStatus::Expired;
            return Err(AuthError::SessionExpired);
        }
        
        // Auto-lock check
        if now() - session.last_activity > self.config.auto_lock_after_idle * 60 {
            session.status = SessionStatus::Locked;
            return Err(AuthError::SessionLocked);
        }
        
        session.last_activity = now();
        Ok(session.clone())
    }
    
    pub fn remote_wipe(&mut self, device_id: &DeviceId) -> Result<(), AuthError> {
        let sessions: Vec<_> = self.sessions.values()
            .filter(|s| s.device_id == *device_id)
            .map(|s| s.id.clone())
            .collect();
        
        for id in sessions {
            self.sessions.get_mut(&id).unwrap().status = SessionStatus::Revoked;
        }
        
        // Push notification to device
        self.push_wipe_command(device_id);
        
        Ok(())
    }
}

struct SessionConfig {
    ttl_minutes: u32,          // 30
    auto_lock_after_idle: u32, // 5
    require_biometry: bool,    // true для "Повышенный"
}
```

### 8.5 Core.Backoffice (GUI)

```typescript
// backoffice/src/gui/app.tsx
export function BackofficeApp() {
    const [section, setSection] = useState('users');
    
    return (
        <Window title="CORE Backoffice">
            <Sidebar>
                <NavItem icon="👥" label="Пользователи" onClick={() => setSection('users')} />
                <NavItem icon="🏢" label="Space" onClick={() => setSection('space')} />
                <NavItem icon="📦" label="Приложения" onClick={() => setSection('apps')} />
                <NavItem icon="🔒" label="Безопасность" onClick={() => setSection('security')} />
                <NavItem icon="💾" label="Бэкап" onClick={() => setSection('backup')} />
                <NavItem icon="🤖" label="AI" onClick={() => setSection('ai')} />
                <NavItem icon="🎧" label="Техподдержка" onClick={() => setSection('support')} />
            </Sidebar>
            <MainContent>
                {section === 'users' && <UsersSection />}
                {section === 'security' && <SecuritySection />}
                {/* ... */}
            </MainContent>
        </Window>
    );
}

// UsersSection
function UsersSection() {
    const users = useUsers();
    
    return (
        <>
            <Toolbar>
                <Button onClick={createUser}>Создать пользователя</Button>
                <Button onClick={createRole}>Создать роль</Button>
            </Toolbar>
            <Table data={users} columns={['Имя', 'Роль', 'Последний вход', 'Действия']} />
        </>
    );
}
```

**Корпоративный режим:**
```typescript
// При установке Бэка с профилем "Corporate"
if (config.profile === 'corporate') {
    config.allow_gui_admin = false;
    // Core.Backoffice не устанавливается
    // Единственный способ администрирования — Core.Hardcore (SSH)
}
```

### 8.6 Core.Hardcore (TUI + CLI)

```rust
// hardcore/src/tui.rs
pub struct HardcoreTUI {
    terminal: Terminal<CrosstermBackend<Stdout>>,
    state: AppState,
}

impl HardcoreTUI {
    pub fn run(&mut self) -> Result<(), io::Error> {
        loop {
            self.draw()?;
            
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('u') => self.state.section = Section::Users,
                    KeyCode::Char('s') => self.state.section = Section::Security,
                    KeyCode::Char('b') => self.state.section = Section::Backup,
                    KeyCode::Enter => self.execute_selected(),
                    KeyCode::Up => self.state.prev_item(),
                    KeyCode::Down => self.state.next_item(),
                    _ => {}
                }
            }
        }
    }
    
    fn draw(&mut self) -> Result<(), io::Error> {
        self.terminal.draw(|f| {
            match self.state.section {
                Section::Users => self.draw_users(f),
                Section::Security => self.draw_security(f),
                // ...
            }
        })
    }
}
```

**CLI команды:**
```bash
# Пользователи
core-cli user add --name "Иван" --role developer
core-cli user list
core-cli user revoke --name "Иван"

# Роли
core-cli role create --name "developer" --capabilities "fs.read,fs.write,network.http"
core-cli role assign --user "Иван" --role "developer" --resource "project-alpha"

# Бэкап
core-cli backup --full --target usb --output /mnt/backup/
core-cli backup list
core-cli restore --target usb --date 2025-01-15

# Настройки
core-cli settings set --key "security.level" --value "enhanced"
core-cli settings get --key "security.level"

# AI
core-cli ai model list
core-cli ai model set --asr "whisper-medium" --nlu "llama-3.1-8b"

# Audit
core-cli audit query --category "auth" --from "2025-01-01" --to "2025-01-31"
core-cli audit export --format json --output ./audit.json
```

---

## Шаги реализации

### Шаг 8.1: RBAC (4 дня)

1. Role model + capabilities
2. User-role assignment
3. Permission checking (before every API call)
4. Inheritance (project < space < group)
5. Тест: Guest не может читать чужие файлы

### Шаг 8.2: Audit (3 дня)

1. SQLite schema
2. Audit logging (13 categories)
3. Query и фильтры
4. Export (JSON, CSV)
5. Тест: каждое действие логируется

### Шаг 8.3: Key Manager (4 дня)

1. BIP-39 recovery phrase
2. Ed25519 key generation
3. HKDF derivation
4. TPM / Secure Enclave integration
5. Encrypt/decrypt (XChaCha20-Poly1305)
6. Тест: шифрование → расшифровка, подпись → проверка

### Шаг 8.4: Session Management (3 дня)

1. Session creation / validation
2. TTL и auto-lock
3. Remote wipe (push notification)
4. Biometry integration (FaceID / TouchID / Hello)
5. Тест: remote wipe → устройство блокируется

### Шаг 8.5: Core.Backoffice (5 дней)

1. TUI framework (ratatui или custom WebGPU)
2. Users section
3. Security section (RBAC, audit view)
4. Backup section
5. AI section
6. Тест: создание пользователя через GUI

### Шаг 8.6: Core.Hardcore (4 дня)

1. SSH server (russh)
2. TUI interface
3. CLI parser (clap)
4. All commands implementation
5. Тест: SSH + `core-cli user add`

### Шаг 8.7: Интеграция (3 дней)

1. Security hooks (BeforeApiCall, OnProfileSwitch)
2. Corp mode (`allow_gui_admin: false`)
3. Full security test (penetration test checklist)
4. Performance: audit logging < 1 мс

---

## Критерии приёмки

- [ ] RBAC: 3 роли + кастомные, наследование работает
- [ ] Audit: все 13 категорий логируются, export работает
- [ ] Key Manager: recovery phrase → master key → encryption
- [ ] Session: TTL, auto-lock, remote wipe
- [ ] Biometry: FaceID/TouchID/Hello для разблокировки
- [ ] Core.Backoffice: GUI для управления пользователями, ролями, бэкапом
- [ ] Core.Hardcore: SSH + TUI + CLI, все команды работают
- [ ] Corp mode: GUI админки не существует
- [ ] Penetration test: Guest не может эскалировать права
- [ ] Audit performance: логирование < 1 мс

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Corp mode: только блокировка GUI | Post-release | Полное hardening |
| Audit: только SQLite | Этап 10 | Elasticsearch для large scale |
| Key Manager: software fallback | Post-release | Аппаратное хранилище everywhere |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| RBAC | layer-8 §14, layer-7 (RBAC) |
| Audit | layer-8 §15, layer-7 (аудит) |
| Key Manager | layer-8 §4.1.1, layer-7 (шифрование) |
| Session Management | layer-8 §4.13, §10.3, layer-7 (сессии) |
| Core.Backoffice | layer-8 §16.2, layer-3 (администрирование) |
| Core.Hardcore | layer-8 §16.2, layer-3 (администрирование) |

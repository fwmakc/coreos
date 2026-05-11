# Этап 5 — P2P Mesh и синхронизация

> **Цель:** Устройства обнаруживают друг друга в локальной сети (mDNS), устанавливают WireGuard-туннели, синхронизируют данные через CRDT. Работает бэкап на USB/S3 и ленивая загрузка файлов.

**После этого этапа:** два устройства в одной Wi-Fi сети видят друг друга, обмениваются данными (проекты, файлы, настройки) без конфликтов. Можно отключить одно устройство, поработать оффлайн, вернуться — данные сходятся.

---

## Зависимости

- **Этап 1 (Фундамент):** Host Shim (сеть через UDP/TCP)
- **Этап 2 (Command Bar):** SQLite (CRDT-журнал)
- **Этап 3 (Проекты):** Project Manager (layout, данные для синхронизации)
- **Этап 4 (Приложения):** App Registry, app-scoped SQLite

---

## Компоненты

### 5.1 P2P Mesh (Level 2)

```rust
// p2p/src/mesh.rs
pub struct P2PMesh {
    local_keypair: Keypair, // Ed25519
    peers: HashMap<PeerId, Peer>,
    mdns: MdnsService,
    wireguard: WireGuardInterface,
    libp2p: Swarm<Behaviour>,
}

impl P2PMesh {
    pub async fn new(config: MeshConfig) -> Result<Self, MeshError>;
    
    // Discovery
    pub async fn discover_peers(&mut self) -> Vec<PeerInfo>;
    pub async fn advertise(&mut self) -> Result<(), MeshError>;
    
    // Connection
    pub async fn connect(&mut self, peer_id: &PeerId) -> Result<Connection, MeshError>;
    pub async fn disconnect(&mut self, peer_id: &PeerId);
    
    // Sync
    pub async fn sync_with(&mut self, peer_id: &PeerId, namespace: &str) -> Result<SyncResult, MeshError>;
    pub async fn broadcast(&mut self, message: MeshMessage);
    
    // Seeding
    pub async fn seed_content(&mut self, content_id: &str) -> Result<(), MeshError>;
    pub async fn request_content(&mut self, content_id: &str, peer_id: Option<PeerId>) -> Result<Vec<u8>, MeshError>;
}

struct Peer {
    id: PeerId,
    addrs: Vec<Multiaddr>,
    wireguard_endpoint: SocketAddr,
    public_key: PublicKey,
    last_seen: Instant,
    latency_ms: u32,
}

enum MeshMessage {
    Delta { namespace: String, crdt_delta: Vec<u8> },
    RequestSync { namespace: String, merkle_root: Hash },
    SyncResponse { namespace: String, deltas: Vec<CrdtDelta> },
    ContentRequest { content_id: String },
    ContentResponse { content_id: String, data: Vec<u8> },
}
```

**Discovery:**
- **mDNS:** `_coreos._tcp.local` — broadcast в локальной сети
- **Bootstrap nodes:** список известных узлов (опционально, для интернет-сценариев)
- **DHT (libp2p):** поиск узлов по PeerId в глобальной сети

**Transport:**
- **WireGuard:** все P2P соединения через WireGuard tunnel (шифрование, NAT traversal)
- **libp2p:** поверх WireGuard для multiplexing streams
- **QUIC:** транспортный протокол (low latency, 0-RTT)

### 5.2 CRDT Engine

```rust
// sync/src/crdt.rs
pub struct CrdtEngine {
    stores: HashMap<String, CrdtStore>, // namespace -> store
    clock: HybridLogicalClock,
}

impl CrdtEngine {
    pub fn new() -> Self;
    
    // Operations
    pub fn insert(&mut self, namespace: &str, key: &str, value: Value) -> Delta;
    pub fn update(&mut self, namespace: &str, key: &str, value: Value) -> Delta;
    pub fn delete(&mut self, namespace: &str, key: &str) -> Delta;
    
    // Sync
    pub fn apply_delta(&mut self, namespace: &str, delta: Delta) -> Result<(), CrdtError>;
    pub fn get_delta_since(&self, namespace: &str, hlc: HLC) -> Vec<Delta>;
    pub fn merkle_root(&self, namespace: &str) -> Hash;
    
    // Conflict resolution
    pub fn resolve_conflict(&self, a: &Delta, b: &Delta) -> Delta {
        // 1. LWW (Last Write Wins) по HLC
        // 2. Если HLC конфликтуют (concurrent) — Hash-based Ordering
        if a.hlc == b.hlc {
            // Hash-based ordering
            let hash_a = blake3::hash(&a.to_bytes());
            let hash_b = blake3::hash(&b.to_bytes());
            if hash_a.as_bytes() < hash_b.as_bytes() { a.clone() } else { b.clone() }
        } else if a.hlc > b.hlc {
            a.clone()
        } else {
            b.clone()
        }
    }
}

struct Delta {
    namespace: String,
    key: String,
    value: Option<Value>, // None = delete
    hlc: HLC,
    peer_id: PeerId,
}

struct HLC {
    physical: u64, // millis
    logical: u32,
    node_id: PeerId,
}
```

**Типы CRDT:**
| Namespace | Тип CRDT | Применение |
|-----------|----------|------------|
| `projects` | LWW-Register + Map | Проекты, метаданные |
| `layouts` | LWW-Register | Layout окон |
| `notes` | LWW-Register + Map | Заметки |
| `tags` | OR-Set | Теги |
| `files` | LWW-Register (metadata) + blob (content) | Файлы |
| `contacts` | LWW-Register + Map | Контакты |
| `settings` | LWW-Register | Настройки |

**Anti-entropy:**
```rust
pub async fn anti_entropy(&mut self, peer_id: &PeerId) -> Result<(), SyncError> {
    for namespace in self.stores.keys() {
        let local_root = self.merkle_root(namespace);
        let remote_root = self.request_merkle_root(peer_id, namespace).await?;
        
        if local_root != remote_root {
            // XOR-sync: находим расхождения через Merkle Search Tree
            let deltas = self.xor_sync(peer_id, namespace).await?;
            for delta in deltas {
                self.apply_delta(namespace, delta)?;
            }
        }
    }
    Ok(())
}
```

### 5.3 Sync Engine

```rust
// sync/src/engine.rs
pub struct SyncEngine {
    crdt: CrdtEngine,
    mesh: P2PMesh,
    sqlite: Database, // CRDT journal
}

impl SyncEngine {
    pub async fn start(&mut self) {
        // 1. Загрузка CRDT journal из SQLite
        self.load_journal().await;
        
        // 2. Запуск mDNS advertisement
        self.mesh.advertise().await.unwrap();
        
        // 3. Периодическая синхронизация
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(30)).await;
                self.sync_all_peers().await;
            }
        });
    }
    
    async fn sync_all_peers(&mut self) {
        let peers = self.mesh.discover_peers().await;
        for peer in peers {
            if let Err(e) = self.mesh.sync_with(&peer.id, "*").await {
                log::warn!("Sync failed with {}: {}", peer.id, e);
            }
        }
    }
    
    // Lazy Boot
    pub async fn lazy_boot(&mut self) -> Result<(), SyncError> {
        // Загрузка только метаданных, без контента
        for namespace in ["projects", "contacts", "settings"] {
            self.crdt.load_metadata(namespace).await?;
        }
        // Контент загружается по требованию (on-demand)
        Ok(())
    }
}
```

**SQLite schema (CRDT journal):**
```sql
CREATE TABLE crdt_journal (
    id INTEGER PRIMARY KEY,
    namespace TEXT NOT NULL,
    key TEXT NOT NULL,
    value TEXT, -- JSON or null (delete)
    hlc_physical INTEGER NOT NULL,
    hlc_logical INTEGER NOT NULL,
    peer_id TEXT NOT NULL,
    UNIQUE(namespace, key, hlc_physical, hlc_logical, peer_id)
);

CREATE INDEX idx_journal_ns_key ON crdt_journal(namespace, key);
CREATE INDEX idx_journal_hlc ON crdt_journal(hlc_physical, hlc_logical);

-- Merkle Search Tree nodes
CREATE TABLE merkle_tree (
    namespace TEXT NOT NULL,
    level INTEGER NOT NULL,
    prefix TEXT NOT NULL,
    hash TEXT NOT NULL,
    PRIMARY KEY (namespace, level, prefix)
);
```

### 5.4 Backup Engine

```typescript
// micro-kernel/src/backup/engine.ts
export class BackupEngine {
    private targets: BackupTarget[];
    
    async backup(): Promise<BackupResult> {
        const data = await this.collectData();
        const encrypted = await this.encrypt(data);
        
        for (const target of this.targets) {
            await target.push(encrypted);
        }
        
        return { timestamp: Date.now(), size: encrypted.length };
    }
    
    async restore(target: BackupTarget, date?: Date): Promise<void> {
        const backups = await target.list();
        const selected = date 
            ? backups.find(b => b.date <= date)
            : backups[backups.length - 1];
        
        const encrypted = await target.pull(selected.id);
        const data = await this.decrypt(encrypted);
        await this.applyData(data);
    }
    
    private async collectData(): Promise<BackupData> {
        return {
            sqlite: await this.exportSQLite(),
            vfs: await this.exportVFS(),
            crdt: await this.exportCRDT(),
            config: await this.exportConfig(),
        };
    }
}

interface BackupTarget {
    push(data: Uint8Array): Promise<void>;
    pull(id: string): Promise<Uint8Array>;
    list(): Promise<BackupEntry[]>;
    delete(id: string): Promise<void>;
}

class UsbTarget implements BackupTarget { /* ... */ }
class S3Target implements BackupTarget { /* ... */ }
class SftpTarget implements BackupTarget { /* ... */ }
class CoreTarget implements BackupTarget { /* другой Бэк CORE OS */ }
```

**Шифрование:**
- Backup key derived из recovery-фразы (BIP-39 → BLAKE3 KDF)
- Алгоритм: XChaCha20-Poly1305
- Каждый бэкап = tarball (zst сжатие) + nonce + tag

### 5.5 Lazy Load / On-Demand

```rust
// sync/src/lazy_load.rs
pub struct LazyLoadEngine {
    vfs: VFS,
    mesh: P2PMesh,
    cache: LruCache<ContentId, Vec<u8>>,
}

impl LazyLoadEngine {
    pub async fn get_file(&mut self, file_id: &ContentId) -> Result<Vec<u8>, LoadError> {
        // 1. Проверка локального кэша
        if let Some(data) = self.cache.get(file_id) {
            return Ok(data.clone());
        }
        
        // 2. Проверка локального VFS
        if let Some(data) = self.vfs.read(file_id).await? {
            self.cache.put(file_id.clone(), data.clone());
            return Ok(data);
        }
        
        // 3. Запрос у peers
        let peers = self.mesh.discover_peers().await;
        for peer in peers {
            if let Ok(data) = self.mesh.request_content(file_id, Some(peer.id)).await {
                self.vfs.write(file_id, &data).await?;
                self.cache.put(file_id.clone(), data.clone());
                return Ok(data);
            }
        }
        
        Err(LoadError::NotFound)
    }
    
    pub async fn stream_media(&mut self, file_id: &ContentId) -> Result<Stream, LoadError> {
        // Потоковая передача: chunked download + playback
        let mut stream = self.mesh.request_stream(file_id).await?;
        Ok(stream)
    }
}
```

**Ghost-файлы:**
- Файл недоступен локально → отображается как "ghost" (серый значок, 0 байт)
- При открытии → автоматический `get_file()` → появляется прогресс-бар
- Умный кэш: предзагрузка следующих файлов в плейлисте

### 5.6 Verified Content Seeding (Public P2P CDN)

```rust
// sync/src/seeding.rs
pub struct ContentSeeding {
    mesh: P2PMesh,
    verified_content: HashMap<ContentId, VerifiedContent>,
    bandwidth_cap: f32, // 0.1 = 10%
}

impl ContentSeeding {
    pub async fn seed_app(&mut self, app_id: &str) -> Result<(), SeedError> {
        let manifest = self.load_manifest(app_id)?;
        
        // Проверка подписи
        if !self.verify_signature(&manifest) {
            return Err(SeedError::InvalidSignature);
        }
        
        // Регистрация в DHT
        self.mesh.seed_content(&manifest.content_id).await?;
        
        Ok(())
    }
    
    pub async fn serve_requests(&mut self) {
        while let Some(request) = self.mesh.next_content_request().await {
            if self.verified_content.contains_key(&request.content_id) {
                let data = self.verified_content[&request.content_id].data.clone();
                self.mesh.send_content_response(request.peer_id, data).await;
            }
        }
    }
}
```

---

## Шаги реализации

### Шаг 5.1: WireGuard + libp2p (5 дней)

1. WireGuard tunnel между двумя устройствами
2. libp2p поверх WireGuard (QUIC transport)
3. PeerId (Ed25519), ключевая пара
4. Тест: ping между двумя устройствами через tunnel

### Шаг 5.2: mDNS Discovery (2 дня)

1. mDNS advertisement (`_coreos._tcp.local`)
2. mDNS browsing (обнаружение peers)
3. Тест: два устройства в одной Wi-Fi находят друг друга

### Шаг 5.3: CRDT Engine (5 дней)

1. HLC (Hybrid Logical Clock) implementation
2. LWW-Register, OR-Set, LWW-Map
3. Delta encoding (minimal diff)
4. SQLite journal (append-only log)
5. Тест: два устройства редактируют одну заметку → конвергенция

### Шаг 5.4: Merkle Search Tree + XOR Sync (4 дня)

1. Merkle tree над CRDT journal
2. XOR-sync algorithm (нахождение расхождений)
3. Zstd compression для дельт
4. Тест: 1000 изменений → sync за < 1 сек

### Шаг 5.5: Sync Engine (3 дня)

1. Автоматическая синхронизация (каждые 30 сек)
2. Lazy Boot (загрузка метаданных)
3. Conflict resolution (LWW + Hash-based Ordering)
4. Тест: offline → online → merge без потерь

### Шаг 5.6: Backup Engine (3 дня)

1. Tar + Zstd + XChaCha20
2. USB target (локальное копирование)
3. S3 target (AWS SDK)
4. Версионность (retention policy)
5. Тест: бэкап → удаление → восстановление

### Шаг 5.7: Lazy Load (3 дня)

1. Ghost-файлы (метаданные без контента)
2. On-demand loading с прогресс-баром
3. LRU cache (100 MB по умолчанию)
4. Media streaming (chunked)
5. Тест: открытие файла с другого устройства

### Шаг 5.8: Seeding (2 дня)

1. Verified content registry
2. DHT seeding
3. Bandwidth cap (10%)
4. Тест: скачивание приложения через P2P

---

## Критерии приёмки

- [ ] Два устройства находят друг друга через mDNS < 5 сек
- [ ] WireGuard tunnel устанавливается < 1 сек
- [ ] CRDT: конвергенция при одновременном редактировании (100 тестов)
- [ ] Offline → online: данные синхронизируются без конфликтов
- [ ] Sync: 1000 дельт → передача < 1 сек (локальная сеть)
- [ ] Backup: полный бэкап → шифрование → USB → восстановление
- [ ] Lazy Load: ghost-файл → открытие → загрузка < 3 сек (Wi-Fi)
- [ ] Seeding: приложение раздаётся, другой узел скачивает
- [ ] Bandwidth cap: раздача не превышает 10% канала

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Только локальная сеть (mDNS) | Post-release | Internet bootstrap nodes, relay |
| XOR-sync (упрощённый) | Post-release | Полный Merkle Search Tree |
| Backup: только USB + S3 | Post-release | SFTP, CustomTarget (plugins) |
| Seeding: только приложения | Post-release | Media, документы |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| P2P Mesh | layer-8 §9.1, layer-5 (устройства) |
| CRDT Engine | layer-8 §9.2, layer-3 (CRDT-слой) |
| Sync Engine | layer-8 §9.3–9.6, layer-1 (устройства как один экран) |
| Backup Engine | layer-8 §9.7, layer-1 (бэкап) |
| Lazy Load | layer-8 §9.4, layer-1 (ленивая загрузка) |
| Seeding | layer-8 §9.1.1, layer-1 (раздача приложений) |

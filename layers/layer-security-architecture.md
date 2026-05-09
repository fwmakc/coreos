# Security Architecture

Конкретные реализации безопасности: seccomp-профили, namespace-конфиги, memory protection, boot integrity. Для команды реализации Host Shim и Micro-Kernel.

**Статус:** спецификация для MVP (реализация — Host Shim на Rust + Micro-Kernel на Bun).  
**Ссылки:** [layer-security.md](layer-security.md), [layer-threat-model.md](layer-threat-model.md), [architecture.md](../project/architecture.md).

---

## 1. Sandbox по уровням приложений

### 1.1 Общая архитектура

```
Уровень 1–2: Island Mode (Chromium sandbox)
  └─ Linux: namespace + seccomp + chroot
  └─ Windows: AppContainer + Job Object
  └─ macOS: seatbelt profile

Уровень 3: Island Mode + V8 Isolate backend
  └─ Linux: namespace + seccomp-bpf + network namespace (whitelist)
  └─ Windows: AppContainer + restricted network
  └─ macOS: seatbelt + network filter

Уровень 4–5: V8 Isolate (Bun runtime)
  └─ Linux: namespace + seccomp-bpf + V8 Sandbox
  └─ Windows: AppContainer + Job Object + V8 Sandbox
  └─ macOS: seatbelt + V8 Sandbox
```

### 1.2 Seccomp-профили (Linux)

**Base profile** — для всех V8 Isolates:

```json
{
  "defaultAction": "SCMP_ACT_ERRNO",
  "architectures": ["SCMP_ARCH_X86_64", "SCMP_ARCH_AARCH64"],
  "syscalls": [
    {"names": ["read", "write", "close", "exit", "exit_group"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["mmap", "munmap", "mprotect", "brk"], "action": "SCMP_ACT_ALLOW", "args": [
      {"index": 2, "op": "SCMP_CMP_MASKED_EQ", "value": 0x7, "mask": 0x7}
    ]},
    {"names": ["futex", "sched_yield", "clock_gettime", "gettimeofday"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["rt_sigaction", "rt_sigprocmask", "sigreturn"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["prctl"], "action": "SCMP_ACT_ALLOW", "args": [
      {"index": 0, "op": "SCMP_CMP_EQ", "value": 15}
    ]},
    {"names": ["openat"], "action": "SCMP_ACT_ALLOW", "args": [
      {"index": 2, "op": "SCMP_CMP_MASKED_EQ", "value": 0x3, "mask": 0x3}
    ]},
    {"names": ["socket", "connect", "sendto", "recvfrom", "getsockopt"], "action": "SCMP_ACT_ERRNO"},
    {"names": ["execve", "execveat", "fork", "vfork", "clone"], "action": "SCMP_ACT_KILL"},
    {"names": ["ptrace"], "action": "SCMP_ACT_KILL"},
    {"names": ["mount", "umount2", "pivot_root"], "action": "SCMP_ACT_KILL"},
    {"names": ["chmod", "chown", "setuid", "setgid", "setresuid"], "action": "SCMP_ACT_KILL"}
  ]
}
```

**Level 3 network profile** — дополнительно к base:

```json
{
  "extends": "base",
  "syscalls": [
    {"names": ["socket"], "action": "SCMP_ACT_ALLOW", "args": [
      {"index": 0, "op": "SCMP_CMP_EQ", "value": 2},
      {"index": 1, "op": "SCMP_CMP_EQ", "value": 1}
    ]},
    {"names": ["connect", "sendto", "recvfrom", "getsockopt", "setsockopt"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["bind", "listen", "accept"], "action": "SCMP_ACT_ERRNO"}
  ]
}
```

**Level 4–5 fs profile** — если приложение запросило `fs` capability:

```json
{
  "extends": "base",
  "syscalls": [
    {"names": ["openat", "read", "write", "close", "lseek", "fstat", "getdents64"], "action": "SCMP_ACT_ALLOW"},
    {"names": ["mkdir", "unlink", "rename"], "action": "SCMP_ACT_ALLOW", "args": [
      {"index": 0, "op": "SCMP_CMP_MASKED_EQ", "value": "/vfs/app/**", "mask": null}
    ]}
  ]
}
```

> **Примечание:** seccomp-bpf не фильтрует по пути — путь фильтруется на уровне Micro-Kernel (Capability Security). seccomp блокирует опасные syscall категориями.

### 1.3 Windows AppContainer

**SID capabilities для уровней 3–5:**

```cpp
// Base capability
S-1-15-3-1  // internetClient (только для уровня 3, если whitelist пуст)

// Level 3 с whitelist → убираем internetClient, добавляем custom capability
S-1-15-3-1024-xxx  // custom: whitelist доменов через proxy (Micro-Kernel)

// Level 4–5 без network capability → полностью изолирован
// FS access через broker (Micro-Kernel)
```

**Job Object ограничения:**
- `JOB_OBJECT_LIMIT_ACTIVE_PROCESS`: 1 (только сам Isolate)
- `JOB_OBJECT_LIMIT_AFFINITY`: 2 ядра максимум
- `JOB_OBJECT_LIMIT_WORKINGSET`: 512MB default, настраивается квотой
- `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`: автоматическое уничтожение при закрытии

### 1.4 macOS Seatbelt

**Base profile (V8 Isolate):**

```scheme
(version 1)
(deny default)
(allow signal)
(allow ipc-posix-shm)
(allow sysctl-read)
(allow mach-lookup)

; Чтение исполняемого кода и shared libraries
(allow file-read*
  (subpath "/System/Library")
  (subpath "/usr/lib")
  (subpath "{{APP_BUNDLE}}/Contents/Frameworks"))

; Запрет сети (для уровней 4-5)
(deny network*)

; Для уровня 3 — разрешение через Micro-Kernel proxy
(allow network-outbound
  (remote ip "{{WHITELIST_IPS}}"))
```

---

## 2. Namespace конфигурации (Linux)

### 2.1 Полная изоляция уровня 3–5

```bash
unshare --pid --net --mount --ipc --uts --user --fork \
  --mount-proc=/proc \
  --pid-file=/run/core/isolate_{{APP_ID}}.pid \
  /opt/core/runtime/isolate_launcher {{APP_ID}}
```

**Что изолируется:**

| Namespace | Что изолировано | Зачем |
|-----------|----------------|-------|
| PID | Собственное дерево процессов | Isolate не видит другие приложения |
| Network | Отдельный loopback + veth к Micro-Kernel | Контроль трафика на уровне ядра |
| Mount | Только /vfs/app/{{APP_ID}} + ro /system | Нет доступа к хост-ФС |
| IPC | Собственные SysV IPC + POSIX shm | Нет перехвата сообщений других приложений |
| UTS | Собственное hostname | Минимизация информации о хосте |
| User | UID 0 внутри = непривилегированный снаружи | Побег из namespace ≠ root |

### 2.2 Mount layout внутри Isolate

```
/
├── proc          # ro, mount-proc (только свои процессы)
├── sys           # ro, пустой или минимальный
├── dev           # минимальный (null, zero, random, urandom, tty)
├── vfs/
│   └── app/
│       └── {{APP_ID}}/  # rw, app-scoped VFS
├── system/
│   └── lib/       # ro, shared libraries runtime
└── tmp/           # tmpfs, ограниченный размер (100MB)
```

---

## 3. V8 Sandbox (memory corruption mitigation)

### 3.1 Параметры запуска

```bash
--v8-sandbox
--v8-sandbox-pointer-compression
--v8-sandbox-page-size=262144  # 256KB sandbox pages
--v8-sandbox-gc
```

### 3.2 Резервирование address space

```rust
// Host Shim (Rust)
let sandbox_size = 1 << 40; // 1TB address space reservation
let sandbox_base = unsafe {
    libc::mmap(
        null_mut(),
        sandbox_size,
        PROT_NONE,
        MAP_PRIVATE | MAP_ANONYMOUS | MAP_NORESERVE,
        -1,
        0,
    )
};
// Передаём sandbox_base в V8 Isolate при создании
```

### 3.3 Runtime integrity check

```rust
// Периодическая проверка (каждые 30 сек)
fn verify_isolate_memory(isolate: &Isolate) -> Result<(), SecurityError> {
    let heap = isolate.get_heap_statistics();
    // Проверка: все указатели внутри sandbox
    // Проверка: нет unexpected executable pages
    // Проверка: stack canaries intact
    isolate.verify_pointer_cage()
}
```

---

## 4. Memory Protection

### 4.1 Анонимный профиль (mlock)

```rust
#[cfg(target_os = "linux")]
fn secure_anonymous_profile(data: &mut [u8]) {
    unsafe {
        libc::mlock(data.as_ptr() as *const c_void, data.len());
    }
}

#[cfg(target_os = "windows")]
fn secure_anonymous_profile(data: &mut [u8]) {
    unsafe {
        winapi::um::memoryapi::VirtualLock(
            data.as_ptr() as LPVOID,
            data.len() as SIZE_T,
        );
    }
}
```

### 4.2 Secure wipe при freeze профиля

```rust
fn secure_wipe_profile(profile_id: &str) {
    // 1. Очистка V8 heap
    for isolate in get_isolates_by_profile(profile_id) {
        isolate.terminate_execution();
        isolate.dispose();
    }
    
    // 2. Очистка SharedArrayBuffer regions
    for region in get_sab_regions(profile_id) {
        region.zeroize(); // zeroize crate
        drop(region);
    }
    
    // 3. Очистка clipboard
    clipboard::clear_by_profile(profile_id);
    
    // 4. Очистка GPU buffers (WebGPU)
    gpu::clear_buffers_by_profile(profile_id);
}
```

### 4.3 Encrypted memory (уровень «Максимальный»)

```rust
#[cfg(feature = "max_security")]
mod encrypted_memory {
    use aes_gcm_siv::{Aes256GcmSiv, Nonce};
    
    pub struct EncryptedPage {
        ciphertext: Vec<u8>,
        nonce: Nonce,
        tag: [u8; 16],
    }
    
    // Данные расшифровываются только при доступе, шифруются после use
    // Реализация через page fault handler (mprotect + SIGSEGV)
}
```

---

## 5. Boot Integrity

### 5.1 Core Base / Raspberry Pi

```bash
# LUKS encrypted rootfs
cryptsetup luksFormat /dev/mmcblk0p2
cryptsetup open /dev/mmcblk0p2 core-root
mkfs.ext4 /dev/mapper/core-root

# Boot partition — только signed kernel + initrd
# initrd проверяет подпись rootfs перед mount
```

### 5.2 Boot-time passphrase via QR (Core Base)

```
Core Base загружается → на HDMI показывает QR-код
└─ QR содержит: ephemeral public key + device nonce

Приложение CORE на телефоне:
└─ Сканирует QR → генерирует shared secret (ECDH)
└─ Шифрует passphrase shared secret
└─ Отправляет encrypted passphrase в Core Base (BLE / LAN)

Core Base:
└─ Расшифровывает passphrase
└─ Открывает LUKS
└─ Уничтожает passphrase из RAM (explicit zeroize)
```

### 5.3 Integrity Monitoring Agent

```rust
// Периодическая проверка (каждые 5 мин)
fn check_integrity() -> IntegrityReport {
    let expected_hash = get_expected_binary_hash(); // от Бэка или embedded
    let actual_hash = blake3::hash(std::fs::read("/opt/core/bin/core-os").unwrap());
    
    if expected_hash != actual_hash {
        IntegrityReport::Tampered {
            expected: expected_hash,
            actual: actual_hash,
            timestamp: now(),
        }
    } else {
        IntegrityReport::Ok
    }
}
```

---

## 6. Network Isolation (уровень 3)

### 6.1 Micro-Kernel network proxy

```
[V8 Isolate Level 3]
  │ socket() → connect()
  │ 
  ▼
[Network Proxy (Micro-Kernel, Level 1)]
  │ Проверка: домен в whitelist манифеста?
  │ Да → проксирование через host-ОС
  │ Нет → ECONNREFUSED
  ▼
[Host-ОС network stack]
```

### 6.2 Firewall-правило localhost

```bash
# Только frontend Island этого приложения может стучаться на localhost:PORT
iptables -A INPUT -p tcp --dport {{APP_PORT}} \
  -m owner --uid-owner {{FRONTEND_UID}} \
  -j ACCEPT

iptables -A INPUT -p tcp --dport {{APP_PORT}} \
  -j DROP
```

> На Windows: Windows Filtering Platform (WFP)  
> На macOS: pf / application firewall

---

## 7. App Lifecycle Security Hooks

### 7.1 Hook points

```rust
enum SecurityHook {
    BeforeIsolateCreate { app_id, level, manifest },
    AfterIsolateCreate { isolate_id, pid },
    BeforeApiCall { isolate_id, api, args },
    AfterApiCall { isolate_id, api, result },
    BeforeNetworkRequest { isolate_id, domain, port },
    OnIsolateFreeze { isolate_id },
    OnIsolateThaw { isolate_id },
    OnIsolateDestroy { isolate_id },
    OnProfileSwitch { from_profile, to_profile },
}
```

### 7.2 Реализация hooks (Rust)

```rust
trait SecurityHookHandler {
    fn handle(&self, hook: SecurityHook) -> HookResult;
}

struct DefaultSecurityHandler;

impl SecurityHookHandler for DefaultSecurityHandler {
    fn handle(&self, hook: SecurityHook) -> HookResult {
        match hook {
            SecurityHook::BeforeNetworkRequest { isolate_id, domain, .. } => {
                if !is_domain_whitelisted(isolate_id, domain) {
                    return HookResult::Block(NetworkError::DomainNotAllowed);
                }
                HookResult::Allow
            }
            SecurityHook::OnProfileSwitch { from_profile, .. } => {
                secure_wipe_profile(&from_profile);
                HookResult::Allow
            }
            _ => HookResult::Allow,
        }
    }
}
```

---

## 8. Session Management

### 8.1 TTL и auto-lock

```rust
struct SessionConfig {
    ttl_minutes: u32,           // 30 по умолчанию
    auto_lock_after_idle: u32,  // 5 минут простоя
    require_biometry: bool,     // true для уровня «Повышенный»
}

// Проверка каждую минуту
fn check_session_expiry(session: &Session) -> SessionAction {
    if session.idle_duration() > session.config.auto_lock_after_idle {
        SessionAction::Lock
    } else if session.total_duration() > session.config.ttl_minutes {
        SessionAction::Logout
    } else {
        SessionAction::Continue
    }
}
```

### 8.2 Принудительный logout Owner-ом

```rust
// Owner через Core.Hardcore или другой Фронт
fn force_logout(device_key: &DeviceKey) {
    // 1. Отзыв session token на Бэке
    backend.revoke_session(device_key);
    
    // 2. Уведомление Фронту (push)
    push_notification(device_key, Notification::SessionRevoked);
    
    // 3. Фронт: lock screen + очистка кэша
    frontend.lock_and_purge(device_key);
}
```

---

## 9. Supply Chain Protection

### 9.1 Pinning зависимостей

```toml
# Cargo.lock — коммитится в репозиторий
# Для каждого crate:
[package]
name = "wgpu"
version = "0.19.0"
source = "registry+https://github.com/rust-lang/crates.io-index"
checksum = "blake3::hash(...)"  # Верификация при сборке
```

```json
// bun.lockb — коммитится
// package.json — exact versions, no ^ or ~
{
  "dependencies": {
    "sqlite3": "5.1.6",
    "zod": "3.22.4"
  }
}
```

### 9.2 Reproducible builds

```bash
# Docker-контейнер для сборки с фиксированным toolchain
docker run --rm -v $(pwd):/src core/builder:v1.0.0 \
  cargo build --release --locked

# Проверка reproducibility
sha256sum target/release/core-os > build.hash
# Два разных разработчика → одинаковый build.hash
```

---

## 10. Интеграция с CI/CD

### 10.1 Security gates

```yaml
# .github/workflows/security.yml
jobs:
  seccomp_validate:
    runs-on: ubuntu-latest
    steps:
      - run: seccomp-tools dump ./target/release/core-os | diff - expected_seccomp.out
      
  namespace_test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --test namespace_isolation
      
  v8_sandbox_test:
    runs-on: ubuntu-latest
    steps:
      - run: cargo test --test v8_sandbox_escape
      
  threat_model_check:
    runs-on: ubuntu-latest
    steps:
      - run: python scripts/check_threat_model.py --diff HEAD~1
```

---

## 11. Роадмап реализации

| Фаза | Что | Срок |
|------|-----|------|
| MVP-1 | Base seccomp + PID namespace + V8 Sandbox | Месяц 1–2 |
| MVP-2 | Network namespace + Micro-Kernel proxy | Месяц 2–3 |
| MVP-3 | Mount namespace + fs isolation | Месяц 3 |
| Post-MVP | Windows AppContainer + macOS seatbelt | Месяц 4–6 |
| Post-MVP | Encrypted memory + mlock max level | Месяц 4–6 |
| Post-MVP | Boot integrity + remote attestation | Месяц 5–7 |

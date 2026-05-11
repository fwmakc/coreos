# Этап 10 — Performance & Scale (Оптимизация и масштабирование)

> **Цель:** Система работает стабильно под нагрузкой. 60 FPS при 1000 окон, P2P sync < 1 сек для 1000 документов, boot < 2 сек, memory < 2 GB для base конфигурации.

**После этого этапа:** CORE OS оптимизирована для production. Все bottleneck'ы устранены, метрики собираются, система масштабируется от Raspberry Pi до сервера.

---

## Зависимости

- Все предыдущие этапы (1–9)

---

## Компоненты

### 10.1 Performance Budget

```rust
// perf/src/budget.rs
pub struct PerformanceBudget {
    targets: HashMap<String, BudgetTarget>,
    metrics: MetricsCollector,
}

impl PerformanceBudget {
    pub fn new() -> Self {
        let mut targets = HashMap::new();
        
        // Rendering
        targets.insert("frame_time".to_string(), BudgetTarget { max: 16.67, unit: "ms" }); // 60 FPS
        targets.insert("input_latency".to_string(), BudgetTarget { max: 8.0, unit: "ms" });
        targets.insert("window_switch".to_string(), BudgetTarget { max: 100.0, unit: "ms" });
        
        // Boot
        targets.insert("cold_boot".to_string(), BudgetTarget { max: 2000.0, unit: "ms" });
        targets.insert("warm_boot".to_string(), BudgetTarget { max: 500.0, unit: "ms" });
        
        // Memory
        targets.insert("base_memory".to_string(), BudgetTarget { max: 2048.0, unit: "MB" });
        targets.insert("per_app_memory".to_string(), BudgetTarget { max: 128.0, unit: "MB" });
        
        // P2P
        targets.insert("sync_latency".to_string(), BudgetTarget { max: 1000.0, unit: "ms" });
        targets.insert("announce_interval".to_string(), BudgetTarget { max: 60.0, unit: "s" });
        
        // Voice
        targets.insert("asr_latency".to_string(), BudgetTarget { max: 500.0, unit: "ms" });
        targets.insert("tts_latency".to_string(), BudgetTarget { max: 100.0, unit: "ms" });
        
        Self { targets, metrics: MetricsCollector::new() }
    }
    
    pub fn check(&self, metric: &str, value: f64) -> BudgetStatus {
        let target = self.targets.get(metric).unwrap();
        let ratio = value / target.max;
        
        if ratio < 0.5 {
            BudgetStatus::Green
        } else if ratio < 0.8 {
            BudgetStatus::Yellow
        } else if ratio < 1.0 {
            BudgetStatus::Orange
        } else {
            BudgetStatus::Red
        }
    }
    
    pub fn report(&self) -> PerformanceReport {
        self.metrics.generate_report(&self.targets)
    }
}
```

### 10.2 Profiling Infrastructure

```rust
// perf/src/profiler.rs
pub struct Profiler {
    spans: Vec<Span>,
    active_spans: HashMap<u64, Instant>,
}

impl Profiler {
    pub fn start_span(&mut self, name: &str) -> SpanId {
        let id = self.next_id();
        self.active_spans.insert(id, Instant::now());
        SpanId(id)
    }
    
    pub fn end_span(&mut self, id: SpanId) {
        if let Some(start) = self.active_spans.remove(&id.0) {
            let duration = start.elapsed();
            self.spans.push(Span {
                id: id.0,
                duration,
                // ...
            });
        }
    }
    
    pub fn trace<F, R>(&mut self, name: &str, f: F) -> R
    where F: FnOnce() -> R {
        let id = self.start_span(name);
        let result = f();
        self.end_span(id);
        result
    }
}

// Использование:
profiler.trace("render_frame", || {
    display_server.render_frame();
});
```

**Chrome DevTools-compatible trace:**
```json
{
    "traceEvents": [
        {"name": "render_frame", "ph": "B", "ts": 1000000, "pid": 1, "tid": 1},
        {"name": "render_frame", "ph": "E", "ts": 1000016, "pid": 1, "tid": 1},
        {"name": "process_input", "ph": "B", "ts": 1000020, "pid": 1, "tid": 2},
        {"name": "process_input", "ph": "E", "ts": 1000024, "pid": 1, "tid": 2}
    ]
}
```

### 10.3 Rendering Optimizations

```rust
// display/src/optimizations.rs
pub struct RenderOptimizer {
    occlusion_culling: OcclusionCulling,
    damage_tracking: DamageTracking,
    texture_atlas: TextureAtlas,
    glyph_cache: GlyphCache,
}

impl RenderOptimizer {
    pub fn optimize(&mut self, scene: &mut Scene) {
        // 1. Occlusion culling — не рендерить скрытые окна
        self.occlusion_culling.cull(scene);
        
        // 2. Damage tracking — перерисовывать только изменённые области
        let damage = self.damage_tracking.compute(scene);
        scene.set_damage_region(damage);
        
        // 3. Texture atlas — объединить маленькие текстуры
        self.texture_atlas.pack(scene);
        
        // 4. Glyph cache — кэшировать глифы
        self.glyph_cache.prerender(scene.text_elements());
    }
    
    pub fn batch_draw_calls(&mut self, scene: &Scene) -> Vec<DrawCall> {
        // Merge consecutive draw calls with same shader/material
        let mut batches = Vec::new();
        let mut current_batch: Option<DrawCall> = None;
        
        for draw in scene.draw_calls() {
            if let Some(ref mut batch) = current_batch {
                if batch.can_merge(draw) {
                    batch.merge(draw);
                    continue;
                }
                batches.push(batch.clone());
            }
            current_batch = Some(draw.clone());
        }
        
        if let Some(batch) = current_batch {
            batches.push(batch);
        }
        
        batches
    }
}
```

**Instanced rendering для UI:**
```rust
// Рендер 1000 кнопок одним draw call
let instances: Vec<ButtonInstance> = buttons.iter().map(|b| {
    ButtonInstance {
        position: b.position,
        size: b.size,
        color: b.color,
        border_radius: b.border_radius,
    }
}).collect();

render_pass.draw_instanced(
    &button_mesh,       // 1 mesh
    &instances_buffer,  // 1000 instances
    &button_shader,
);
```

### 10.4 Memory Optimizations

```rust
// perf/src/memory.rs
pub struct MemoryOptimizer {
    allocator: ArenaAllocator,
    object_pool: ObjectPool,
    compression: CompressionEngine,
}

impl MemoryOptimizer {
    pub fn optimize_vfs(&mut self, vfs: &mut VFS) {
        // 1. LRU cache для часто используемых файлов
        vfs.set_cache_policy(CachePolicy::LRU { max_size: 256 * 1024 * 1024 }); // 256 MB
        
        // 2. Compression для неактивных файлов
        vfs.set_compression(Compression::Zstd { level: 3 });
        
        // 3. Memory mapping для больших файлов
        vfs.use_mmap_for_files_larger_than(10 * 1024 * 1024); // 10 MB
    }
    
    pub fn optimize_rendering(&mut self, display: &mut DisplayServer) {
        // 1. Texture compression (BC7/ASTC)
        display.set_texture_format(TextureFormat::BC7);
        
        // 2. Mipmaps для уменьшения bandwidth
        display.generate_mipmaps();
        
        // 3. GPU memory budget
        display.set_gpu_memory_budget(1024 * 1024 * 1024); // 1 GB
    }
    
    pub fn gc(&mut self) {
        // 1. Очистка неиспользуемых текстур
        self.texture_cache.sweep();
        
        // 2. Очистка glyph cache
        self.glyph_cache.sweep();
        
        // 3. Compact arenas
        self.allocator.compact();
    }
}
```

### 10.5 P2P Optimizations

```rust
// p2p/src/optimizations.rs
pub struct P2POptimizer {
    sync: SyncOptimizer,
    routing: RoutingOptimizer,
    compression: DeltaCompression,
}

impl P2POptimizer {
    pub fn optimize_sync(&mut self, crdt: &mut CRDT) {
        // 1. Bloom filter для пропуска уже синхронизированных
        crdt.enable_bloom_filter();
        
        // 2. Delta encoding для текстовых документов
        self.compression.enable_for_type("text/plain");
        
        // 3. Batch sync (накопление изменений 100 мс)
        crdt.set_batch_interval(100);
        
        // 4. Priority sync (активные документы первыми)
        crdt.set_priority_policy(PriorityPolicy::ActiveFirst);
    }
    
    pub fn optimize_routing(&mut self, mesh: &mut P2PMesh) {
        // 1. Kademlia DHT для быстрого поиска
        mesh.enable_kademlia();
        
        // 2. Connection pooling
        mesh.set_max_connections(50);
        
        // 3. Circuit breaking для нестабильных узлов
        mesh.enable_circuit_breaker(CircuitBreakerConfig {
            failure_threshold: 5,
            recovery_timeout: 30,
        });
    }
}
```

### 10.6 Stress Tests

```rust
// tests/stress/src/lib.rs
pub struct StressTestSuite {
    scenarios: Vec<Box<dyn StressScenario>>,
}

impl StressTestSuite {
    pub fn new() -> Self {
        let mut scenarios = Vec::new();
        scenarios.push(Box::new(WindowStressTest::new(1000)));
        scenarios.push(Box::new(SyncStressTest::new(1000)));
        scenarios.push(Box::new(BootStressTest::new()));
        scenarios.push(Box::new(MemoryStressTest::new(95)));
        scenarios.push(Box::new(NetworkStressTest::new(500)));
        scenarios.push(Box::new(VoiceStressTest::new()));
        Self { scenarios }
    }
    
    pub async fn run_all(&mut self) -> Vec<StressResult> {
        let mut results = Vec::new();
        for scenario in &mut self.scenarios {
            results.push(scenario.run().await);
        }
        results
    }
}

// Window stress test
pub struct WindowStressTest {
    count: usize,
}

#[async_trait]
impl StressScenario for WindowStressTest {
    async fn run(&mut self) -> StressResult {
        let start = Instant::now();
        
        // Создать N окон
        for i in 0..self.count {
            let window = Window::new(format!("Window {}", i));
            window_manager.add(window);
        }
        
        // Измерить FPS
        let fps = benchmark_fps(Duration::from_secs(10));
        
        // Измерить memory
        let memory = get_memory_usage();
        
        StressResult {
            name: "window_stress",
            passed: fps > 55.0 && memory < 2048 * 1024 * 1024,
            fps,
            memory,
            duration: start.elapsed(),
        }
    }
}

// Sync stress test
pub struct SyncStressTest {
    document_count: usize,
}

#[async_trait]
impl StressScenario for SyncStressTest {
    async fn run(&mut self) -> StressResult {
        let start = Instant::now();
        
        // Создать N документов
        let docs: Vec<_> = (0..self.document_count)
            .map(|i| Document::new(format!("doc-{}", i), "Lorem ipsum..."))
            .collect();
        
        // Синхронизировать
        let sync_start = Instant::now();
        for doc in &docs {
            crdt.sync(doc).await?;
        }
        let sync_time = sync_start.elapsed();
        
        StressResult {
            name: "sync_stress",
            passed: sync_time < Duration::from_secs(1),
            sync_time,
            document_count: self.document_count,
            duration: start.elapsed(),
        }
    }
}
```

---

## Шаги реализации

### Шаг 10.1: Performance Budget (2 дня)

1. Определить бюджеты для всех подсистем
2. Metrics collector
3. Budget checking (Green/Yellow/Orange/Red)
4. Dashboard для визуализации
5. Тест: фрейм > 16.67 мс → Red alert

### Шаг 10.2: Profiling (3 дня)

1. Span-based profiler
2. Chrome trace export
3. Real-time metrics (FPS, memory, CPU)
4. On-screen profiler (F12 для разработчиков)
5. Тест: профилирование render loop

### Шаг 10.3: Rendering Optimizations (4 дня)

1. Occlusion culling
2. Damage tracking
3. Texture atlas
4. Glyph cache
5. Instanced rendering
6. Тест: 1000 окон → > 55 FPS

### Шаг 10.4: Memory Optimizations (3 дня)

1. LRU cache для VFS
2. Texture compression (BC7/ASTC)
3. GPU memory budget
4. Arena allocator
5. Object pooling
6. Тест: memory < 2 GB для base конфигурации

### Шаг 10.5: P2P Optimizations (3 дня)

1. Bloom filter для sync
2. Delta compression
3. Batch sync
4. Kademlia routing
5. Circuit breaker
6. Тест: sync 1000 документов < 1 сек

### Шаг 10.6: Stress Tests (4 дня)

1. Window stress (1000 окон)
2. Sync stress (1000 документов)
3. Boot stress (cold/warm)
4. Memory stress (95% RAM)
5. Network stress (500 мс latency)
6. Voice stress (continuous recognition)
7. Тест: все stress tests проходят

### Шаг 10.7: CI/CD Pipeline (3 дня)

1. GitHub Actions / GitLab CI
2. Build matrix (Windows, macOS, Linux, Android)
3. Unit tests
4. Integration tests
5. Stress tests (nightly)
6. Performance regression detection
7. Тест: CI проходит на всех платформах

---

## Критерии приёмки

- [ ] Frame time < 16.67 мс при 100 окон
- [ ] Input latency < 8 мс
- [ ] Cold boot < 2 сек
- [ ] Warm boot < 500 мс
- [ ] Base memory < 2 GB
- [ ] P2P sync 1000 документов < 1 сек
- [ ] ASR latency < 500 мс
- [ ] TTS latency < 100 мс
- [ ] Stress tests: все проходят
- [ ] CI: зелёный на всех платформах
- [ ] No regression: метрики не ухудшились

---

## Placeholder'ы

| Placeholder | Замена в этапе | Примечание |
|-------------|----------------|------------|
| Profiling: только development | Post-release | Production profiling (sampling) |
| Stress tests: только synthetic | Post-release | Real-world load testing |
| CI: только GitHub Actions | Post-release | Self-hosted runners |

---

## Cross-reference

| Компонент | Слои |
|-----------|------|
| Performance Budget | layer-8 §17.1, layer-11 (performance targets) |
| Profiling | layer-8 §17.2, layer-11 (debugging) |
| Rendering Optimizations | layer-8 §3.8, layer-1 (60 FPS) |
| P2P Optimizations | layer-8 §11.3, layer-5 (P2P sync) |
| Stress Tests | layer-8 §17.3, project/stress-tests.md |
| CI/CD | layer-8 §18, layer-11 (development workflow) |

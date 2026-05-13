# AGENTS.md — Workspace Project Configuration

## Project: Workspace

Cross-platform overlay runtime providing a unified, secure workspace across Windows, macOS, Linux, Android, and iOS. Leverages host OS drivers while delivering a consistent user experience, offline-first P2P sync, and local AI inference.

### Documentation Structure

```
os/
├── AGENTS.md                    # This file — project configuration
├── archive/                     # Original brainstorming sessions + code snapshots (read-only)
│   ├── README.md                # What lives in archive/
│   ├── core.md                  # Core brainstorm
│   ├── architector.md           # Technical architecture review
│   ├── marketolog.md            # Marketing strategy
│   ├── investor.md              # Investor pitch draft (internal)
│   ├── gazprom.md               # Industrial case study (Gazprom)
│   ├── gorynych.md              # Yandex/Sber/VK consortium scenario
│   └── demo/                    # Phase 0 prototype snapshot before refactoring
├── layers/                      # Design layers (top-down)
│   ├── layer-1-user-experience.md          # UX + Space: user-facing layer
│   ├── layer-2-ai.md                       # AI layer: Intent API, Voice, Generative UI
│   ├── layer-3-system-split.md             # Front (Shell) vs Back (Backoffice)
│   ├── layer-4-installation-scenarios.md   # Installation & deployment
│   ├── layer-5-devices.md                  # Devices & media: USB, disks, network, P2P
│   ├── layer-6-apps.md                     # App model: 5 integration levels
│   ├── layer-7-security.md                 # Security: cross-layer document
│   ├── layer-8-technical-decomposition.md  # Subsystems: technical decomposition
│   ├── layer-9-hardware-requirements.md    # Hardware requirements
│   ├── layer-10-business-model.md          # Business model & go-to-market
│   └── layer-11-developer-reference.md     # Aggregated developer reference
├── plan/                        # Implementation plan: 37 phases + roadmap
│   ├── README.md                # Splitting principles, phase summary
│   ├── roadmap.md               # Human-readable description of all 37 phases
│   └── phase-01..37             # Detailed phase specifications
├── src/                         # Source code
│   ├── demo/                    # Phase 0: playable prototype (disposable)
│   ├── display_server/
│   ├── host_shim/
│   ├── island_mode/
│   └── micro_kernel/
├── tests/                       # Cross-phase test specifications
└── log/                         # Development session logs
```

### Archive Directory

`archive/` is **read-only historical storage**. Two kinds of content:

1. **Brainstorming notes** (`core.md`, `architector.md`, etc.) — raw role-play discussions and early drafts that preceded the structured `layers/` documentation.
2. **Code snapshots** (`demo/`, etc.) — disposable prototypes captured before refactoring. The active code lives in `src/`; the snapshot exists so lessons learned can be traced back to original implementations.

> Never edit files in `archive/`. If you need to revive an idea, copy it out and evolve it in `src/` or `layers/`.

### Language Policy

- **Project documentation:** Russian
- **Source code, commits, and API docs:** English
- **Public-facing docs:** Bilingual (Russian primary, English secondary)

### Before Committing

1. Format: Markdown, headers `##`, subsections `###`
2. Each document must be self-contained — readable without the others
3. Cross-reference: link to other documents as `[See layer-3-system-split.md](layer-3-system-split.md)`

### Build Commands

#### Windows (native)

```powershell
# Phase 0 Demo (Rust + wgpu + winit)
cd src; cargo run --bin demo

# Host Shim & Display Server (Rust)
cd src/host_shim; cargo build
cd src/display_server; cargo build

# Micro-Kernel & Runtime (Bun/TypeScript)
cd src/micro_kernel; bun install; bun run build

# Run tests
cargo test
bun test

# Linting (must pass on Windows AND WSL before committing)
cargo clippy -- -D warnings
```

#### WSL Ubuntu (cross-platform verification)

```bash
# Build (from /mnt/c mount — slower I/O, ~25s cold)
wsl -d Ubuntu -- bash -c "source ~/.cargo/env && cd /mnt/c/dev/games/ts/coreos/src && cargo build --bin demo"

# Run tests
wsl -d Ubuntu -- bash -c "source ~/.cargo/env && cd /mnt/c/dev/games/ts/coreos/src && cargo test"

# Linting
wsl -d Ubuntu -- bash -c "source ~/.cargo/env && cd /mnt/c/dev/games/ts/coreos/src && cargo clippy -- -D warnings"

# Run demo (WSLg required for window display)
wsl -d Ubuntu -- bash -c "source ~/.cargo/env && DISPLAY=:0 RUST_LOG=warn /mnt/c/dev/games/ts/coreos/src/target/debug/demo"

# Run demo in background
wsl -d Ubuntu -- bash -c "source ~/.cargo/env && DISPLAY=:0 RUST_LOG=warn setsid /mnt/c/dev/games/ts/coreos/src/target/debug/demo </dev/null >/tmp/coreos-demo.log 2>&1 &"

# Kill background demo
wsl -d Ubuntu -- pkill -f "target/debug/demo"
```

> **Note:** WSL uses Mesa llvmpipe (CPU Vulkan) — expect ~25 FPS instead of 60. GPU passthrough (dxgkrnl) is not enabled by default. The WSL distro is `Ubuntu` (verify with `wsl --list --verbose`).

## Coding Standards

### Architecture
- **Single Responsibility:** One module — one concern. Rendering, input, business logic, and resource management must live in separate modules.
- **No God Objects:** A struct must not accumulate unrelated state. If a struct has more than ~8 fields, split it into subsystems.
- **Layered Design:** Keep host/shim, display server, and app logic isolated. The app must not directly touch wgpu command encoders.

### Code Quality
- **DRY:** Extract duplication immediately. Two similar blocks → one generic function or trait.
- **Type Safety:** Prefer newtypes (`struct Pixel(u32)`, `struct Ndc(f32)`) over raw primitives when mixing coordinate spaces.
- **Error Handling:** Use `Result` everywhere. `unwrap` / `expect` are allowed only for truly unrecoverable invariants (e.g., shader compilation at startup). Log errors with context before propagating.
- **Function Size:** Keep functions under ~40 lines. Extract helper functions for readability.
- **Module Size:** Keep modules under ~400 lines. Split by responsibility.
- **No Inline Shaders:** WGSL/HLSL/GLSL live in separate `.wgsl` / `.hlsl` files, loaded at runtime or compiled with `include_str!`.
- **Unsafe:** Requires a `// SAFETY:` comment explaining why the invariant holds and why it cannot be expressed in safe Rust.
- **Constants:** No magic numbers. All colors, sizes, thresholds, and paths are named constants or config values.
- **Documentation:** Every `pub` item needs a doc comment. Internal items need comments when the logic is non-obvious.

### What Makes Code "Excellent"
1. **A new teammate can read it without asking questions.** Self-documenting through names and structure.
2. **Changing one feature does not touch unrelated files.** High cohesion, low coupling.
3. **Tests exist for every non-trivial decision.** Unit tests for pure logic, integration tests for subsystems.
4. **No warnings on `cargo clippy -- -D warnings`.** Zero tolerance for dead code, unused imports, or style violations.
5. **Resource lifecycle is explicit.** GPU buffers, textures, and handles are created, updated, and destroyed in predictable places.
6. **Performance is measured, not guessed.** Bottlenecks are identified with profiling before optimization.
7. **The code is boring.** Clever tricks are avoided; straightforward, idiomatic Rust is preferred.

## Multi-Platform Support

### Verified Platforms
- **Windows 11** (native): AMD Radeon 780M, Vulkan backend, wgpu 22.1
- **WSL2 Ubuntu 24.04**: Mesa llvmpipe (CPU Vulkan), WSLg Wayland compositor

### Cross-Platform Checklist
- Use `#[cfg(target_os = "...")]` for OS-specific paths (fonts, system dirs)
- Keep shader sources in `.wgsl` files loaded via `include_str!` — no platform differences
- wgpu + winit handle windowing abstraction; no platform-specific window code needed
- Run `cargo clippy -- -D warnings` on **both** Windows and Linux before committing

## Known Issues

### D3D12 Validation Warnings from OBS Studio Vulkan Layer

On Windows systems with **OBS Studio** installed, its implicit Vulkan capture layer (`VK_LAYER_OBS_HOOK`) injects D3D12 command lists that trigger `INVALID_SUBRESOURCE_STATE` validation warnings in wgpu/Vulkan logs (especially on AMD GPUs).

- **Not our bug** — this occurs inside OBS's internal D3D12 capture path.
- **Impact:** Log noise only; does not affect stability or rendering correctness.
- **Workaround:** Disable the OBS Vulkan capture layer for the current shell session:
  ```bash
  $env:DISABLE_VULKAN_OBS_CAPTURE=1
  cargo run --bin demo
  ```
- **Permanent fix:** Uninstall OBS Studio or disable its Vulkan capture hook globally.

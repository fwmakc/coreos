# AGENTS.md — CORE OS Project Configuration

## Project: CORE OS

Cross-platform overlay runtime providing a unified, secure workspace across Windows, macOS, Linux, Android, and iOS. Leverages host OS drivers while delivering a consistent user experience, offline-first P2P sync, and local AI inference.

### Documentation Structure

```
os/
├── AGENTS.md                    # This file — project configuration
├── archive/                     # Original brainstorming sessions
│   ├── core.md                  # Core brainstorm
│   ├── architector.md           # Technical architecture review
│   ├── marketolog.md            # Marketing strategy
│   ├── investor.md              # Investor pitch draft (internal)
│   ├── gazprom.md               # Industrial case study (Gazprom)
│   └── gorynych.md              # Yandex/Sber/VK consortium scenario
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
└── src/                         # Source code
    ├── display_server/
    ├── host_shim/
    ├── island_mode/
    └── micro_kernel/
```

### Language Policy

- **Project documentation:** Russian
- **Source code, commits, and API docs:** English
- **Public-facing docs:** Bilingual (Russian primary, English secondary)

### Before Committing

1. Format: Markdown, headers `##`, subsections `###`
2. Each document must be self-contained — readable without the others
3. Cross-reference: link to other documents as `[See layer-3-system-split.md](layer-3-system-split.md)`

### Build Commands

```bash
# Host Shim & Display Server (Rust)
cd src/host_shim && cargo build
cd src/display_server && cargo build

# Micro-Kernel & Runtime (Bun/TypeScript)
cd src/micro_kernel && bun install && bun run build

# Run tests
cargo test
bun test
```

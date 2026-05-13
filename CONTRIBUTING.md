# Contributing to Workspace

Thank you for your interest in Workspace! This document outlines how to participate in the project.

## Code of Conduct

Please read and follow our [Code of Conduct](CODE_OF_CONDUCT.md).

## How to Contribute

### Reporting Issues

- Use GitHub Issues to report bugs or request features.
- Provide a clear title and description.
- Include steps to reproduce, expected behavior, and actual behavior.
- Mention your OS and version.

### Pull Requests

1. Fork the repository.
2. Create a feature branch (`git checkout -b feature/my-feature`).
3. Make your changes.
4. Ensure tests pass (`cargo test`, `bun test`).
5. Ensure formatting is correct (`cargo fmt`, `bun run lint`).
6. Commit with a clear message in **English**.
7. Open a Pull Request with a detailed description.

### Commit Message Style

- Use English for all commit messages.
- Format: `<area>: <description>` (e.g., `host_shim: add Windows event loop`).

### Documentation

- Project docs (`layers/`, `plan/`, `archive/`): Russian.
- Code comments, API docs, and commit messages: English.
- Public-facing docs: bilingual (Russian primary, English secondary).

## Development Setup

### Rust (Host Shim + Display Server)

```bash
cd src
rustc --version  # >= 1.78
cargo build
cargo test
```

### TypeScript / Bun (Micro-Kernel + Apps)

```bash
cd src/micro_kernel/ts  # path may change as project evolves
bun install
bun run build
bun test
```

## Questions?

Open a Discussion on GitHub or contact the team at team@Workspace.dev.

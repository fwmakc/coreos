# Changelog

All notable changes to Workspace will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.0-alpha.1] - 2026-05-12

### Added
- Initialized Rust workspace with 5 crates: `host_shim`, `display_server`, `micro_kernel`, `island_mode`, `fuzzy_search`, plus `integration_tests`.
- **Phase 0 — Playable Demo:** Introduced accelerated prototype phase for tangible UX before foundation completion.
- Added real test suite: 120 tests passing (winit window, wgpu adapter, SQLite WAL + checkpoint + migrations, thread IPC, Levenshtein fuzzy search, property-based tests via proptest).
- Added test runner CLI (`tests/runners/workspace-test-runner`) with TC-ID mapping and JSON reporting.
- Added CI pipeline via GitHub Actions (Rust build, test, clippy, fmt; markdown lint; test report artifact).
- Added project infrastructure: `README.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`, `NOTICE`, `PROJECT_STATUS.md`, `CHANGELOG.md`.
- Added development log system (`log/README.md`, `log/phase-00-playable-demo.md`).
- Added issue and pull request templates.
- Added `.gitignore` and `archive/README.md` with disclaimer.
- Established bilingual documentation policy (Russian for project docs, English for code and commits).

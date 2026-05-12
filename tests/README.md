# Test Specifications

This directory contains acceptance-test specifications for each implementation phase.

## Structure

Each `phase-NN-*.md` file corresponds to a phase in [`plan/`](../plan/) and defines:

- **Test scenarios** — functional and non-functional requirements
- **Acceptance criteria** — what must pass for the phase to be considered complete
- **Integration checks** — how this phase interacts with previous ones

## Running Tests

### Rust (Phases 1–11)

```bash
cd src
cargo test
```

### TypeScript / Bun (Phases 12–37)

```bash
cd src/micro_kernel/ts  # path may change as project evolves
bun test
```

## Test Runners

Platform-specific test runners live in `runners/`.

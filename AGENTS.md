# Vidz - Agent Instructions

## Project Structure

- `/` - Tauri 2.x application (SolidJS frontend + Rust backend)
- `docs/` - active project documentation
- `docs/audits/` - audits and phased roadmap checklists
- `docs/_legacy/` - historical planning/task docs

## Current Source-of-Truth Docs

- `docs/ARCHITECTURE.md`
- `docs/OPERATIONS.md`
- `docs/PERFORMANCE.md`
- `docs/audits/optimization-roadmap.md`
- `docs/audits/session-handoff.md`

## Commands

### Frontend (from repo root)

```bash
pnpm install
pnpm dev
pnpm build
pnpm exec tsc --noEmit
pnpm lint
pnpm test
```

### Backend (from `src-tauri/`)

```bash
cargo check
cargo build
cargo clippy
cargo test
```

### Full App (from repo root)

```bash
pnpm tauri dev
pnpm tauri build
```

## Requirements

- Rust 1.70+
- Node.js 18+
- pnpm
- `ffmpeg` and `ffprobe` in PATH or bundled in `src-tauri/bin/`

## Code Style

- Frontend: TypeScript + SolidJS, minimal comments
- Backend: Rust 2021 edition, explicit error handling
- No inline comments unless logic is non-obvious

## Optimization Guardrails

- Preserve feature depth (do not reduce supported concurrent playback behavior to hide perf issues)
- Prefer batching/backpressure and scheduling improvements over hard feature cuts
- Keep scan and UI updates incremental for large libraries
- Validate with real large-library workflows before closing perf tasks

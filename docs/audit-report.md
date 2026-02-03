# Vidz Audit Report

**Date:** 2026-01-30

## Scope

Audit against `docs/plan.md` and `docs/TASKS.md` for missing or incomplete items. No code changes in this report.

## Overall Status

Core product goals are achieved: the app is functional with grid playback, focused player, sorting/filtering, file watching, and thumbnail/metadata caching. Remaining work is now limited to manual verification (10k smoke test) and the explicit 10k placeholder prototype (out of scope for this phase).

## Implemented (Highlights)

- Tauri 2.x + SolidJS + TanStack Virtual grid
- SQLite-backed library cache with metadata and thumbnails
- Background scanning and watcher auto-import
- Autoplay/hover playback with focused player pausing grid
- Sorting and filtering (as finalized)
- Watched folders persistence
- Scan progress indicator + empty states
- Bundled ffmpeg/ffprobe and resource lookup in Rust
- Windows bundle icons configured in `tauri.conf.json`

## Gaps / Not Yet Implemented (from TASKS.md)

### Phase 1 — Scaffolding

- **1.4** 10k placeholder grid prototype (explicit benchmark prototype not present) **out of scope for this phase**
- **1.5** Rust tooling setup (rustfmt/clippy config & scripts) ✅
- **1.6** Frontend tooling setup (eslint/prettier minimal) ✅

### Phase 2 — Metadata

- **2.3.4** Skip ffprobe if mtime unchanged and fields exist ✅

### Phase 6 — Performance Hardening

- **6.9** Scan cancellation support ✅

### Phase 7 — Polish & Release

- **7.4** Surface non-fatal failures subtly in UI ✅
- **7.6** Verify path resolution in production builds ✅
- **7.7** Configure installer metadata beyond icons (publisher, description, etc.) ✅
- **7.8** Manual 10k library smoke test (pending)
- **7.9** Rust unit tests (DB upsert, ID generation) ✅
- **7.10** Frontend tests (sorting/filtering correctness) ✅

## Notes on Items Marked Unchecked but Present

The task list still showed **2.3.1** and **2.4.1** (ffprobe/ffmpeg bundling) unchecked, but `src-tauri/bin/` contains the binaries and `tauri.conf.json` includes `resources: ["bin/*"]`. These are now marked complete in `docs/TASKS.md`.

## Optimizations (Industry-Standard Ideas)

### Frontend Rendering

- **Reduce reactive churn**: ensure derived lists (sorted/filtered) are memoized once per change, not per render.
- **Virtualization tuning**: adjust overscan based on GPU/RAM; consider dynamic overscan during fast scroll.
- **Video element pooling**: reuse a limited pool of `<video>` elements to avoid mount/unmount spikes.
- **Visibility gating**: use IntersectionObserver or virtualizer range thresholds to avoid decoding offscreen clips.
- **CSS containment**: add `contain: content` to tiles/rows to reduce layout/reflow cost.

### Media Pipeline

- **Thumb generation batcher**: queue thumbnails in batches with backpressure based on UI activity.
- **Thumb size discipline**: generate thumbnails at display resolution only; avoid oversized JPEGs.
- **Decode hints**: use `preload="metadata"` for grid videos when autoplay is off.

### Database / Storage

- **WAL mode + tuned pragmas**: ensure SQLite runs in WAL with tuned cache size for large libraries.
- **Bulk upserts**: batch DB writes per folder scan to reduce fsync overhead.
- **Thumb cleanup jobs**: background sweep for orphaned thumbnails.

### File Watching & Scanning

- **Scan cancellation**: allow cancel token to stop current scan without blocking UI.
- **Incremental scanning**: persist last scan cursor and only rescan deltas on startup.
- **Debounce strategy**: adaptive debounce for bursty folder changes (large copy operations).

### Tauri / Build

- **Release CSP**: reintroduce a minimal CSP for production safety.
- **Disable devtools**: ensure devtools are not available in release builds.
- **Installer metadata**: add publisher, description, and upgrade code for Windows installer polish.

## Latest Work Progress

Summary:

Implemented scan cancellation support with cancel events, UI controls, and backend cancel state handling.
Added non-fatal warning surfacing (toast + toolbar status) and warning emissions for metadata/thumbnail failures.
Completed tooling and test scaffolding: Rust fmt/clippy config, ESLint/Prettier/Vitest setup, Rust/TS unit tests, installer metadata, and updated docs/TASKS/audit report.
Tests:

pnpm lint
pnpm test
cargo check --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml

## Suggested Next Steps

1. Run the 10k library manual test and log scroll/CPU/IO metrics.
2. Decide whether to build the 10k placeholder prototype (task 1.4) or rely on manual testing only.

# Vidz

Local-first desktop viewer for large clip libraries (10k+), built for smooth scroll, fast incremental loading, and in-grid playback.

## Stack

- Tauri 2.x (Rust backend)
- SolidJS + Vite (frontend)
- TanStack Virtual (grid virtualization)
- SQLite (`rusqlite`) for library cache
- `ffprobe` (metadata) + `ffmpeg` (thumbnails)

## Key Behavior

- Responsive virtualized grid with density control
- Incremental scan surfacing (clips appear in batches during scan)
- Progressive metadata/thumb fill while scan is active
- Focused player mode pauses grid playback
- Autoplay for visible clips, with hover playback when autoplay is off
- Sorting: size, resolution, aspect, duration, folder, favorites
- Filtering: folder and favorites
- Recursive watched-folder auto-import via file watcher

## Performance Notes

Recent optimizations:

- Batched DB upserts during scan (`upsert_videos_batch`)
- Batched `library:discovered` emission during scan
- Throttled background job kicks while scanning
- Viewport-based autoplay gating with configurable concurrency cap
- Thumbnail-first tile rendering + conditional video source attachment
- CSS containment on grid/tile containers

## Prerequisites

- Node.js 18+
- pnpm
- Rust 1.70+
- `ffmpeg` and `ffprobe` in PATH or bundled in `src-tauri/bin/`

## Development

From repo root:

```bash
pnpm install
pnpm dev
pnpm exec tsc --noEmit
pnpm lint
pnpm test
```

Backend checks:

```bash
cargo check --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml
cargo test --manifest-path src-tauri/Cargo.toml
```

Run full app:

```bash
pnpm tauri dev
```

Build release:

```bash
pnpm tauri build
```

## Docs

- `docs/ARCHITECTURE.md` - system architecture and event/data flow
- `docs/OPERATIONS.md` - runbooks, troubleshooting, release checks
- `docs/PERFORMANCE.md` - performance strategy, knobs, and next targets
- `docs/audits/optimization-roadmap.md` - phased checklist for optimization work
- `docs/audits/audit-report.md` - historical audit summary
- `docs/audits/runtime-process-audit.md` - media process/caching audit
- `docs/_legacy/` - previous plan/task snapshots

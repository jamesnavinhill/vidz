# Vidz Operations

Last updated: 2026-02-07

## 1. Prerequisites

- Node.js 18+
- pnpm
- Rust 1.70+
- `ffmpeg` and `ffprobe` available from:
  - bundled resources (`src-tauri/bin/` in project, `resources/bin/` in packaged app), or
  - system `PATH`

## 2. Daily Development Commands

From repo root:

```bash
pnpm install
pnpm dev
pnpm exec tsc --noEmit
pnpm lint
pnpm test
```

From `src-tauri/`:

```bash
cargo check
cargo clippy
cargo test
```

Full app:

```bash
pnpm tauri dev
pnpm tauri build
```

## 3. Runtime Operational Flow

1. App boots from cached DB.
2. Watched folders are loaded and watcher starts.
3. New scan or watcher ingestion upserts files.
4. Metadata and thumbnails are processed by background job queue.
5. Frontend receives batched `discovered`/`updated` events.

## 4. Runbook: Large Library Scan Validation

1. Add a folder with a large clip set.
2. Verify clips appear incrementally during scan (not only at end).
3. Verify thumbnails/metadata continue filling while scan is running.
4. Scroll aggressively through the grid and confirm smoothness.
5. Open/close focused player and verify grid playback pauses/resumes correctly.
6. Toggle autoplay off and validate hover playback still works.

## 5. Troubleshooting

### No metadata or thumbnails generated

- Confirm `ffprobe`/`ffmpeg` are available.
- Check warnings surfaced in UI (`library:warning`).
- In packaged app, verify resources include `bin/ffprobe.exe` and `bin/ffmpeg.exe`.

### Scan appears stalled

- Verify scan progress events are changing.
- Use Cancel, then retry scan.
- Confirm selected folder actually contains supported video extensions.

### High CPU during scroll

- Lower density temporarily to reduce simultaneously visible clips.
- Keep autoplay enabled only when desired; disable to hover-play only.
- Check `maxConcurrentVideos` setting in Settings.

### Missing clips after folder remove

- Expected behavior: removing watched folder removes matching DB records and thumbnail files.

## 6. Data Hygiene

- Orphaned thumbnail cleanup runs on startup.
- Thumbnail files are removed when watched-folder media is removed through app flow.

## 7. Recommended Release Checks

1. `pnpm exec tsc --noEmit`
2. `pnpm lint`
3. `pnpm test`
4. `cargo check --manifest-path src-tauri/Cargo.toml`
5. `cargo test --manifest-path src-tauri/Cargo.toml`
6. Manual smoke test with large library (target: 10k+ clips)

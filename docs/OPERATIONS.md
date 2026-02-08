# Vidz Operations

Last updated: 2026-02-08

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
3. Incremental watched-folder startup scan runs using persisted folder cursors.
4. New scan or watcher ingestion upserts files (watcher path is adaptive-debounced).
5. Metadata and thumbnails are processed by background job queue.
6. Watcher emits saturation/debounce/recovery telemetry and runs periodic reconciliation.
7. Frontend receives batched `discovered`/`updated` events plus telemetry events.

## 4. Runbook: Large Library Scan Validation

1. Add a folder with a large clip set.
2. Verify clips appear incrementally during scan (not only at end).
3. Verify thumbnails/metadata continue filling while scan is running.
4. Verify scan-batch telemetry is emitted (`library:scan_batch`).
5. Scroll aggressively through the grid and confirm smoothness.
6. Verify job telemetry emits under scroll load (`library:job_telemetry`).
7. Stress-copy/move files into watched folders and verify watcher telemetry (`library:watcher_telemetry`) shows adaptive debounce without persistent drops.
8. Leave app running and confirm reconciliation recovers any intentionally missed watcher updates.
9. Open/close focused player and verify grid playback pauses/resumes correctly.
10. Toggle autoplay off and validate hover playback still works.

## 5. Troubleshooting

### No metadata or thumbnails generated

- Confirm `ffprobe`/`ffmpeg` are available.
- Check warnings surfaced in UI (`library:warning`).
- In packaged app, verify resources include `bin/ffprobe.exe` and `bin/ffmpeg.exe`.

### Scan appears stalled

- Verify scan progress events are changing.
- Use Cancel, then retry scan.
- Confirm selected folder actually contains supported video extensions.
- Check whether startup scan is in incremental mode (unchanged files are intentionally skipped).

### High CPU during scroll

- Lower density temporarily to reduce simultaneously visible clips.
- Keep autoplay enabled only when desired; disable to hover-play only.
- Check `maxConcurrentVideos` setting in Settings.
- Verify UI activity hints are flowing to backend (`update_ui_activity`) so backpressure can engage.
- Verify decode-heavy clipping behavior is active during high scroll velocity (non-heavy clips should remain prioritized).

### Watcher misses or delayed updates

- Inspect watcher telemetry (`library:watcher_telemetry`) for queue saturation/dropped events.
- If saturation appears, allow the automatic recovery reconciliation run to complete.
- Confirm watched path remains in settings and on-disk path still exists.

### Missing clips after folder remove

- Expected behavior: removing watched folder removes matching DB records and thumbnail files.

## 6. Data Hygiene

- Orphaned thumbnail cleanup runs on startup.
- Thumbnail files are removed when watched-folder media is removed through app flow.
- Reconciliation scan also removes stale DB records for disappeared files.

## 7. Recommended Release Checks

1. `pnpm exec tsc --noEmit`
2. `pnpm lint`
3. `pnpm test`
4. `cargo check --manifest-path src-tauri/Cargo.toml`
5. `cargo test --manifest-path src-tauri/Cargo.toml`
6. Manual smoke test with large library (target: 10k+ clips)

# Vidz Architecture

Last updated: 2026-02-08

## 1. System Overview

Vidz is a Tauri 2.x desktop app with a SolidJS frontend and a Rust backend.

- Frontend (`src/`): virtualized media grid, playback orchestration, user settings.
- Backend (`src-tauri/src/`): scanning, file watching, SQLite persistence, metadata/thumb jobs.
- Media tools: `ffprobe` for metadata, `ffmpeg` for thumbnails (bundled or PATH fallback).

## 2. Runtime Components

### Frontend

- `src/App.tsx`
  - Boots app state from backend.
  - Subscribes to Tauri events.
  - Batches incoming `library:discovered` and `library:updated` updates before writing to the store.
  - Runs incremental watched-folder scan on startup.
  - Kicks backend job processing during active scans (throttled).
- `src/components/VideoGrid.tsx`
  - Uses TanStack Virtual row virtualization.
  - Computes row heights from media aspect ratio.
  - Adaptive overscan based on scroll velocity.
  - Gated autoplay budget based on viewport rows and `maxConcurrentVideos`.
  - Idle-time decode warmup for near-viewport tiles.
  - Decode-heavy drop strategy under active scroll/decode load.
  - Optional dense-layout video mount pooling budget.
  - Near-future prefetch window and UI activity hint updates to backend job queue.
- `src/components/VideoTile.tsx`
  - Thumbnail-first rendering.
  - Video source attached for playable tiles, prefetch tiles, and idle warmup tiles.
  - Optional mount gating for dense-layout pooling evaluation.
  - Hover playback when autoplay is off.
- `src/store.ts`
  - Central app/UI state and derived sorting/filtering logic.

### Backend

- `src-tauri/src/commands/mod.rs`
  - Tauri command API surface.
  - Long-running scan uses `spawn_blocking`.
  - Startup incremental scan command (`scan_watched_folders_incremental`).
  - UI activity hint command (`update_ui_activity`) for backend scheduling.
  - Emits discovered clips in batches during scan, not only at completion.
- `src-tauri/src/scanner/mod.rs`
  - File discovery and `VideoItem` creation.
  - Incremental mode skips unchanged files using folder scan cursors.
  - Batched DB upserts and batched `library:discovered` event emission.
  - Emits `library:scan_batch` telemetry events (size and latency).
  - Metadata extraction includes codec capture (`codec_name`).
- `src-tauri/src/jobs/mod.rs`
  - Metadata and thumbnail background queue with retry budget.
  - Priority ordering from viewport-near IDs.
  - UI-aware backpressure (batch limits and dynamic thumbnail parallelism).
  - Dynamic thumbnail sizing/quality from estimated tile width hints.
  - Emits `library:job_telemetry` with categorized failure counters.
  - Concurrency controls via semaphores (`4` metadata, `2` thumbnails).
- `src-tauri/src/watcher/mod.rs`
  - Recursive folder watcher with adaptive debounce based on burst rate/queue pressure.
  - Queue saturation counters and telemetry (`library:watcher_telemetry`).
  - Saturation-triggered recovery and periodic reconciliation scan for watcher misses.
  - Ingests created/modified files and emits removal events.
- `src-tauri/src/db/mod.rs`
  - SQLite schema and access layer.
  - WAL + performance pragmas.
  - Batch upsert transaction support.

## 3. Core Data Flow

### Startup

1. Frontend invokes:
   - `get_library`
   - `get_app_settings`
   - `get_watched_folders`
2. Frontend starts watcher (`start_file_watcher`).
3. Frontend invokes incremental startup scan (`scan_watched_folders_incremental`).
4. If library has pending metadata/thumbs, frontend invokes `process_pending_jobs`.

### Add Folder / Scan

1. Frontend invokes `add_watched_folder(path)`.
2. Backend persists watched folder and starts scan.
3. Scanner discovers files and upserts in DB in batches.
4. Scanner emits `library:discovered` in batches while scan is still running.
5. Scanner emits `library:scan_batch` telemetry per batch.
6. Frontend flushes batched store updates and periodically triggers `process_pending_jobs`.
7. Job queue emits `library:updated` as metadata/thumbs are filled.

### Watcher Updates

1. `notify` event received.
2. Event is queued with adaptive debounce scheduling (burst-aware).
3. Debounced create/modify events upsert `VideoItem` records, emit `library:discovered`, and kick jobs.
4. Remove events delete DB rows and emit `library:removed`.
5. Periodic + saturation-triggered reconciliation scans heal missed watcher events.

## 4. Event Contract

- `library:discovered` (`VideoItem[]`): new or newly-seen files, batched.
- `library:updated` (`VideoItem[]`): metadata/thumb/favorite updates.
- `library:removed` (`string`): one removed id.
- `library:removed_bulk` (`string[]`): removed ids for folder removal.
- `library:scan_progress` (`ScanProgress`): current scan status.
- `library:scan_batch` (`ScanBatchTelemetry`): batch size and latency metrics.
- `library:scan_finished` (`()`): scan done.
- `library:scan_cancelled` (`()`): scan cancellation acknowledged.
- `library:job_telemetry` (`JobTelemetry`): queue processing/failure counters.
- `library:watcher_telemetry` (`WatcherTelemetry`): watcher debounce/saturation/recovery counters.
- `library:warning` (`string`): non-fatal warning.
- `library:watched_folders_updated` (`string[]`): persisted folder list changed.

## 5. Persistence Layout

- Database: `AppDataDir/Vidz/library.db`
- Thumbnails: `AppCacheDir/Vidz/thumbs/`
- Settings keys in SQLite:
  - `watched_folders`
  - `app_settings`
  - `scan_cursors`

## 6. Performance-Sensitive Design Choices

- Virtualized rows + computed row heights.
- Velocity-adaptive overscan + near-future prefetch window.
- Viewport-based autoplay gating to keep decode pressure bounded without reducing feature support.
- Decode-heavy codec shedding during high scroll/decode load.
- Idle warmup + dense-layout mount pooling evaluation to smooth first-frame latency and mount churn.
- Video source attachment only for playable tiles.
- Thumbnail-first tile rendering.
- Batched scan ingest (DB + UI event emission).
- Incremental startup scan cursor to skip unchanged ingest work.
- Throttled job queue kicks during active scanning for progressive metadata/thumb fill.
- Job priority/backpressure from live UI activity hints.
- Retry budget and categorized failure counters for media job reliability telemetry.
- Adaptive watcher debounce + periodic reconciliation for robustness under burst file operations.

# Vidz Architecture

Last updated: 2026-02-07

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
  - Kicks backend job processing during active scans (throttled).
- `src/components/VideoGrid.tsx`
  - Uses TanStack Virtual row virtualization.
  - Computes row heights from media aspect ratio.
  - Gated autoplay budget based on viewport rows and `maxConcurrentVideos`.
- `src/components/VideoTile.tsx`
  - Thumbnail-first rendering.
  - Video source attached only when tile is eligible to play.
  - Hover playback when autoplay is off.
- `src/store.ts`
  - Central app/UI state and derived sorting/filtering logic.

### Backend

- `src-tauri/src/commands/mod.rs`
  - Tauri command API surface.
  - Long-running scan uses `spawn_blocking`.
  - Emits discovered clips in batches during scan, not only at completion.
- `src-tauri/src/scanner/mod.rs`
  - File discovery and `VideoItem` creation.
  - Batched DB upserts and batched `library:discovered` event emission.
  - Metadata and thumbnail process invocation.
- `src-tauri/src/jobs/mod.rs`
  - Metadata and thumbnail background queue.
  - Concurrency controls via semaphores (`4` metadata, `2` thumbnails).
- `src-tauri/src/watcher/mod.rs`
  - Recursive folder watcher with debounce.
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
3. If library has pending metadata/thumbs, frontend invokes `process_pending_jobs`.

### Add Folder / Scan

1. Frontend invokes `add_watched_folder(path)`.
2. Backend persists watched folder and starts scan.
3. Scanner discovers files and upserts in DB in batches.
4. Scanner emits `library:discovered` in batches while scan is still running.
5. Frontend flushes batched store updates and periodically triggers `process_pending_jobs`.
6. Job queue emits `library:updated` as metadata/thumbs are filled.

### Watcher Updates

1. `notify` event received.
2. Debounced create/modify events upsert a single `VideoItem`.
3. Backend emits `library:discovered` and triggers job processing.
4. Remove events delete DB row and emit `library:removed`.

## 4. Event Contract

- `library:discovered` (`VideoItem[]`): new or newly-seen files, batched.
- `library:updated` (`VideoItem[]`): metadata/thumb/favorite updates.
- `library:removed` (`string`): one removed id.
- `library:removed_bulk` (`string[]`): removed ids for folder removal.
- `library:scan_progress` (`ScanProgress`): current scan status.
- `library:scan_finished` (`()`): scan done.
- `library:scan_cancelled` (`()`): scan cancellation acknowledged.
- `library:warning` (`string`): non-fatal warning.
- `library:watched_folders_updated` (`string[]`): persisted folder list changed.

## 5. Persistence Layout

- Database: `AppDataDir/Vidz/library.db`
- Thumbnails: `AppCacheDir/Vidz/thumbs/`
- Settings keys in SQLite:
  - `watched_folders`
  - `app_settings`

## 6. Performance-Sensitive Design Choices

- Virtualized rows + computed row heights.
- Viewport-based autoplay gating to keep decode pressure bounded without reducing feature support.
- Video source attachment only for playable tiles.
- Thumbnail-first tile rendering.
- Batched scan ingest (DB + UI event emission).
- Throttled job queue kicks during active scanning for progressive metadata/thumb fill.

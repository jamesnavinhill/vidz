# Vidz Runtime Process & Caching Audit

**Date:** 2026-01-30

## Scope

Review current ffmpeg/ffprobe execution, sidecar usage, batching/queueing, and caching. Updated to reflect completed improvements.

## ffmpeg/ffprobe Usage (Current)

- **Invocation points:**
  - `src-tauri/src/scanner/mod.rs`
    - `extract_metadata()` uses `std::process::Command` to call **ffprobe**.
    - `generate_thumbnail()` uses `std::process::Command` to call **ffmpeg**.
- **Paths:**
  - Resolved via `get_ffprobe_path()` and `get_ffmpeg_path()` in `src-tauri/src/commands/mod.rs`.
  - Prefer bundled binaries in `resources/bin/` if present; fallback to PATH.
- **Sidecar usage:**
  - No Tauri sidecar config is used here yet; binaries are launched directly with `Command::new(...)`.

## Why Console Windows Appear in Release (Resolved)

When the app is launched from the `.exe` (not from a terminal), each `Command::new(ffmpeg/ffprobe)` can create its own console window on Windows. We now apply `CREATE_NO_WINDOW` when spawning these processes in `scanner/mod.rs`, so the release build no longer spawns visible terminal windows per file.

## Sidecar vs. No-Window Process Flags

**No-window process flags** (simple fix):
- Use Windows-specific flags (e.g., `CREATE_NO_WINDOW`) when spawning ffmpeg/ffprobe.
- Minimal change, keeps current execution flow.
- Good for local tooling and bundled binaries.

**Tauri sidecar** (packaged helper):
- Configure sidecars in `tauri.conf.json` and invoke via Tauri APIs.
- Gives more structured lifecycle and distribution control.
- More involved to retrofit; best if you want standardized process management and arguments.

**Conclusion:** Given the current architecture, **no-window flags** were implemented as the cleaner, lower-risk fix. Sidecar remains a future option if we want centralized process management.

## Batching / Queueing (Current)

- **JobQueue** (`src-tauri/src/jobs/mod.rs`):
  - Metadata extraction uses a semaphore of **4** concurrent ffprobe jobs.
  - Thumbnail generation uses a semaphore of **2** concurrent ffmpeg jobs.
  - `process_all()` runs metadata first, then thumbnails, and prevents overlapping runs via `running` mutex.

- **Directory scanning** (`scanner::scan_directory`):
  - Walks files and upserts to DB one-by-one.
  - Reports progress to UI.
  - No chunked batching for DB writes, but does respect scan cancellation.

- **Watcher** (`src-tauri/src/watcher/mod.rs`):
  - Debounces events by 500ms.
  - Retries file size > 0 check (copy-in-progress handling).
  - On file create/modify: upserts and emits `library:discovered`.
  - Now triggers `JobQueue::process_all` after a new file is ingested, so metadata/thumbnail processing starts automatically for watcher events.

### Queueing Gaps / Improvement Ideas

- ✅ Implemented: watcher triggers job queue processing on create/modify.
- Optional batch DB updates during scan to reduce fsync overhead for very large libraries.
- Consider a unified job scheduler that accepts work from both initial scans and watcher events.

## Caching (Current)

- **Library cache:** SQLite database at `AppDataDir/Vidz/library.db`.
  - `upsert_video` preserves metadata/thumbnail when `mtime` is unchanged.
  - Metadata/thumbnail re-scan occurs if `mtime` changes or fields are missing.

- **Thumbnail cache:** JPEGs stored in `AppCacheDir/Vidz/thumbs/`.
  - Thumbs are removed when a video is deleted via watcher.

- **Settings cache:** stored in `settings` table (`watched_folders`, `app_settings`).

### Cache Improvements

- **Startup re-scan optimization:** Current behavior still iterates all files during scan. Consider tracking scan cursor or a folder hash to skip untouched directories.
- ✅ **Thumbnail hygiene:** `Database::cleanup_orphaned_thumbnails()` runs on startup to remove stale thumbnails.
- ✅ **Index tuning / WAL mode:** SQLite WAL + pragmas applied on database creation.
- **Metadata cache validation:** still relies on `mtime` checks; optional future work to add more granular cache keys.

## Recommendation Summary

1. ✅ **Hidden ffmpeg/ffprobe console windows** using Windows no-console flags on process spawn.
2. **JobQueue semaphores** remain at 4 metadata / 2 thumbs.
3. ✅ **Watcher events now bridge to JobQueue** for auto metadata/thumbnail processing.
4. ✅ **Cache hygiene**: startup cleanup of stale thumbnails + WAL pragmas.

## Relevant Code Locations

- ffmpeg/ffprobe calls: `src-tauri/src/scanner/mod.rs`
- Job batching: `src-tauri/src/jobs/mod.rs`
- File watcher + debounce: `src-tauri/src/watcher/mod.rs`
- Cache storage + cleanup: `src-tauri/src/db/mod.rs`
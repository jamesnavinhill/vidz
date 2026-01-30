# Vidz Runtime Process & Caching Audit

**Date:** 2026-01-30

## Scope

Review current ffmpeg/ffprobe execution, sidecar usage, batching/queueing, and caching. Provide improvement options only (no code changes).

## ffmpeg/ffprobe Usage (Current)

- **Invocation points:**
  - `app/src-tauri/src/scanner/mod.rs`
    - `extract_metadata()` uses `std::process::Command` to call **ffprobe**.
    - `generate_thumbnail()` uses `std::process::Command` to call **ffmpeg**.
- **Paths:**
  - Resolved via `get_ffprobe_path()` and `get_ffmpeg_path()` in `app/src-tauri/src/commands/mod.rs`.
  - Prefer bundled binaries in `resources/bin/` if present; fallback to PATH.
- **Sidecar usage:**
  - No Tauri sidecar config is used here yet; binaries are launched directly with `Command::new(...)`.

## Why Console Windows Appear in Release

When the app is launched from the `.exe` (not from a terminal), each `Command::new(ffmpeg/ffprobe)` can create its own console window on Windows. In `pnpm tauri dev`, the process is already attached to a console so the popups are less noticeable. This explains the difference you observed.

## Sidecar vs. No-Window Process Flags

**No-window process flags** (simple fix):
- Use Windows-specific flags (e.g., `CREATE_NO_WINDOW`) when spawning ffmpeg/ffprobe.
- Minimal change, keeps current execution flow.
- Good for local tooling and bundled binaries.

**Tauri sidecar** (packaged helper):
- Configure sidecars in `tauri.conf.json` and invoke via Tauri APIs.
- Gives more structured lifecycle and distribution control.
- More involved to retrofit; best if you want standardized process management and arguments.

**Conclusion:** Given the current architecture, **no-window flags** are the cleaner and lower-risk fix. Sidecar is more “official,” but requires extra setup and refactor.

## Batching / Queueing (Current)

- **JobQueue** (`app/src-tauri/src/jobs/mod.rs`):
  - Metadata extraction uses a semaphore of **4** concurrent ffprobe jobs.
  - Thumbnail generation uses a semaphore of **2** concurrent ffmpeg jobs.
  - `process_all()` runs metadata first, then thumbnails, and prevents overlapping runs via `running` mutex.

- **Directory scanning** (`scanner::scan_directory`):
  - Walks files and upserts to DB one-by-one.
  - Reports progress to UI.
  - No chunked batching for DB writes, but does respect scan cancellation.

- **Watcher** (`app/src-tauri/src/watcher/mod.rs`):
  - Debounces events by 500ms.
  - Retries file size > 0 check (copy-in-progress handling).
  - On file create/modify: upserts and emits `library:discovered`.
  - **It does not currently trigger metadata/thumbnail job processing**, so new files added by watcher will wait until `process_pending_jobs` is called elsewhere.

### Queueing Gaps / Improvement Ideas

- Add a small “job enqueue” step when watcher detects a new file (or trigger `process_all`).
- Optional batch DB updates during scan to reduce fsync overhead for very large libraries.
- Consider a unified job scheduler that accepts work from both initial scans and watcher events.

## Caching (Current)

- **Library cache:** SQLite database at `AppDataDir/Vidz/library.db`.
  - `upsert_video` preserves metadata/thumbnail when `mtime` is unchanged.
  - Metadata/thumbnail re-scan occurs if `mtime` changes or fields are missing.

- **Thumbnail cache:** JPEGs stored in `AppCacheDir/Vidz/thumbs/`.
  - Thumbs are removed when a video is deleted via watcher.

- **Settings cache:** stored in `settings` table (`watched_folders`, `app_settings`).

### Cache Gaps / Improvement Ideas

- **Startup re-scan optimization:** Current behavior still iterates all files during scan. Consider tracking scan cursor or a folder hash to skip untouched directories.
- **Thumbnail hygiene:** background cleanup for orphaned thumbs (if files removed outside watcher coverage).
- **Metadata cache validation:** optionally persist a `last_scanned` and reuse results unless size/mtime changes (partially present via `mtime` logic).
- **Index tuning / WAL mode:** ensure SQLite uses WAL + tuned pragmas for large libraries.

## Recommendation Summary

1. **Hide ffmpeg/ffprobe console windows** using Windows no-console flags on process spawn. This is the fastest, lowest-risk fix.
2. **Keep current JobQueue semaphores** (4 metadata / 2 thumbs) — they already act as a batch limiter.
3. **Bridge watcher events to the JobQueue** so new files picked up after launch automatically get metadata and thumbnails.
4. **Add cache hygiene**: optional cleanup of stale thumbnails and incremental scan skipping for unchanged folders.

## Relevant Code Locations

- ffmpeg/ffprobe calls: `app/src-tauri/src/scanner/mod.rs`
- Job batching: `app/src-tauri/src/jobs/mod.rs`
- File watcher + debounce: `app/src-tauri/src/watcher/mod.rs`
- Cache storage: `app/src-tauri/src/db/mod.rs`
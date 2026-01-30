# Vidz - Implementation Task List

> Generated: January 30, 2026  
> Status: MVP Implementation Complete - App Running

---

## Phase 0 — Pre-Implementation Setup (~1 hour)

- [x] **0.1** Lock implementation decisions and document in README
  - ffprobe (bundled) for metadata extraction
  - ffmpeg (bundled) for first-frame thumbnail generation
  - SQLite via rusqlite for library cache
- [x] **0.2** Define `VideoItem` data contract (Rust ⇄ UI)
  - `id` (stable hash of canonical path)
  - `path`, `folder`, `size_bytes`, `mtime`
  - `duration_ms`, `width`, `height`, `aspect_ratio`
  - `favorite` (bool), `thumb_path` (nullable)
- [x] **0.3** Document MVP "done" criteria

---

## Phase 1 — Project Scaffolding (~3 hours)

- [x] **1.1** Create Tauri 2.x + Vite + SolidJS project
- [x] **1.2** Configure Windows target (icons, product name, window defaults)
- [x] **1.3** Add TanStack Virtual dependency
- [ ] **1.4** Create 10k placeholder grid prototype to validate smooth scroll
- [ ] **1.5** Set up Rust tooling (rustfmt, clippy)
- [ ] **1.6** Set up frontend tooling (prettier, eslint minimal)
- [x] **1.7** Configure Tauri filesystem permissions (user-chosen directories only)
- [x] **1.8** Define app directories:
  - `AppDataDir/Vidz/library.db`
  - `AppCacheDir/Vidz/thumbs/`

---

## Phase 2 — Rust Backend Foundation (~2 days)

### 2.1 Database Layer

- [x] **2.1.1** Add rusqlite dependency
- [x] **2.1.2** Implement DB initialization and migrations
- [x] **2.1.3** Create `videos` table schema with indexes
- [x] **2.1.4** Implement CRUD operations (upsert, list, update favorite, update thumb)

### 2.2 File Discovery

- [x] **2.2.1** Implement directory scanning command
- [x] **2.2.2** Filter by video extensions (mp4, mkv, webm, avi, mov)
- [x] **2.2.3** Compute stable ID (hash of canonical path)
- [x] **2.2.4** Upsert basic fields (path, folder, mtime, size_bytes)

### 2.3 Metadata Extraction

- [ ] **2.3.1** Bundle ffprobe for Windows
- [x] **2.3.2** Implement `probe_video(path)` → duration, width, height
- [x] **2.3.3** Add timeout and error handling
- [ ] **2.3.4** Skip probe if mtime unchanged and fields exist

### 2.4 Thumbnail Generation

- [ ] **2.4.1** Bundle ffmpeg for Windows
- [x] **2.4.2** Implement first-frame thumbnail extraction → JPEG
- [x] **2.4.3** Output to `thumbs/{id}.jpg`
- [x] **2.4.4** Store thumb_path in DB

### 2.5 Background Processing

- [x] **2.5.1** Implement worker pool with job queues
- [x] **2.5.2** Separate queues: discovery (fast), probe (medium), thumbnail (heavy)
- [x] **2.5.3** Limit ffmpeg concurrency (2-4 concurrent)
- [x] **2.5.4** Implement event streaming to frontend:
  - `library:discovered`
  - `library:updated`
  - `library:scan_progress`
  - `library:scan_finished`

### 2.6 Tauri Commands

- [x] **2.6.1** `select_directories_and_scan()`
- [x] **2.6.2** `get_library()` → returns all VideoItems
- [x] **2.6.3** `set_favorite(id, bool)`

---

## Phase 3 — SolidJS Frontend Foundation (~2 days)

### 3.1 State Management

- [x] **3.1.1** Create Solid store for library state
- [x] **3.1.2** Add UI settings state (autoplay, sortMode, filterFolder, filterFavorites, focusedId)
- [x] **3.1.3** Wire Tauri commands (get_library on startup)
- [x] **3.1.4** Subscribe to library events and patch store

### 3.2 Virtualized Grid

- [x] **3.2.1** Implement TanStack Virtual grid virtualizer
- [x] **3.2.2** Responsive column count based on window width
- [x] **3.2.3** Implement density slider for tile sizing
- [x] **3.2.4** Render tiles with thumbnail-first strategy

### 3.3 Playback Behavior

- [x] **3.3.1** Implement autoplay toggle (default: ON)
- [x] **3.3.2** Define "active tiles" using virtualization range
- [x] **3.3.3** Mount `<video>` only on active tiles
- [x] **3.3.4** Implement play/pause orchestration:
  - If focusedId set → all grid videos pause
  - If autoplay ON → active tiles play
  - If autoplay OFF → show thumbnails only

### 3.4 Focused Player Mode

- [x] **3.4.1** Clicking tile sets focusedId
- [x] **3.4.2** Create dedicated player view (modal/overlay)
- [x] **3.4.3** Large video player with controls
- [x] **3.4.4** Close restores grid behavior

---

## Phase 4 — Product Features (~3 hours)

### Favorites

- [x] **4.1** Toggle favorite in focused player UI
- [x] **4.2** Persist via set_favorite command
- [x] **4.3** Emit library:updated event

### Sorting (frontend)

- [x] **4.4** File size sort
- [x] **4.5** Resolution sort (width × height)
- [x] **4.6** Aspect ratio sort
- [x] **4.7** Duration sort
- [x] **4.8** Folder path sort
- [x] **4.9** Favorites sort
- [x] **4.10** Stable secondary sort (by path)

### Filtering

- [x] **4.11** Folder filter dropdown (unique folders)
- [x] **4.12** Favorites filter toggle

### UI

- [x] **4.13** Density slider (tile size / column count)
- [x] **4.14** "Add Folder" button with native dialog
- [x] **4.15** Persist watched folders

---

## Phase 5 — File Watching & Auto-Import (~2 days)

- [x] **5.1** Add `notify` crate for directory watching
- [x] **5.2** Watch configured root folders recursively
- [x] **5.3** Debounce file event bursts
- [x] **5.4** On create/update: validate extension → upsert → enqueue jobs
- [x] **5.5** On remove: mark missing in DB, optionally delete thumbnail
- [x] **5.6** Emit events so frontend updates automatically
- [x] **5.7** Handle "file still copying" with retry backoff
- [x] **5.8** Cap queue size, log dropped events

---

## Phase 6 — Performance Hardening (~2 days)

### Startup

- [x] **6.1** Measure app start → first grid render time
- [x] **6.2** Ensure get_library is fast (indexes, minimal transforms)

### Scrolling

- [x] **6.3** Tune TanStack Virtual overscan settings
- [x] **6.4** Minimize DOM nodes and reactive computations per tile

### Video Decoding

- [x] **6.5** Enforce max simultaneous playing tiles (e.g., 16)
- [x] **6.6** "Play only when mostly visible" threshold if needed

### I/O

- [x] **6.7** Tune ffmpeg concurrency (2 thumbnails, 4 metadata)
- [x] **6.8** Ensure scanning doesn't block UI
- [ ] **6.9** Add scan cancellation support

---

## Phase 7 — Polish & Release (~3 hours)

### UX

- [x] **7.1** Empty states (no folders, scanning in progress)
- [x] **7.2** Scan progress indicator
- [x] **7.3** Settings panel (autoplay, density, watched folders)

### Error Handling

- [ ] **7.4** Surface non-fatal failures subtly

### Packaging

- [x] **7.5** Bundle ffmpeg/ffprobe in app resources
- [ ] **7.6** Verify path resolution in production builds
- [ ] **7.7** Configure icons and installer metadata

### Testing

- [ ] **7.8** Manual smoke test with 10k library
- [ ] **7.9** Rust unit tests (DB upsert, ID generation)
- [ ] **7.10** Frontend tests (sorting/filtering correctness)

---

## Dependency Summary

```
Phase 0 → Phase 1 → Phase 2 → Phase 3 → Phase 4 → Phase 5 → Phase 6 → Phase 7
                          ↘          ↗
                           Phase 4 depends on Phase 2 (DB) + Phase 3 (UI)
                           Phase 5 depends on Phase 2 (jobs) + Phase 4 (folder persistence)
```

---

## Dependency Notes

- **glib advisory (Dependabot #1)**: Blocked by Tauri/GTK constraints.
  - Current stack: `tauri 2.9.5` → `gtk 0.18.x` → `glib 0.18.x`.
  - Patched version requires `glib 0.20+`, which is not yet compatible with `gtk 0.18`.
  - **Action:** Revisit after Tauri/GTK upgrades to a `glib 0.20+` compatible release.

---

## Total Estimated Time

| Phase | Estimate |
|-------|----------|
| 0 | ~1 hour |
| 1 | ~3 hours |
| 2 | ~2 days |
| 3 | ~2 days |
| 4 | ~3 hours |
| 5 | ~2 days |
| 6 | ~2 days |
| 7 | ~3 hours |
| **Total** | **~9-10 days** |

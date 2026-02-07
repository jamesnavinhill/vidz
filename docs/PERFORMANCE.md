# Vidz Performance Guide

Last updated: 2026-02-07

## 1. First Principles

Large media libraries fail when decode, layout, and IO spikes happen at the same time.

Vidz performance strategy is to:

- Stream results early and continuously (scan and UI batches).
- Keep decode budget bounded by viewport and concurrency caps.
- Avoid unnecessary reactive and DOM churn in the grid.
- Move heavy media work to background queues.

## 2. Current Performance Mechanisms

### Scan and Ingest

- Batched DB upserts during scan (`upsert_videos_batch`).
- Batched `library:discovered` emission while scan runs.
- Throttled `process_pending_jobs` kicks during active scan for progressive enrichment.

### Grid and Playback

- Virtualized row rendering with precomputed row heights.
- Viewport-based autoplay gating.
- Concurrency cap (`maxConcurrentVideos`) without reducing supported feature surface.
- Thumbnail-first tiles.
- Video source attachment only when tile is actually eligible to play.
- CSS containment for grid/tile regions.

### Media Pipeline

- Metadata queue: 4 concurrent workers.
- Thumbnail queue: 2 concurrent workers.
- Process spawn on Windows uses no-console flag.

## 3. Tuning Knobs

- `density`: controls tile size/visible clip count.
- `maxConcurrentVideos`: decode pressure ceiling.
- Virtualizer overscan: currently fixed to `4` rows.
- Job queue semaphores: metadata `4`, thumbnails `2`.

## 4. What To Measure

- Time to first discovered clip after starting scan.
- Time to first thumbnail appearance.
- Time to first frame while scrolling.
- Dropped frames / stutter while rapid scrolling.
- CPU/GPU utilization during scan+scroll overlap.

## 5. Next Optimization Targets

- Priority thumbnailing for currently visible clips.
- Adaptive overscan based on scroll velocity.
- Incremental scan cursor (delta scan on startup).
- Smarter watcher debounce for burst file-copy operations.
- Optional preload policy adaptation based on autoplay and scroll state.

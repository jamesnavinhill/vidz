# Vidz Performance Guide

Last updated: 2026-02-08

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
- Incremental startup scan cursor to skip unchanged file ingest.
- Batch telemetry emission (`library:scan_batch`) for size/latency tracking.
- Throttled `process_pending_jobs` kicks during active scan for progressive enrichment.

### Grid and Playback

- Virtualized row rendering with precomputed row heights.
- Adaptive overscan based on measured scroll velocity.
- Viewport-based autoplay gating.
- Near-future prefetch window based on current scroll direction and speed.
- Concurrency cap (`maxConcurrentVideos`) without reducing supported feature surface.
- Idle decode warmup for near-viewport rows when scroll settles.
- Decode-heavy drop strategy (codec + resolution heuristic) under active load.
- Optional dense-layout video mount pooling budget.
- Thumbnail-first tiles.
- Video source attachment for playable and prefetch-window tiles.
- CSS containment for grid/tile regions.

### Media Pipeline

- Metadata queue: 4 concurrent workers.
- Thumbnail queue: 2 concurrent workers.
- Job prioritization from viewport-near IDs (`update_ui_activity` hints).
- UI-aware backpressure (thumbnail batch limits + dynamic parallelism under active scrolling).
- Dynamic thumbnail sizing/quality based on estimated tile width.
- Retry budget + categorized failure counters exposed via `library:job_telemetry`.
- Process spawn on Windows uses no-console flag.
- Metadata extraction records `codec_name` for decode scheduling decisions.

### Watcher Robustness

- Adaptive watcher debounce for burst copy/move activity.
- Saturation counters + telemetry event (`library:watcher_telemetry`).
- Saturation-triggered recovery reconciliation.
- Periodic reconciliation scan for watcher misses.

## 3. Tuning Knobs

- `density`: controls tile size/visible clip count.
- `maxConcurrentVideos`: decode pressure ceiling.
- Virtualizer overscan: adaptive (`4` to `12`) by scroll velocity.
- Job queue semaphores: metadata `4`, thumbnails `2`.
- Watcher debounce: adaptive (`~220ms` to `~1800ms`) by burst/queue pressure.

## 4. What To Measure

- Time to first discovered clip after starting scan.
- Time to first thumbnail appearance.
- Time to first frame while scrolling.
- Dropped frames / stutter while rapid scrolling.
- CPU/GPU utilization during scan+scroll overlap.
- Watcher queue saturation and reconciliation recovery counts.

## 5. Next Optimization Targets

- Priority thumbnailing for currently visible clips.
- Adaptive overscan based on scroll velocity.
- Incremental scan cursor (delta scan on startup).
- 10k+ and mixed-codec playback profiling to tune decode-heavy drop heuristics.
- CI/perf harness for repeatable scan+scroll+watcher burst scenarios.

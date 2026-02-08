# Optimization Roadmap Checklist

Last updated: 2026-02-08

## Phase 0 - Baseline and Instrumentation

- [ ] Capture baseline metrics (scan start -> first clip visible, first thumb, smooth-scroll FPS)
- [ ] Add repeatable perf test scenario (small, medium, 10k+ libraries)
- [ ] Log profile snapshots for CPU/GPU/IO under scan+scroll load

## Phase 1 - Scan and Ingest Pipeline

- [x] Emit discovered clips in batches during scan
- [x] Batch DB upserts in scan transaction chunks
- [x] Kick metadata/thumb job processing during active scan (throttled)
- [x] Add incremental scan cursor for startup delta scanning
- [x] Add batch-level telemetry events (batch size, latency)

## Phase 2 - Media Enrichment Throughput

- [x] Prioritize metadata/thumb jobs for viewport-near clips
- [x] Add queue backpressure policy aware of UI activity
- [x] Tune ffmpeg thumbnail quality/size by actual tile resolution
- [x] Add retry budget and categorized failure counters for media jobs

## Phase 3 - Grid Render and Scroll Smoothness

- [x] Replace per-tile visibility observers with viewport-range gating
- [x] Replace global per-tile play registry churn with deterministic autoplay allowance
- [x] Thumbnail-first tile rendering
- [x] Add CSS containment on grid/tile containers
- [x] Add adaptive overscan based on scroll velocity
- [x] Add scroll-state prefetch window for near-future rows

## Phase 4 - Playback Quality and Decode Scheduling

- [x] Keep configurable playback concurrency ceiling (`maxConcurrentVideos`)
- [x] Add decode warmup for near-viewport items when idle
- [x] Add drop strategy for decode-heavy codecs under load
- [x] Evaluate optional pooled video element strategy for very dense layouts

## Phase 5 - Watcher and Robustness

- [x] Implement adaptive watcher debounce for burst copy/move operations
- [x] Add watcher queue saturation counters and recovery behavior
- [x] Add periodic reconciliation scan for watcher misses

## Phase 6 - Validation and Release Hardening

- [ ] Run 10k+ manual smoke test and record quantitative outcomes
- [ ] Validate behavior on low-end and high-end Windows hardware profiles
- [ ] Lock release checklist in docs and CI gates

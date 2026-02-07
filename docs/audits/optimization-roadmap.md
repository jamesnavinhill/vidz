# Optimization Roadmap Checklist

Last updated: 2026-02-07

## Phase 0 - Baseline and Instrumentation

- [ ] Capture baseline metrics (scan start -> first clip visible, first thumb, smooth-scroll FPS)
- [ ] Add repeatable perf test scenario (small, medium, 10k+ libraries)
- [ ] Log profile snapshots for CPU/GPU/IO under scan+scroll load

## Phase 1 - Scan and Ingest Pipeline

- [x] Emit discovered clips in batches during scan
- [x] Batch DB upserts in scan transaction chunks
- [x] Kick metadata/thumb job processing during active scan (throttled)
- [ ] Add incremental scan cursor for startup delta scanning
- [ ] Add batch-level telemetry events (batch size, latency)

## Phase 2 - Media Enrichment Throughput

- [ ] Prioritize metadata/thumb jobs for viewport-near clips
- [ ] Add queue backpressure policy aware of UI activity
- [ ] Tune ffmpeg thumbnail quality/size by actual tile resolution
- [ ] Add retry budget and categorized failure counters for media jobs

## Phase 3 - Grid Render and Scroll Smoothness

- [x] Replace per-tile visibility observers with viewport-range gating
- [x] Replace global per-tile play registry churn with deterministic autoplay allowance
- [x] Thumbnail-first tile rendering
- [x] Add CSS containment on grid/tile containers
- [ ] Add adaptive overscan based on scroll velocity
- [ ] Add scroll-state prefetch window for near-future rows

## Phase 4 - Playback Quality and Decode Scheduling

- [x] Keep configurable playback concurrency ceiling (`maxConcurrentVideos`)
- [ ] Add decode warmup for near-viewport items when idle
- [ ] Add drop strategy for decode-heavy codecs under load
- [ ] Evaluate optional pooled video element strategy for very dense layouts

## Phase 5 - Watcher and Robustness

- [ ] Implement adaptive watcher debounce for burst copy/move operations
- [ ] Add watcher queue saturation counters and recovery behavior
- [ ] Add periodic reconciliation scan for watcher misses

## Phase 6 - Validation and Release Hardening

- [ ] Run 10k+ manual smoke test and record quantitative outcomes
- [ ] Validate behavior on low-end and high-end Windows hardware profiles
- [ ] Lock release checklist in docs and CI gates

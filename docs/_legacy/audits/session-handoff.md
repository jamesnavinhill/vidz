# Session Handoff (Agent Bootstrap)

Last updated: 2026-02-08

## Purpose

Use this document to start a new agent session with minimal context loss.

Primary workflow:

1. Paste this handoff into a new session.
2. Attach `docs/audits/optimization-roadmap.md`.
3. Give a task like: `Implement Phase 2`.

The agent must implement code, validate changes, and keep documentation current for the next handoff.

## Copy/Paste Prompt

```text
You are continuing work on the Vidz project.

Read and use these docs as source of truth:
- docs/ARCHITECTURE.md
- docs/OPERATIONS.md
- docs/PERFORMANCE.md
- docs/audits/optimization-roadmap.md
- docs/audits/session-handoff.md

Task:
Implement PHASE <X> from docs/audits/optimization-roadmap.md.

Execution rules:
- Preserve feature depth. Do not reduce supported concurrent playback behavior to hide performance issues.
- Prefer batching, backpressure, scheduling, prioritization, and incremental rendering/ingest.
- Keep scan and UI updates incremental for large libraries.
- Keep behavior coherent with focused-player and autoplay rules.

Required workflow:
1) Start by summarizing current phase status and selecting the exact checklist items you will implement now.
2) Implement production code changes (frontend/backend/docs as needed).
3) Run validation commands that are available in this environment:
   - pnpm exec tsc --noEmit
   - pnpm lint
   - pnpm test
   - cargo check (if cargo exists)
   - cargo test (if cargo exists)
4) Update documentation before finishing:
   - Mark completed items in docs/audits/optimization-roadmap.md
   - Update docs/audits/session-handoff.md “Current State Snapshot” and “Next Session Start Here”
   - Update docs/ARCHITECTURE.md, docs/OPERATIONS.md, docs/PERFORMANCE.md if behavior or operations changed
   - Update README.md and AGENTS.md if project workflow/structure/commands changed
5) End with a concise handoff summary that includes:
   - What was implemented
   - What remains in the phase
   - Validation results
   - Risks/known gaps
   - Exact next command for the next session (example: “Implement Phase 3 remaining items”)

Quality bar:
- Explain tradeoffs briefly and concretely.
- No placeholder-only planning. Ship implemented changes in the same session when feasible.
- If blocked, document blocker and provide best possible partial completion with clear next step.
```

## Current State Snapshot

Project status at latest handoff:

- Implemented in roadmap:
  - Phase 1: complete
  - Phase 2: complete
  - Phase 3: complete
  - Phase 4: complete
  - Phase 5: complete
- Not yet complete:
  - Phase 0, Phase 6

Recent architectural/performance changes already in code:

- Batched scan discovery emission during scan.
- Batched DB upserts for scan ingest.
- Incremental startup scan cursor and batch-level scan telemetry events.
- Throttled job kicks while scanning.
- Job queue priority and backpressure from live UI activity hints.
- Dynamic thumbnail sizing/quality from estimated tile width.
- Retry budget with categorized media failure counters and job telemetry events.
- Viewport-gated autoplay allowance and thumbnail-first tile rendering.
- Adaptive overscan and near-future prefetch window based on scroll state.
- CSS containment for grid/tile regions.
- Idle decode warmup for near-viewport rows.
- Decode-heavy load shedding during active scroll pressure.
- Optional dense-layout video mount pooling budget.
- Metadata extraction now persists `codec_name`.
- Watcher adaptive debounce, saturation telemetry/recovery, and periodic reconciliation scans.

## Next Session Start Here

Default recommendation:

1. Implement `Phase 0 - Baseline and Instrumentation`.
2. Then implement `Phase 6 - Validation and Release Hardening`.

Suggested first concrete target in next session:

- `Phase 0`: capture and log baseline metrics for scan->first clip, first thumb, and scroll smoothness.

## End-of-Session Update Template

Use this block at the end of each session and update this file in-place.

```text
Session date: YYYY-MM-DD
Implemented:
- ...
Roadmap updates:
- [x] ...
- [ ] ...
Validation:
- pnpm exec tsc --noEmit: pass/fail
- pnpm lint: pass/fail
- pnpm test: pass/fail
- cargo check: pass/fail/not available
- cargo test: pass/fail/not available
Known risks/gaps:
- ...
Next session command:
- Implement Phase <X> remaining items: ...
```

## Latest Session Update

```text
Session date: 2026-02-08
Implemented:
- Phase 4 remaining items:
  - Idle decode warmup for near-viewport tiles.
  - Decode-heavy drop strategy under load.
  - Optional dense-layout video mount pooling evaluation path.
- Phase 5 items:
  - Adaptive watcher debounce for burst copy/move operations.
  - Watcher queue saturation counters + `library:watcher_telemetry`.
  - Saturation-triggered recovery + periodic watcher reconciliation scan.
- Metadata pipeline now stores `codec_name` for decode scheduling heuristics.
Roadmap updates:
- [x] Phase 1 complete
- [x] Phase 2 complete
- [x] Phase 3 complete
- [x] Phase 4 complete
- [x] Phase 5 complete
Validation:
- pnpm exec tsc --noEmit: pass
- pnpm lint: pass
- pnpm test: pass
- cargo check: not available (cargo not installed in this environment)
- cargo test: not available (cargo not installed in this environment)
Known risks/gaps:
- Phase 0 baseline instrumentation still incomplete.
- Phase 6 validation/hardware-profile checks remain pending.
- Rust compilation could not be executed in this environment; backend changes should be checked on a machine with Rust installed.
Next session command:
- Implement Phase 0 baseline instrumentation and metric capture workflow.
```

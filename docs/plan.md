# Vidz - Plan (Finalized, No Ambiguity)

> **Date:** January 30, 2026  
> **Status:** Finalized Plan (Implementation-Ready)

This document captures the finalized, unambiguous product and technical choices for **Vidz**, a local video viewer. It is the single source of truth for scope and behavior.

---

## Goal

Build a **clean, performant, local video viewer** that can smoothly display and play **10,000+ short clips** in a continuous, minimal grid. The app should be **frictionless**, **fast**, and **visually clean**, with no overlays and no wasted space.

---

## Finalized Technology Choices

- **App Architecture:** **Tauri 2.x**
  - Windows-first desktop app with native file system access and Rust backend performance.
- **Frontend Framework:** **SolidJS + Vite**
  - Fine-grained reactivity and high performance for large lists.
- **Virtualization:** **TanStack Virtual**
  - Grid virtualization for 10k+ videos with smooth scrolling.

---

## Content & Media Decisions (Unambiguous)

- **Thumbnail Type:** Single frame thumbnail
  - **Frame position:** can be **first frame** (clean starting frame is acceptable).
- **Thumbnail Source:** All scenes from the videos are acceptable.
- **Playback on Hover/Scroll:** Video playback happens in-grid unless autoplay is toggled off.
- **Playback Behavior:**
  - When a single video is selected for a dedicated player view, **all other videos stop**.
- **Video Length Expectation:** Clips are short (typically 6–10 seconds).
- **Formats:** Support all common formats that “make sense” (mp4, mkv, webm, avi, mov, etc.).

---

## UI/UX Decisions

- **Grid Layout:** Responsive grid with adjustable density.
- **No text overlays** on videos (no labels, no metadata on thumbnails).
- **Minimal spacing/padding** between thumbnails; no wasted space.
- **Scrolling:** Continuous infinite scroll.
- **Autoplay Toggle:**
  - Default behavior: **Videos play while scrolling**.
  - If autoplay is toggled off, videos do not auto-play.

---

## Sorting & Filtering (Finalized List)

Only the following options are needed:

**Sorting:**
- File size
- Resolution
- Aspect ratio
- Duration
- Folder (group/sort by folder path)
- Favorites

**Filtering:**
- Folder
- Favorites

**Explicitly excluded:** No codec/bitrate/fps/technical filters.

---

## File Watching & Auto-Import

- The app **auto-adds new files** using a background watcher on watched directories.
- New videos appear in the grid without needing a manual refresh.

---

## Playback Modes

1. **Grid mode (default):**
   - All visible videos play simultaneously while scrolling.
2. **Focused player mode:**
   - Selecting a video opens a dedicated larger player.
   - **All other videos pause** while focused player is active.

---

## Performance Targets

- Smooth infinite scroll with **10,000+ videos**.
- Responsive interactions with no lag on Windows hardware similar to:
  - RTX 2080, 32GB RAM, multi-terabyte local storage.

---

## Non-Goals / Out of Scope

- No export/import workflows.
- No heavy metadata overlays or text on thumbnails.
- No advanced technical filters (codec, bitrate, fps).
- No requirement for cross-platform support in v1 (Windows only is acceptable).

---

## Implementation Notes (Guidance Only)

- Use Rust backend (Tauri) for directory scanning and file watching.
- Cache thumbnails and metadata for fast startup.
- Keep the frontend component structure clean and modular.

---

## Success Criteria

- App displays a large local library instantly with a smooth, continuous grid.
- Sorting and filtering work **only** on the finalized list above.
- Autoplay behavior matches the exact rules:
  - **Videos play while scrolling unless autoplay is off.**
  - **Focused player stops all other videos.**
- No ambiguous behavior remains in the plan.
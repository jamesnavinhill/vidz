# Vidz

A clean, performant local video viewer that can smoothly display and play 10,000+ short clips in a continuous, minimal grid.

## Tech Stack

- **App Architecture:** Tauri 2.x (Windows-first desktop app)
- **Frontend:** SolidJS + Vite
- **Virtualization:** TanStack Virtual (grid virtualization for 10k+ videos)
- **Database:** SQLite via rusqlite
- **Media Processing:** ffprobe (metadata) + ffmpeg (thumbnails) - bundled

## Data Model

```typescript
interface VideoItem {
  id: string;           // SHA256 hash of canonical path (first 32 chars)
  path: string;         // Full file path
  folder: string;       // Parent directory
  size_bytes: number;   // File size
  mtime: number;        // Modified time (Unix timestamp)
  duration_ms: number;  // Video duration in milliseconds
  width: number;        // Video width
  height: number;       // Video height
  aspect_ratio: number; // width / height
  favorite: boolean;    // User favorite flag
  thumb_path: string;   // Path to cached thumbnail
}
```

## Features

- **Grid Layout:** Responsive grid with adjustable density
- **Playback on Hover:** Videos play in-grid when hovered (unless autoplay is off)
- **Focused Player:** Click to open dedicated player view (pauses all other videos)
- **Sorting:** File size, resolution, aspect ratio, duration, folder, favorites
- **Filtering:** By folder, favorites only
- **Auto-import:** Background file watcher for watched directories

## Development

### Prerequisites

- Node.js 18+
- pnpm
- Rust 1.70+
- ffmpeg & ffprobe (in PATH or bundled in `app/src-tauri/bin/`)

### Setup

```bash
cd app
pnpm install
pnpm tauri dev
```

### Tooling

```bash
cd app
pnpm lint
pnpm format
pnpm format:check
pnpm test
```

```bash
cd app/src-tauri
cargo fmt
cargo clippy
```

### Build

```bash
cd app
pnpm tauri build
```

## Project Structure

```
.
├── app/                     # Tauri app (SolidJS + Rust)
│   ├── src/                 # Frontend (SolidJS)
│   │   ├── components/
│   │   │   ├── VideoGrid.tsx   # Virtualized grid
│   │   │   ├── VideoTile.tsx   # Individual video tile
│   │   │   ├── FocusedPlayer.tsx
│   │   │   └── Toolbar.tsx     # Sort/filter controls
│   │   ├── store.ts            # App state
│   │   └── types.ts            # TypeScript types
│   ├── src-tauri/            # Backend (Rust)
│   │   ├── src/
│   │   │   ├── commands/       # Tauri commands
│   │   │   ├── db/             # SQLite database
│   │   │   ├── scanner/        # File scanning & media processing
│   │   │   ├── models.rs       # Data models
│   │   │   └── lib.rs          # App entry
│   │   └── bin/                # Bundled ffmpeg/ffprobe
│   └── README.md               # (Deprecated) Old location
├── docs/
│   ├── plan.md                 # Product spec
│   └── audit-report.md
└── README.md                   # Project overview (this file)
```
# Vidz - Agent Instructions

## Project Structure
- `/` - Tauri 2.x application (SolidJS frontend + Rust backend)
- `docs/` - Documentation and planning

## Commands

### Frontend (from repo root)
```bash
pnpm install          # Install dependencies
pnpm dev              # Start Vite dev server
pnpm build            # Build frontend
pnpm exec tsc --noEmit  # TypeScript check
```

### Backend (from src-tauri/)
```bash
cargo check           # Type check Rust code
cargo build           # Build Rust backend
cargo clippy          # Lint Rust code
```

### Full App (from repo root)
```bash
pnpm tauri dev        # Run full app in dev mode
pnpm tauri build      # Build production app
```

## Requirements
- Rust 1.70+
- Node.js 18+
- pnpm
- ffmpeg & ffprobe (in PATH or bundled in src-tauri/bin/)

## Code Style
- Frontend: TypeScript + SolidJS, minimal comments
- Backend: Rust 2021 edition, use thiserror for errors
- No inline comments unless complex logic requires explanation

# AGENTS.md

Tauri 2 desktop app — duplicate image detector (SHA-256 hash + EXIF metadata). Rust backend, React/TypeScript frontend.

## Commands

- **Dev server (frontend only):** `npm run dev` — Vite on port 1420
- **Full app dev:** `npm run tauri dev` — compiles Rust, opens window with hot reload
- **Build frontend:** `npm run build` — `tsc && vite build`
- **Build release:** `npx tauri build` — produces installers in `src-tauri/target/release/bundle/`
- **Rust tests:** `cd src-tauri && cargo test`

No lint, typecheck, or test scripts in `package.json`. Frontend typecheck is just `npx tsc --noEmit`.

## Architecture

- **Frontend** (`src/`): React + TypeScript, uses `@tauri-apps/api` and `@tauri-apps/plugin-dialog`
  - `App.tsx` — main component, handles folder picker, scan invocation, progress events
  - `components/GroupCard.tsx` — duplicate group display + delete actions
  - `components/ImageViewer.tsx` — full-size image preview with keyboard navigation (Esc/←/→)
  - `components/ConfirmDialog.tsx` — custom delete confirmation modal (kept vs deleted files)
  - `types.ts` — `ImageInfo`, `DupGroup`, `ScanProgress`, `ScanResult` interfaces
- **Backend** (`src-tauri/src/`): Rust, Tauri 2 IPC commands
  - `commands.rs` — Tauri command handlers (`scan_folder`, `preview`, `delete_to_trash`, `delete_permanent`)
  - `scanner.rs` — recursive file scanning
  - `hasher.rs` — SHA-256 hashing
  - `metadata.rs` — EXIF extraction + shared duplicate-signature function
  - `matcher.rs` — union-find grouping logic
  - `thumbnails.rs` — thumbnail generation with on-disk cache (`/tmp/imagen-duplicada-thumbs`)
  - `reader.rs` — mmap-based file reader wrapper
  - `lib.rs` — module declarations, `ImageInfo`/`DupGroup`/`ScanResult` structs, app builder
- **IPC contract**: Frontend calls `invoke<T>("command_name", { args })`. Types must stay in sync between `lib.rs` structs and `src/types.ts`.
- **Tauri config**: `src-tauri/tauri.conf.json` — dev server at `localhost:1420`, asset protocol enabled for local file access.

## Platform Notes

- Linux requires: `sudo apt install -y libwebkit2gtk-4.1-dev libssl-dev librsvg2-dev patchelf build-essential`
- Windows needs WebView2 Runtime; macOS needs Xcode CLI tools
- Rust toolchain required for all platforms (install via rustup)

## Conventions

- App language is Spanish (UI strings, commit messages, variable names)
- Supported image formats: JPEG, PNG, WebP, GIF, TIFF, BMP
- No monorepo — single package
- **Idioma del asistente**: Responder y razonar siempre en español

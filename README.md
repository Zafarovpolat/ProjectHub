# ProjectHub

Always-on-top project dock & orchestrator for Windows. Designed for the
"4 Chrome windows + 1 Telegram + 1 IDE per project" pain — one keystroke
collapses every other window so Alt-Tab only shows the project you're
actually working on.

> Status: **v0.1 MVP — Windows only.** Vibe-coded. Pragmatic, not
> production-grade.

## What it does

- Pins a thin 240×720 dock to the right edge of the screen, always on top,
  transparent with a blurred background.
- Lists your projects. Each project is a named bundle of external windows
  (Chrome instances, IDE, Telegram, etc.) the hub remembers by exe + title
  pattern.
- Clicking a project enters **focus mode (hard switch)**:
  - Every other top-level window is minimised.
  - Every window stored in the project is restored and raised.
  - Alt-Tab now only shows the active project.
- Global hotkeys: `Ctrl+Alt+1..9` switch projects, `Ctrl+Alt+Space`
  toggles the dock.
- System tray icon with Show / Hide / Quit.
- All actions are appended to a JSON-Lines event log
  (`%APPDATA%\ProjectHub\events.jsonl`) so a future AI assistant can read
  your work history.

## What it intentionally does **not** do (yet)

- Embed Chrome / IDE / Telegram (orchestrates external apps only).
- Auto-launch missing windows (v0.2).
- Tile or position windows (v0.2).
- Telegram unread badges or full TG client (v0.3+).
- Cross-platform support — `windows` crate APIs only.

## Tech stack

- [Tauri 2](https://v2.tauri.app) — Rust desktop runtime.
- Frontend: SvelteKit (SPA mode) + Svelte 5 runes + Tailwind CSS 4 +
  lucide icons.
- Backend: Rust + `windows` 0.58 (Win32 FFI).
- Plugins: `tauri-plugin-global-shortcut`, `tauri-plugin-store`,
  `tauri-plugin-positioner`, `tauri-plugin-opener`.
- Storage: JSON for projects (`projects.json`), JSON-Lines for events
  (`events.jsonl`), both under `%APPDATA%\ProjectHub\`.

## Project layout

```
projecthub/
├── src/                          # Svelte frontend
│   ├── app.css                   # Tailwind + design tokens
│   ├── app.html
│   ├── lib/
│   │   ├── api.ts                # Typed wrappers around Tauri commands
│   │   ├── store.svelte.ts       # Svelte 5 runes-based global store
│   │   ├── types.ts              # TS mirrors of Rust types
│   │   └── components/
│   │       ├── Dock.svelte
│   │       ├── ProjectCard.svelte
│   │       ├── AddProjectDialog.svelte
│   │       └── WindowPicker.svelte
│   └── routes/                   # SvelteKit pages (single page in SPA mode)
└── src-tauri/                    # Rust backend
    ├── Cargo.toml
    ├── tauri.conf.json
    ├── capabilities/default.json # Tauri 2 capabilities
    └── src/
        ├── lib.rs                # Plugin/tray/hotkey wiring
        ├── commands.rs           # #[tauri::command] handlers
        ├── window_manager.rs     # Win32 EnumWindows / focus / minimise
        ├── project.rs            # Project / WindowRef data model
        ├── store.rs              # JSON persistence (atomic writes)
        └── event_log.rs          # JSONL append-only audit log
```

## Build & run

Prerequisites:

- Node 20+ and `pnpm`.
- Rust stable (`rustup default stable`).
- On Windows: MSVC build tools + Windows SDK.
  On Linux (for `cargo check` only): the usual Tauri deps —
  `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libsoup-3.0-dev`,
  `librsvg2-dev`, `libayatana-appindicator3-dev`, `pkg-config`.

Install JS deps and verify the frontend:

```bash
pnpm install
pnpm check     # typecheck + svelte-check (must be 0 errors)
pnpm build     # produces ./build with the static SPA
```

Verify the Rust backend compiles (note: window enumeration only works on
Windows):

```bash
cd src-tauri
cargo check
```

Run in development:

```bash
pnpm tauri dev
```

Build the installer (MSI on Windows):

```bash
pnpm tauri build
```

The MSI ends up under `src-tauri/target/release/bundle/msi/`.

## How a project is created

1. Arrange the windows you want as a project (e.g. open the right Chrome
   profile, open Telegram on the right chat, open the IDE on the right
   folder).
2. Click `+` in the dock → name the project → tick the windows in the
   picker → save.
3. The hub stores each window's `(exe_path, title_pattern, class_name)`.
   On future activations it re-discovers the live HWNDs via `EnumWindows`
   and matches by exe + title substring.

## Event log

Every action (project created/activated/deleted, hotkey fired, dock
toggled, etc.) is appended as a JSON line to
`%APPDATA%\ProjectHub\events.jsonl`. The file rotates at 10 MB. Format:

```json
{"id":"…","timestamp":"2026-05-10T18:41:00Z","type":"project_activated","name":"Devin Task","from":null,"duration_in_prev_ms":null,"windows_focused":4,"windows_minimized":7,"project_id":"…"}
```

This is the foundation for the v0.2+ "ask the AI what I did" feature.

## Hotkeys

| Combo            | Action                                |
|------------------|----------------------------------------|
| `Ctrl+Alt+Space` | Toggle dock visibility                 |
| `Ctrl+Alt+1`–`9` | Activate project bound to that index   |

The hotkey index is assigned automatically when a project is created.
You can re-bind it later by editing the project (UI for this is v0.2).

## License

MIT

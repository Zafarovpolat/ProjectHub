# ProjectHub roadmap

Status legend: `[done]` shipped, `[priority]` planned next, `[idea]` open
proposal — discuss before implementing.

## v0.1 (shipped)

- Always-on-top 240×720 dock pinned to the right edge.
- Manual project bundle: name + set of windows matched by `(exe_path,
  title_pattern, class_name)`.
- "Hard switch" focus mode: minimise everything else, restore + raise the
  project's windows, focus the primary one.
- Global hotkeys — `Ctrl+Alt+1..9` (per project) and `Ctrl+Alt+Space`
  (toggle dock) — are now defaults; both are user-overridable per
  project + globally for the dock toggle.
- System tray icon + JSON-Lines event log.
- Manage projects dialog: reorder, assign hotkeys, delete.
- Add-project dialog with a window picker (separate Tauri window).
- Auto-remove closed windows — background pruner every 5 s, 15 s grace
  period; "closed" badge shown in Manage projects while a window is in
  grace.
- **A. Auto-rebind on reopen.** When a window with the same
  `(exe_path, title_pattern)` as a previously-removed ref re-opens
  (e.g. Chrome restart), it is automatically reattached to its old
  project. Dropped fingerprints are remembered for 24 h and GC'd by the
  same pruner. *(this change)*
- **B. Add / remove windows on an existing project.** Manage projects
  dialog now has a `+ Add windows` affordance per project that opens
  the shared window picker, and a hover-revealed `×` icon on each
  window row to drop it. Removing a window also drops any matching
  dropped fingerprint so it doesn't re-attach itself on the next tick.
  *(this change)*
- **G. Per-project last-activity timestamp in the dock.** Cards show
  "… · 5m ago" and refresh on a 30 s reactive timer so the relative
  string doesn't go stale. *(this change)*
- **Custom hotkey infrastructure.** Each project has both a numeric
  slot (`hotkey_index`) and an arbitrary user-defined combo
  (`hotkey_combo`, e.g. `Ctrl+Shift+KeyM`). When set, the combo
  overrides the slot. The dock-toggle combo is configurable via a
  `preferences.json` next to `projects.json`
  (`set_dock_toggle_hotkey` command). All shortcuts go through a
  single `ShortcutRouter` that's rebuilt on every project /
  preferences mutation — no restart needed. *(this change)*

## Open ideas (still ideas — discuss before implementing)

- **C. Quick-switch overlay (`Alt+\``).** Center-screen palette of project
  cards with previews, overlaying everything. More discoverable than
  the digit hotkeys for users who haven't memorised them yet.

- **D. Drag-and-drop windows between projects in Manage.** Pick up a
  window row in one project and drop it into another. Useful when a tab
  was added to the wrong project. Backend just needs to support
  "move-window" (today only "replace project windows" via
  `update_project.window_hwnds`).

- **E. Auto-attach rules.** Power-user feature: rule like
  *"when a window with `exe = figma.exe` and title matching `*Figma*`
  opens, add it to project `Design`"*. Implementation needs persistent
  rule storage + the pruner to consult rules on each new live window
  enumeration.

- **F. Dock auto-hide.** Slide the dock 90 % off-screen at rest and
  expand on hover. Smaller permanent footprint; minor risk of being
  finicky on multi-monitor.

- **H. Event-log viewer in UI.** Button in Manage projects → opens a
  read-only window showing the last 200 events from `events.jsonl`.
  Trivial: backend `read_recent_events` command already exists.
  Helpful for debugging "why did focus fail on this Chrome window?".

## Out-of-scope for the dock (kept here for memory)

- Embedding Chrome/IDE/Telegram — orchestrate external apps only.
- Cross-platform support — Windows only by design.
- Telegram unread-badge / TG client features — v0.3+.

## Implementation notes

### Auto-remove + auto-rebind

Pruner runs in a dedicated thread, ticks every
`pruner::TICK_INTERVAL` (5 s). Each tick does two passes:

**Pass 1** — for each `WindowRef` in every project tries `match_window`
against the live enumeration:

- Match → reset `missed_ticks=0`, `live=true`.
- Miss → `missed_ticks += 1`, `live=false`. Once `missed_ticks >=
  pruner::GRACE_PERIOD_TICKS` (3, ≈ 15 s) the ref is dropped and a
  `WindowAutoRemoved` event is appended to `events.jsonl`. Its
  `(title_snapshot, title_pattern, exe_path, class_name)` is pushed
  into `Project.dropped_fingerprints` with a timestamp.

The 15 s grace exists because Chrome/Telegram routinely retitle windows
during navigation; without it, a single flaky tick would yank a window
from a project mid-use.

**Pass 2** — garbage-collects dropped fingerprints older than
`DROPPED_FINGERPRINT_TTL` (24 h) and, for every survivor, looks for a
live window matching `(exe_path, title_pattern)` that isn't already
bound to some project. A match becomes a fresh `WindowRef` (preserving
the user's `title_pattern` so the bound ref keeps matching across
future title changes) and we emit `WindowAutoRebound` plus
`project:changed`.

Disk is only touched when a ref is actually removed, rebound, or a
fingerprint is GC'd. Transient `live`/`missed_ticks` updates stay in
memory; the dock learns about them via `project:changed` events
emitted whenever a window's live state flips.

### Custom hotkeys

Global shortcuts go through a single in-memory `ShortcutRouter`:

```rust
pub struct ShortcutRouter {
    pub by_combo: HashMap<String, RouterAction>,
}
pub enum RouterAction { ToggleDock, ActivateProject(Uuid) }
```

`hotkeys::reregister(app)`:

1. Unregisters every previously-registered shortcut.
2. Resolves the effective dock-toggle combo (user override else
   `DEFAULT_DOCK_TOGGLE_COMBO = "Ctrl+Alt+Space"`).
3. For each project resolves its effective combo:
   `Project.hotkey_combo` if set, else `Ctrl+Alt+Digit<hotkey_index>`
   (legacy 1-9 slot), else no binding.
4. Parses each combo via `Shortcut::from_str`, registers it, and
   inserts a `(canonical_combo → action)` entry in the router.

The global-shortcut handler in `lib.rs` does **no** matching of its own:
it canonicalises the incoming `Shortcut` to the same string format the
router uses and looks the action up. This means a brand-new combo
set in the UI works instantly — every `create_project` /
`update_project` / `delete_project` / `set_dock_toggle_hotkey` call ends
with `hotkeys::reregister(app)`.

The dock-toggle override is persisted in `preferences.json` next to
`projects.json` (`PreferencesStore`). Per-project combos live on the
project record itself, so no schema migration is needed beyond the new
optional field.

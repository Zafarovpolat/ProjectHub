# ProjectHub roadmap

Status legend: `[done]` shipped, `[priority]` planned next, `[idea]` open
proposal — discuss before implementing.

## v0.1 (shipped)

- Always-on-top 240×720 dock pinned to the right edge.
- Manual project bundle: name + set of windows matched by `(exe_path,
  title_pattern, class_name)`.
- "Hard switch" focus mode: minimise everything else, restore + raise the
  project's windows, focus the primary one.
- Global hotkeys: `Ctrl+Alt+1..9` (per project) and `Ctrl+Alt+Space`
  (toggle dock).
- System tray icon + JSON-Lines event log.
- Manage projects dialog: reorder, assign hotkeys, delete.
- Add-project dialog with a window picker (separate Tauri window).
- **Auto-remove closed windows** — background pruner every 5 s, 15 s grace
  period; "closed" badge shown in Manage projects while a window is in
  grace. *(this change)*

## Priority next (selected by user)

- **A. Auto-rebind on reopen.** When a window matching `(exe_path,
  title_pattern)` reappears (e.g. Chrome restarted), automatically attach
  it back to the project whose window it used to be. The pruner already
  re-binds live HWNDs; this would extend that to refs that were
  auto-removed by remembering the dropped fingerprint for some window of
  time (e.g. last 24 h, capped) and reattaching when a matching window
  shows up. Without this, after rebooting the PC the project only sees
  whatever happens to be open at boot and the user has to re-add windows
  manually.

- **B. Add windows to an existing project.** Today the only way to grow a
  project is to delete + recreate it. Add a `+` button in Manage projects
  next to each project's window list that opens the existing window
  picker, scoped to that project. Also expose a "Remove window" affordance
  on each row in the expanded list. This is the only critical CRUD gap
  left in v0.1.

- **G. Per-project last-activity timestamp in the dock.** Show
  "DT · 2m ago" next to the active card. The data (`last_activated_at`)
  is already plumbed through `ProjectView`; only the dock display needs
  it surfaced. Tiny change, big perceived freshness.

## Open ideas (discuss before implementing)

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

### Auto-remove (this PR)

Pruner runs in a dedicated thread, ticks every
`pruner::TICK_INTERVAL` (5 s). For each `WindowRef` in every project it
tries `match_window` against the live enumeration:

- Match → reset `missed_ticks=0`, `live=true`.
- Miss → `missed_ticks += 1`, `live=false`. Once `missed_ticks >=
  pruner::GRACE_PERIOD_TICKS` (3, ≈ 15 s) the ref is dropped and a
  `WindowAutoRemoved` event is appended to `events.jsonl`. The 15 s
  grace exists because Chrome/Telegram routinely retitle windows during
  navigation; without it, a single flaky tick would yank a window from
  a project mid-use.

Disk is only touched when a ref is actually removed. Transient
`live`/`missed_ticks` updates stay in memory; the dock learns about them
via `project:changed` events emitted whenever a window's live state
flips.

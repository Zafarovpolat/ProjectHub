//! Background pruner — keeps each project's window list in sync with the
//! actual set of open top-level windows.
//!
//! Every `TICK_INTERVAL` we enumerate live windows and, for each
//! `WindowRef` in every project, try to re-bind it via
//! [`window_manager::match_window`].
//!
//! * If we find a match → reset `missed_ticks` to 0, mark `live=true`,
//!   record the HWND.
//! * If we don't → increment `missed_ticks`, mark `live=false`. Once
//!   `missed_ticks` reaches [`GRACE_PERIOD_TICKS`] the ref is removed
//!   from the project (so the dock no longer surfaces it) and a
//!   `WindowAutoRemoved` event is appended to the log.
//!
//! The grace period exists because Chrome/Telegram/etc. routinely retitle
//! their windows during navigation or chat switches. Without it, a single
//! flaky tick would yank a window out of a project mid-use.

use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::commands::AppState;
use crate::event_log::EventKind;
use crate::window_manager;

/// How often the pruner wakes up and re-binds windows.
pub const TICK_INTERVAL: Duration = Duration::from_secs(5);

/// Number of consecutive misses before a window ref is auto-removed.
/// 3 ticks × 5 s = 15 s grace period.
pub const GRACE_PERIOD_TICKS: u8 = 3;

/// Spawn the pruner thread. The thread loops forever; the OS reaps it
/// when the process exits.
pub fn spawn(app: AppHandle, state: Arc<AppState>) {
    std::thread::Builder::new()
        .name("projecthub-pruner".to_string())
        .spawn(move || run(app, state))
        .expect("spawn pruner thread");
}

fn run(app: AppHandle, state: Arc<AppState>) {
    loop {
        std::thread::sleep(TICK_INTERVAL);
        if let Err(err) = tick(&app, &state) {
            tracing::warn!(?err, "pruner tick failed");
        }
    }
}

/// What changed in a single pruner tick. Used to decide whether to emit
/// `project:changed` and to append removal events without holding the
/// store lock.
#[derive(Default)]
struct TickOutcome {
    /// Window refs auto-removed this tick. `(project_id, project_name,
    /// window_title, missed_ticks)`.
    removed: Vec<(Uuid, String, String, u8)>,
    /// Whether any window's `live` state flipped this tick. When `true`
    /// the dock needs to refresh so badges update.
    any_live_state_changed: bool,
}

fn tick(app: &AppHandle, state: &AppState) -> anyhow::Result<()> {
    let live = window_manager::enumerate_windows(Some(state.self_pid));

    let outcome: TickOutcome = state.store.with_projects_mut(|projects| {
        let mut outcome = TickOutcome::default();

        for project in projects.iter_mut() {
            let original_len = project.windows.len();
            let mut survivors = Vec::with_capacity(project.windows.len());
            let drained: Vec<_> = project.windows.drain(..).collect();

            for mut w in drained {
                match window_manager::match_window(&w, &live) {
                    Some(found) => {
                        let was_dead = !w.live || w.missed_ticks > 0;
                        w.missed_ticks = 0;
                        w.live = true;
                        w.last_seen_hwnd = Some(found.hwnd);
                        if was_dead {
                            outcome.any_live_state_changed = true;
                        }
                        survivors.push(w);
                    }
                    None => {
                        let was_live = w.live;
                        w.missed_ticks = w.missed_ticks.saturating_add(1);
                        w.live = false;
                        w.last_seen_hwnd = None;

                        if w.missed_ticks >= GRACE_PERIOD_TICKS {
                            outcome.removed.push((
                                project.id,
                                project.name.clone(),
                                w.title_snapshot.clone(),
                                w.missed_ticks,
                            ));
                            // Don't push into survivors — drop the ref.
                        } else {
                            if was_live {
                                outcome.any_live_state_changed = true;
                            }
                            survivors.push(w);
                        }
                    }
                }
            }

            project.windows = survivors;
            if project.windows.len() != original_len {
                project.updated_at = Utc::now();
            }
        }

        // Persist to disk only when a window was actually removed —
        // transient `live`/`missed_ticks` updates don't need to hit the
        // filesystem every 5 s.
        let dirty = !outcome.removed.is_empty();
        (outcome, dirty)
    })?;

    // Append events outside the store lock so a slow disk doesn't block
    // future ticks.
    for (project_id, project_name, title, missed) in &outcome.removed {
        state.events.append(
            EventKind::WindowAutoRemoved {
                project_name: project_name.clone(),
                title: title.clone(),
                missed_ticks: *missed,
            },
            Some(*project_id),
        );
    }

    // Tell the frontend to refresh if anything observable changed: either
    // a ref was removed, or at least one window's `live` flipped.
    if !outcome.removed.is_empty() || outcome.any_live_state_changed {
        let _ = app.emit("project:changed", ());
    }

    Ok(())
}

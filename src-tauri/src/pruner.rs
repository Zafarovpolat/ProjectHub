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
//!   from the project (so the dock no longer surfaces it). Its
//!   fingerprint goes into `Project.dropped_fingerprints` so we can
//!   reattach the window automatically when it reopens later
//!   (see `attempt_rebinds` below).
//!
//! The grace period exists because Chrome/Telegram/etc. routinely retitle
//! their windows during navigation or chat switches. Without it, a single
//! flaky tick would yank a window out of a project mid-use.

use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;
use tauri::{AppHandle, Emitter};
use uuid::Uuid;

use crate::commands::AppState;
use crate::event_log::EventKind;
use crate::project::{DroppedFingerprint, WindowRef, DROPPED_FINGERPRINT_TTL};
use crate::window_manager::{self, EnumeratedWindow};

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
/// `project:changed` and to append events without holding the store lock.
#[derive(Default)]
struct TickOutcome {
    /// Window refs auto-removed this tick. `(project_id, project_name,
    /// window_title, missed_ticks)`.
    removed: Vec<(Uuid, String, String, u8)>,
    /// Window refs auto-attached from a previously dropped fingerprint
    /// (i.e. the window reopened). `(project_id, project_name, title)`.
    rebound: Vec<(Uuid, String, String)>,
    /// Whether any window's `live` state flipped this tick. When `true`
    /// the dock needs to refresh so badges update.
    any_live_state_changed: bool,
}

fn tick(app: &AppHandle, state: &AppState) -> anyhow::Result<()> {
    let live = window_manager::enumerate_windows(Some(state.self_pid));
    let now = Utc::now();

    let outcome: TickOutcome = state.store.with_projects_mut(|projects| {
        let mut outcome = TickOutcome::default();

        // ---- Pass 1: rebind / mark missing / auto-remove --------------
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
                            project.dropped_fingerprints.push(DroppedFingerprint {
                                title_snapshot: w.title_snapshot.clone(),
                                title_pattern: w.title_pattern.clone(),
                                exe_path: w.exe_path.clone(),
                                class_name: w.class_name.clone(),
                                dropped_at: now,
                            });
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
                project.updated_at = now;
            }
        }

        // ---- Pass 2: GC old fingerprints + auto-rebind on reopen ------
        // Build the set of HWNDs that are already bound to a project so
        // we don't double-attach.
        let mut bound_hwnds: HashSet<isize> = HashSet::new();
        for p in projects.iter() {
            for w in &p.windows {
                if let Some(h) = w.last_seen_hwnd {
                    bound_hwnds.insert(h);
                }
            }
        }

        for project in projects.iter_mut() {
            // Drop fingerprints older than TTL.
            let original_fp_len = project.dropped_fingerprints.len();
            project
                .dropped_fingerprints
                .retain(|fp| now - fp.dropped_at < DROPPED_FINGERPRINT_TTL);

            // Try to reattach any fingerprint to a free live window.
            let mut kept: Vec<DroppedFingerprint> =
                Vec::with_capacity(project.dropped_fingerprints.len());
            for fp in std::mem::take(&mut project.dropped_fingerprints) {
                if let Some(found) = find_match(&fp, &live, &bound_hwnds) {
                    let class = if found.class_name.is_empty() {
                        None
                    } else {
                        Some(found.class_name.clone())
                    };
                    let mut w = WindowRef::new(
                        found.title.clone(),
                        found.exe_path.clone(),
                        class,
                        found.hwnd,
                    );
                    // Preserve the user's pattern so future re-attach
                    // keeps working even if the new title differs.
                    w.title_pattern = fp.title_pattern.clone();
                    bound_hwnds.insert(found.hwnd);
                    outcome.rebound.push((
                        project.id,
                        project.name.clone(),
                        found.title.clone(),
                    ));
                    outcome.any_live_state_changed = true;
                    project.windows.push(w);
                } else {
                    kept.push(fp);
                }
            }
            project.dropped_fingerprints = kept;
            if project.dropped_fingerprints.len() != original_fp_len {
                project.updated_at = now;
            }
        }

        // Persist to disk when refs were removed or rebound, or when a
        // fingerprint was GC'd. Transient `live`/`missed_ticks` updates
        // don't need to hit the filesystem.
        let dirty = !outcome.removed.is_empty() || !outcome.rebound.is_empty();
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
    for (project_id, project_name, title) in &outcome.rebound {
        state.events.append(
            EventKind::WindowAutoRebound {
                project_name: project_name.clone(),
                title: title.clone(),
            },
            Some(*project_id),
        );
    }

    // Tell the frontend to refresh if anything observable changed.
    let any_change = !outcome.removed.is_empty()
        || !outcome.rebound.is_empty()
        || outcome.any_live_state_changed;
    if any_change {
        let _ = app.emit("project:changed", ());
    }

    Ok(())
}

/// Find a live window whose `(exe_path, title_pattern)` (and optionally
/// `class_name`) matches the dropped fingerprint and which isn't already
/// bound to another project.
fn find_match<'a>(
    fp: &DroppedFingerprint,
    candidates: &'a [EnumeratedWindow],
    bound: &HashSet<isize>,
) -> Option<&'a EnumeratedWindow> {
    let needle = fp.title_pattern.trim().to_lowercase();
    let needle = needle.as_str();
    candidates
        .iter()
        .filter(|w| !bound.contains(&w.hwnd))
        .filter(|w| w.exe_path.eq_ignore_ascii_case(&fp.exe_path) || w.exe_path == fp.exe_path)
        .find(|w| {
            if !needle.is_empty() && w.title.to_lowercase().contains(needle) {
                return true;
            }
            if let Some(class) = &fp.class_name {
                return &w.class_name == class;
            }
            false
        })
}

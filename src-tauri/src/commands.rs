//! Tauri commands exposed to the Svelte frontend.
//!
//! Each command is thin: it routes input to a domain operation in
//! `window_manager`, `store`, or `event_log`, emits an event log entry, and
//! returns a serializable response. Errors are flattened to `String` so the
//! frontend gets readable messages without leaking Rust types.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

use chrono::Utc;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::event_log::{EventKind, EventLog};
use crate::project::{palette_for_index, Project, WindowRef, DEFAULT_PALETTE};
use crate::store::ProjectStore;
use crate::window_manager::{self, EnumeratedWindow};

/// Application-level state shared across all commands.
pub struct AppState {
    pub store: ProjectStore,
    pub events: EventLog,
    pub active: Mutex<ActiveProject>,
    pub self_pid: u32,
}

#[derive(Default)]
pub struct ActiveProject {
    pub id: Option<Uuid>,
    pub since: Option<Instant>,
}

/// Convenience: convert a Rust error into a string for frontend consumption.
fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

// -------------------------------------------------------------------
// Window enumeration
// -------------------------------------------------------------------

#[tauri::command]
pub fn list_open_windows(state: State<'_, Arc<AppState>>) -> Vec<EnumeratedWindow> {
    window_manager::enumerate_windows(Some(state.self_pid))
}

// -------------------------------------------------------------------
// Project CRUD
// -------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct CreateProjectInput {
    pub name: String,
    /// HWNDs (as `isize`) selected from the live enumeration.
    pub window_hwnds: Vec<isize>,
    /// Optional explicit accent colour. If `None` we round-robin the palette.
    pub color: Option<String>,
}

#[tauri::command]
pub fn list_projects(state: State<'_, Arc<AppState>>) -> Vec<ProjectView> {
    state
        .store
        .projects()
        .into_iter()
        .map(ProjectView::from)
        .collect()
}

#[tauri::command]
pub fn create_project(
    state: State<'_, Arc<AppState>>,
    input: CreateProjectInput,
) -> Result<ProjectView, String> {
    let live = window_manager::enumerate_windows(Some(state.self_pid));
    let wanted: HashSet<isize> = input.window_hwnds.iter().copied().collect();

    let mut window_refs = Vec::new();
    for w in &live {
        if !wanted.contains(&w.hwnd) {
            continue;
        }
        let class = if w.class_name.is_empty() {
            None
        } else {
            Some(w.class_name.clone())
        };
        window_refs.push(WindowRef::new(
            w.title.clone(),
            w.exe_path.clone(),
            class,
            w.hwnd,
        ));
    }

    let existing_count = state.store.projects().len();
    let color = input
        .color
        .unwrap_or_else(|| palette_for_index(existing_count));
    let mut project = Project::new(input.name.trim().to_string(), color, window_refs);
    if existing_count < 9 {
        project.hotkey_index = Some(existing_count as u8 + 1);
    }
    state.store.upsert(project.clone()).map_err(err)?;
    state.events.append(
        EventKind::ProjectCreated {
            name: project.name.clone(),
        },
        Some(project.id),
    );
    Ok(ProjectView::from(project))
}

#[tauri::command]
pub fn delete_project(state: State<'_, Arc<AppState>>, id: Uuid) -> Result<bool, String> {
    let name = state
        .store
        .get(id)
        .map(|p| p.name)
        .unwrap_or_else(|| "<unknown>".to_string());
    let removed = state.store.delete(id).map_err(err)?;
    if removed {
        state
            .events
            .append(EventKind::ProjectDeleted { name }, Some(id));
        let mut active = state.active.lock();
        if active.id == Some(id) {
            active.id = None;
            active.since = None;
        }
    }
    Ok(removed)
}

#[derive(Debug, Deserialize)]
pub struct UpdateProjectInput {
    pub id: Uuid,
    pub name: Option<String>,
    pub color: Option<String>,
    pub initials: Option<String>,
    pub hotkey_index: Option<Option<u8>>,
    /// Optional list of HWNDs to *replace* the project's window set with. If
    /// `None`, the existing windows are kept untouched.
    pub window_hwnds: Option<Vec<isize>>,
}

#[tauri::command]
pub fn update_project(
    state: State<'_, Arc<AppState>>,
    input: UpdateProjectInput,
) -> Result<ProjectView, String> {
    let mut project = state
        .store
        .get(input.id)
        .ok_or_else(|| "project not found".to_string())?;
    if let Some(name) = input.name {
        project.name = name.trim().to_string();
    }
    if let Some(color) = input.color {
        project.color = color;
    }
    if let Some(initials) = input.initials {
        let trimmed = initials.trim();
        if !trimmed.is_empty() {
            project.initials = trimmed.chars().take(2).collect::<String>().to_uppercase();
        }
    }
    if let Some(hk) = input.hotkey_index {
        project.hotkey_index = hk;
    }
    if let Some(hwnds) = input.window_hwnds {
        let live = window_manager::enumerate_windows(Some(state.self_pid));
        let live_by_hwnd: HashMap<isize, &EnumeratedWindow> =
            live.iter().map(|w| (w.hwnd, w)).collect();
        project.windows = hwnds
            .into_iter()
            .filter_map(|h| live_by_hwnd.get(&h).copied())
            .map(|w| {
                let class = if w.class_name.is_empty() {
                    None
                } else {
                    Some(w.class_name.clone())
                };
                WindowRef::new(w.title.clone(), w.exe_path.clone(), class, w.hwnd)
            })
            .collect();
    }
    project.updated_at = Utc::now();
    state.store.upsert(project.clone()).map_err(err)?;
    state.events.append(
        EventKind::ProjectUpdated {
            name: project.name.clone(),
        },
        Some(project.id),
    );
    Ok(ProjectView::from(project))
}

#[tauri::command]
pub fn reorder_projects(
    state: State<'_, Arc<AppState>>,
    order: Vec<Uuid>,
) -> Result<(), String> {
    state.store.reorder(&order).map_err(err)
}

// -------------------------------------------------------------------
// Activation (focus mode)
// -------------------------------------------------------------------

#[derive(Debug, Serialize)]
pub struct ActivationResult {
    pub project: ProjectView,
    pub windows_focused: usize,
    pub windows_minimized: usize,
    pub windows_missing: Vec<String>,
}

/// Activate a project by ID. Implements "Hard Switch" (mode A):
/// 1. Re-discover live HWNDs for each stored `WindowRef`.
/// 2. Minimize every other visible top-level window that is not part of
///    this project (and not the dock itself).
/// 3. Restore + raise every window in the project, focusing the first one
///    last so it ends up on top.
#[tauri::command]
pub fn activate_project(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    id: Uuid,
) -> Result<ActivationResult, String> {
    do_activate(&app, state.inner(), id)
}

/// Same as [`activate_project`] but callable from non-command contexts
/// (e.g. the global-shortcut handler).
pub fn do_activate(
    app: &AppHandle,
    state: &Arc<AppState>,
    id: Uuid,
) -> Result<ActivationResult, String> {
    let mut project = state
        .store
        .get(id)
        .ok_or_else(|| "project not found".to_string())?;

    // Re-bind stored windows to live HWNDs.
    let live = window_manager::enumerate_windows(Some(state.self_pid));
    let mut active_hwnds: HashSet<isize> = HashSet::new();
    let mut missing: Vec<String> = Vec::new();
    for w in &mut project.windows {
        match window_manager::match_window(w, &live) {
            Some(found) => {
                w.last_seen_hwnd = Some(found.hwnd);
                active_hwnds.insert(found.hwnd);
            }
            None => {
                w.last_seen_hwnd = None;
                missing.push(w.title_snapshot.clone());
            }
        }
    }

    // Add the dock window itself to the "do-not-touch" set so we don't
    // accidentally minimise it.
    #[cfg(windows)]
    if let Some(dock) = app.get_webview_window("main") {
        if let Ok(dock_hwnd) = dock.hwnd() {
            active_hwnds.insert(dock_hwnd.0 as isize);
        }
    }
    #[cfg(not(windows))]
    let _ = &app; // silence unused warning on non-Windows

    // Minimise everything else.
    let mut minimized = 0usize;
    for w in &live {
        if active_hwnds.contains(&w.hwnd) || w.minimized {
            continue;
        }
        window_manager::minimize_window(w.hwnd);
        minimized += 1;
    }

    // Decide which window is "primary" (= the one we actually steal focus
    // for). For now: the first window in the stored order that has a live
    // HWND. The user can re-order windows inside a project later in v0.2.
    let primary_idx = project.windows.iter().position(|w| w.last_seen_hwnd.is_some());
    let mut focused = 0usize;

    if let Some(primary_idx) = primary_idx {
        // Phase 1: raise every non-primary project window without stealing
        // focus, so they're all visible behind the primary.
        for (i, w) in project.windows.iter().enumerate() {
            if i == primary_idx {
                continue;
            }
            if let Some(h) = w.last_seen_hwnd {
                window_manager::raise_window_noactivate(h);
                focused += 1;
            }
        }

        // Phase 2: focus the primary window with the full foreground-lock
        // dance, and log the result so we can debug failed switches.
        let primary = &project.windows[primary_idx];
        if let Some(h) = primary.last_seen_hwnd {
            let (succeeded, used_fallback) = window_manager::focus_window(h);
            focused += 1;
            state.events.append(
                EventKind::FocusAttempt {
                    project_name: project.name.clone(),
                    title: primary.title_snapshot.clone(),
                    hwnd: h as i64,
                    succeeded,
                    used_fallback,
                },
                Some(project.id),
            );
        }
    }

    // Bookkeeping: update last_activated_at and active state.
    project.last_activated_at = Some(Utc::now());
    project.updated_at = Utc::now();
    state.store.upsert(project.clone()).map_err(err)?;

    let prev_id;
    let duration_in_prev_ms;
    {
        let mut active = state.active.lock();
        prev_id = active.id;
        duration_in_prev_ms = active
            .since
            .map(|t| t.elapsed().as_millis().min(u64::MAX as u128) as u64);
        active.id = Some(project.id);
        active.since = Some(Instant::now());
    }

    state.events.append(
        EventKind::ProjectActivated {
            name: project.name.clone(),
            from: prev_id,
            duration_in_prev_ms,
            windows_focused: focused,
            windows_minimized: minimized,
        },
        Some(project.id),
    );

    // Inform any missing windows in the log too — useful for debugging.
    for title in &missing {
        state.events.append(
            EventKind::WindowMissing {
                project_name: project.name.clone(),
                title: title.clone(),
            },
            Some(project.id),
        );
    }

    Ok(ActivationResult {
        project: ProjectView::from(project),
        windows_focused: focused,
        windows_minimized: minimized,
        windows_missing: missing,
    })
}

// -------------------------------------------------------------------
// Dock visibility / activation hotkey routing helper
// -------------------------------------------------------------------

#[tauri::command]
pub fn set_dock_visible(app: AppHandle, state: State<'_, Arc<AppState>>, visible: bool) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("main") {
        if visible {
            win.show().map_err(err)?;
        } else {
            win.hide().map_err(err)?;
        }
        state.events.append(EventKind::DockToggled { visible }, None);
    }
    Ok(())
}

#[tauri::command]
pub fn activate_by_hotkey_index(
    app: AppHandle,
    state: State<'_, Arc<AppState>>,
    index: u8,
) -> Result<Option<ActivationResult>, String> {
    let projects = state.store.projects();
    let target = projects.into_iter().find(|p| p.hotkey_index == Some(index));
    if let Some(p) = target {
        Ok(Some(do_activate(&app, state.inner(), p.id)?))
    } else {
        Ok(None)
    }
}

// -------------------------------------------------------------------
// Event log read-back
// -------------------------------------------------------------------

#[tauri::command]
pub fn read_recent_events(
    state: State<'_, Arc<AppState>>,
    limit: usize,
) -> Result<Vec<crate::event_log::Event>, String> {
    state.events.read_recent(limit).map_err(err)
}

// -------------------------------------------------------------------
// Misc
// -------------------------------------------------------------------

#[tauri::command]
pub fn palette_colors() -> Vec<&'static str> {
    DEFAULT_PALETTE.to_vec()
}

// -------------------------------------------------------------------
// Frontend-facing project view (drops volatile HWND fields).
// -------------------------------------------------------------------

#[derive(Debug, Serialize, Deserialize)]
pub struct ProjectView {
    pub id: Uuid,
    pub name: String,
    pub color: String,
    pub initials: String,
    pub hotkey_index: Option<u8>,
    pub windows: Vec<WindowRefView>,
    pub created_at: chrono::DateTime<Utc>,
    pub updated_at: chrono::DateTime<Utc>,
    pub last_activated_at: Option<chrono::DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WindowRefView {
    pub id: Uuid,
    pub title_snapshot: String,
    pub title_pattern: String,
    pub exe_path: String,
}

impl From<Project> for ProjectView {
    fn from(p: Project) -> Self {
        Self {
            id: p.id,
            name: p.name,
            color: p.color,
            initials: p.initials,
            hotkey_index: p.hotkey_index,
            windows: p
                .windows
                .into_iter()
                .map(|w| WindowRefView {
                    id: w.id,
                    title_snapshot: w.title_snapshot,
                    title_pattern: w.title_pattern,
                    exe_path: w.exe_path,
                })
                .collect(),
            created_at: p.created_at,
            updated_at: p.updated_at,
            last_activated_at: p.last_activated_at,
        }
    }
}

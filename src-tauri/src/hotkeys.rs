//! Global-hotkey registration & routing.
//!
//! Every project's activation combo and the global dock-toggle combo
//! are end-user-customisable. Whenever projects or preferences change we
//! rebuild the global-shortcut set from scratch:
//!
//! 1. Unregister every previously-registered shortcut.
//! 2. Walk the projects + preferences and parse each combo via
//!    [`tauri_plugin_global_shortcut::Shortcut::from_str`].
//! 3. Re-register each, populating the [`ShortcutRouter`] so the global
//!    handler in `lib.rs` knows what to do when a combo fires.
//!
//! Parsing failures are logged (so a typo in a stored combo doesn't
//! crash the app) but never fatal — the offending project simply loses
//! its hotkey until it's edited.

use std::str::FromStr;
use std::sync::Arc;

use anyhow::Result;
use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Modifiers, Shortcut};

use crate::commands::{AppState, RouterAction, ShortcutRouter};
use crate::project::Project;

/// Build the canonical "Ctrl+Alt+Digit5"-style string for a `Shortcut`.
/// Used both at registration time (to populate the router) and at
/// handler time (to look up the incoming shortcut's action).
pub fn format_shortcut(s: &Shortcut) -> String {
    let mut parts = Vec::new();
    if s.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl".to_string());
    }
    if s.mods.contains(Modifiers::ALT) {
        parts.push("Alt".to_string());
    }
    if s.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift".to_string());
    }
    if s.mods.contains(Modifiers::SUPER) {
        parts.push("Win".to_string());
    }
    parts.push(format!("{:?}", s.key));
    parts.join("+").to_lowercase()
}

/// Re-register every global shortcut from scratch using the current
/// state of `AppState`. Safe to call from any thread that can talk to
/// the Tauri runtime.
pub fn reregister(app: &AppHandle) -> Result<()> {
    let Some(state) = app.try_state::<Arc<AppState>>() else {
        return Ok(());
    };

    let gs = app.global_shortcut();
    let _ = gs.unregister_all();

    let mut router = ShortcutRouter::default();

    // 1. Dock toggle.
    let dock_combo = state.prefs.dock_toggle_combo();
    register_or_log(
        gs,
        &dock_combo,
        RouterAction::ToggleDock,
        &mut router,
        "dock toggle",
    );

    // 2. Per-project shortcuts.
    let projects = state.store.projects();
    for p in &projects {
        if let Some(combo) = effective_combo(p) {
            register_or_log(
                gs,
                &combo,
                RouterAction::ActivateProject(p.id),
                &mut router,
                &format!("project {:?}", p.name),
            );
        }
    }

    *state.router.lock() = router;
    Ok(())
}

/// Resolve the activation combo for a project: prefer the user's custom
/// override, otherwise fall back to the legacy slot index. Returns
/// `None` when the project has no hotkey at all.
fn effective_combo(p: &Project) -> Option<String> {
    if let Some(combo) = &p.hotkey_combo {
        if !combo.trim().is_empty() {
            return Some(combo.clone());
        }
    }
    let idx = p.hotkey_index?;
    if (1..=9).contains(&idx) {
        Some(format!("Ctrl+Alt+Digit{idx}"))
    } else {
        None
    }
}

fn register_or_log<R: tauri::Runtime>(
    gs: &tauri_plugin_global_shortcut::GlobalShortcut<R>,
    combo: &str,
    action: RouterAction,
    router: &mut ShortcutRouter,
    label: &str,
) {
    let shortcut = match Shortcut::from_str(combo) {
        Ok(s) => s,
        Err(err) => {
            tracing::warn!(?err, combo, label, "skipping invalid hotkey combo");
            return;
        }
    };
    let key = format_shortcut(&shortcut);
    if router.by_combo.contains_key(&key) {
        tracing::warn!(
            combo,
            label,
            "duplicate hotkey combo — keeping first binding"
        );
        return;
    }
    if let Err(err) = gs.register(shortcut) {
        tracing::warn!(?err, combo, label, "failed to register hotkey");
        return;
    }
    router.by_combo.insert(key, action);
}

/// Parse and canonicalise a combo, returning `Err` (as a user-facing
/// string) if the input is invalid. Used by command handlers to
/// validate user input before persisting.
pub fn validate_combo(combo: &str) -> std::result::Result<String, String> {
    let trimmed = combo.trim();
    if trimmed.is_empty() {
        return Err("Hotkey combo is empty".to_string());
    }
    Shortcut::from_str(trimmed).map_err(|e| format!("Invalid hotkey combo: {e}"))?;
    Ok(trimmed.to_string())
}

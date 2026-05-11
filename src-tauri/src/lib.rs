//! ProjectHub — always-on-top dock & orchestrator for multi-window workflows.

mod commands;
mod event_log;
mod project;
mod pruner;
mod store;
mod window_manager;

use std::sync::Arc;

use parking_lot::Mutex;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut, ShortcutState};

use crate::commands::{ActiveProject, AppState};
use crate::event_log::{EventKind, EventLog};
use crate::store::ProjectStore;

/// Hotkey combos we register globally. Mirror these in `frontend` keymaps.
const HOTKEY_TOGGLE_DOCK: (Modifiers, Code) =
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Space);
const HOTKEY_PROJECT_PREFIX: &[(Modifiers, Code, u8)] = &[
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit1, 1),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit2, 2),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit3, 3),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit4, 4),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit5, 5),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit6, 6),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit7, 7),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit8, 8),
    (Modifiers::CONTROL.union(Modifiers::ALT), Code::Digit9, 9),
];

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    init_tracing();

    let store = ProjectStore::default_path()
        .and_then(ProjectStore::load_or_init)
        .expect("failed to open project store");
    let events = EventLog::default_path()
        .and_then(EventLog::open)
        .expect("failed to open event log");
    events.append(EventKind::AppStarted, None);

    let state = Arc::new(AppState {
        store,
        events,
        active: Mutex::new(ActiveProject::default()),
        self_pid: window_manager::current_pid(),
    });

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_positioner::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(handle_global_shortcut)
                .build(),
        )
        .manage(state.clone())
        .setup(move |app| {
            register_global_shortcuts(app.handle())?;
            install_tray(app.handle())?;
            if let Some(state) = app.try_state::<Arc<AppState>>() {
                pruner::spawn(app.handle().clone(), state.inner().clone());
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_open_windows,
            commands::list_projects,
            commands::create_project,
            commands::update_project,
            commands::delete_project,
            commands::reorder_projects,
            commands::activate_project,
            commands::activate_by_hotkey_index,
            commands::set_dock_visible,
            commands::read_recent_events,
            commands::palette_colors,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn init_tracing() {
    use tracing_subscriber::EnvFilter;
    let _ = tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .with_target(false)
        .try_init();
}

fn register_global_shortcuts(app: &tauri::AppHandle) -> tauri::Result<()> {
    let gs = app.global_shortcut();

    let toggle = Shortcut::new(Some(HOTKEY_TOGGLE_DOCK.0), HOTKEY_TOGGLE_DOCK.1);
    gs.register(toggle).map_err(to_tauri_err)?;

    for (mods, code, _idx) in HOTKEY_PROJECT_PREFIX {
        let shortcut = Shortcut::new(Some(*mods), *code);
        gs.register(shortcut).map_err(to_tauri_err)?;
    }

    Ok(())
}

fn to_tauri_err<E: std::fmt::Display>(err: E) -> tauri::Error {
    tauri::Error::Anyhow(anyhow::anyhow!(err.to_string()))
}

fn handle_global_shortcut(
    app: &tauri::AppHandle,
    shortcut: &Shortcut,
    event: tauri_plugin_global_shortcut::ShortcutEvent,
) {
    if event.state() != ShortcutState::Pressed {
        return;
    }

    let combo = format_shortcut(shortcut);

    if matches_hotkey(shortcut, HOTKEY_TOGGLE_DOCK.0, HOTKEY_TOGGLE_DOCK.1) {
        toggle_dock_visibility(app, &combo);
        return;
    }

    if let Some(idx) = hotkey_to_project_index(shortcut) {
        if let Some(state) = app.try_state::<Arc<AppState>>() {
            state
                .events
                .append(EventKind::HotkeyTriggered { combo: combo.clone() }, None);
            let _ = app.emit("hotkey:project", idx);

            // Run activation off the UI thread.
            let app_clone = app.clone();
            let state_clone: Arc<AppState> = state.inner().clone();
            std::thread::spawn(move || {
                let projects = state_clone.store.projects();
                let Some(p) = projects.iter().find(|p| p.hotkey_index == Some(idx)) else {
                    return;
                };
                match commands::do_activate(&app_clone, &state_clone, p.id) {
                    Ok(result) => {
                        let _ = app_clone.emit("project:activated", &result);
                    }
                    Err(err) => tracing::warn!(?err, "hotkey activation failed"),
                }
            });
        }
    }
}

fn toggle_dock_visibility(app: &tauri::AppHandle, combo: &str) {
    if let Some(state) = app.try_state::<Arc<AppState>>() {
        state
            .events
            .append(EventKind::HotkeyTriggered { combo: combo.to_string() }, None);
    }
    if let Some(win) = app.get_webview_window("main") {
        let visible = win.is_visible().unwrap_or(true);
        if visible {
            let _ = win.hide();
        } else {
            let _ = win.show();
            let _ = win.set_focus();
        }
        if let Some(state) = app.try_state::<Arc<AppState>>() {
            state.events.append(
                EventKind::DockToggled { visible: !visible },
                None,
            );
        }
    }
}

fn matches_hotkey(shortcut: &Shortcut, mods: Modifiers, key: Code) -> bool {
    shortcut.mods == mods && shortcut.key == key
}

fn hotkey_to_project_index(shortcut: &Shortcut) -> Option<u8> {
    HOTKEY_PROJECT_PREFIX
        .iter()
        .find(|(m, k, _)| shortcut.mods == *m && shortcut.key == *k)
        .map(|(_, _, idx)| *idx)
}

fn format_shortcut(s: &Shortcut) -> String {
    let mut parts = Vec::new();
    if s.mods.contains(Modifiers::CONTROL) {
        parts.push("Ctrl");
    }
    if s.mods.contains(Modifiers::ALT) {
        parts.push("Alt");
    }
    if s.mods.contains(Modifiers::SHIFT) {
        parts.push("Shift");
    }
    if s.mods.contains(Modifiers::SUPER) {
        parts.push("Win");
    }
    let key = format!("{:?}", s.key);
    parts.push(&key);
    parts.join("+")
}

fn install_tray(app: &tauri::AppHandle) -> tauri::Result<()> {
    let show_item = MenuItem::with_id(app, "tray:show", "Show dock", true, None::<&str>)?;
    let hide_item = MenuItem::with_id(app, "tray:hide", "Hide dock", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "tray:quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_item, &hide_item, &quit_item])?;

    TrayIconBuilder::with_id("main-tray")
        .tooltip("ProjectHub")
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(|app, event| match event.id.as_ref() {
            "tray:show" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.show();
                    let _ = win.set_focus();
                }
            }
            "tray:hide" => {
                if let Some(win) = app.get_webview_window("main") {
                    let _ = win.hide();
                }
            }
            "tray:quit" => {
                if let Some(state) = app.try_state::<Arc<AppState>>() {
                    state.events.append(EventKind::AppShutdown, None);
                }
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(win) = app.get_webview_window("main") {
                    let visible = win.is_visible().unwrap_or(false);
                    if visible {
                        let _ = win.hide();
                    } else {
                        let _ = win.show();
                        let _ = win.set_focus();
                    }
                }
            }
        })
        .build(app)?;

    Ok(())
}

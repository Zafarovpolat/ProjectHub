// TypeScript mirrors of the Rust types in src-tauri/src/commands.rs.
// Keep these in sync with `ProjectView`, `WindowRefView`, etc.

export interface WindowRefView {
  id: string;
  title_snapshot: string;
  title_pattern: string;
  exe_path: string;
  /// Whether the window is currently rebindable to an open HWND. Goes
  /// `false` the moment the window disappears and stays false until either
  /// the window reappears or the backend's grace period expires and the
  /// ref is auto-removed.
  live: boolean;
  /// Consecutive pruner ticks the window has been missing. 0 when live.
  missed_ticks: number;
}

export interface PreferencesView {
  /// Effective dock-toggle combo (user override if set, else default).
  dock_toggle_hotkey: string;
  /// Whether `dock_toggle_hotkey` is a user override or the default.
  dock_toggle_is_custom: boolean;
}

export interface ProjectView {
  id: string;
  name: string;
  color: string;
  initials: string;
  hotkey_index: number | null;
  /// User-defined activation combo (e.g. `"Ctrl+Alt+F1"`). When set,
  /// overrides `hotkey_index`. `null` means "use the slot-index combo".
  hotkey_combo: string | null;
  windows: WindowRefView[];
  created_at: string;
  updated_at: string;
  last_activated_at: string | null;
}

export interface EnumeratedWindow {
  hwnd: number;
  title: string;
  exe_path: string;
  class_name: string;
  pid: number;
  minimized: boolean;
}

export interface ActivationResult {
  project: ProjectView;
  windows_focused: number;
  windows_minimized: number;
  windows_missing: string[];
}

export interface CreateProjectInput {
  name: string;
  window_hwnds: number[];
  color?: string | null;
}

export interface UpdateProjectInput {
  id: string;
  name?: string | null;
  color?: string | null;
  initials?: string | null;
  hotkey_index?: number | null | undefined;
  /// Set/clear the per-project hotkey combo. Use `undefined` to leave
  /// it untouched, `null` to clear, or a string to set.
  hotkey_combo?: string | null | undefined;
  window_hwnds?: number[] | null;
}

export type EventKind =
  | { type: "app_started" }
  | { type: "app_shutdown" }
  | { type: "project_created"; name: string }
  | { type: "project_deleted"; name: string }
  | { type: "project_updated"; name: string }
  | {
      type: "project_activated";
      name: string;
      from: string | null;
      duration_in_prev_ms: number | null;
      windows_focused: number;
      windows_minimized: number;
    }
  | { type: "window_reattached"; project_name: string; title: string }
  | { type: "window_missing"; project_name: string; title: string }
  | {
      type: "window_auto_removed";
      project_name: string;
      title: string;
      missed_ticks: number;
    }
  | { type: "window_auto_rebound"; project_name: string; title: string }
  | { type: "hotkey_triggered"; combo: string }
  | { type: "dock_toggled"; visible: boolean };

export interface EventEntry {
  id: string;
  timestamp: string;
  project_id: string | null;
  // EventKind fields are flattened in serialization (serde flatten).
  // We use a loose type because `serde(flatten)` makes the discriminant share
  // the level with the rest of the struct.
  [key: string]: unknown;
}

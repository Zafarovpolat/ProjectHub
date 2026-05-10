// TypeScript mirrors of the Rust types in src-tauri/src/commands.rs.
// Keep these in sync with `ProjectView`, `WindowRefView`, etc.

export interface WindowRefView {
  id: string;
  title_snapshot: string;
  title_pattern: string;
  exe_path: string;
}

export interface ProjectView {
  id: string;
  name: string;
  color: string;
  initials: string;
  hotkey_index: number | null;
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

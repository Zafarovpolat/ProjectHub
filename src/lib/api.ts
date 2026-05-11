// Thin wrappers around `invoke()` that give us typed names + a single place to
// debounce, retry, or log Tauri calls in the future.

import { invoke } from "@tauri-apps/api/core";
import type {
  ActivationResult,
  CreateProjectInput,
  EnumeratedWindow,
  EventEntry,
  PreferencesView,
  ProjectView,
  UpdateProjectInput,
} from "./types";

export const api = {
  listOpenWindows(): Promise<EnumeratedWindow[]> {
    return invoke("list_open_windows");
  },

  listProjects(): Promise<ProjectView[]> {
    return invoke("list_projects");
  },

  createProject(input: CreateProjectInput): Promise<ProjectView> {
    return invoke("create_project", { input });
  },

  updateProject(input: UpdateProjectInput): Promise<ProjectView> {
    return invoke("update_project", { input });
  },

  deleteProject(id: string): Promise<boolean> {
    return invoke("delete_project", { id });
  },

  reorderProjects(order: string[]): Promise<void> {
    return invoke("reorder_projects", { order });
  },

  addWindowsToProject(id: string, hwnds: number[]): Promise<ProjectView> {
    return invoke("add_windows_to_project", { id, hwnds });
  },

  removeWindowFromProject(id: string, windowId: string): Promise<ProjectView> {
    return invoke("remove_window_from_project", { id, windowId });
  },

  activateProject(id: string): Promise<ActivationResult> {
    return invoke("activate_project", { id });
  },

  setDockVisible(visible: boolean): Promise<void> {
    return invoke("set_dock_visible", { visible });
  },

  readRecentEvents(limit = 100): Promise<EventEntry[]> {
    return invoke("read_recent_events", { limit });
  },

  paletteColors(): Promise<string[]> {
    return invoke("palette_colors");
  },

  getPreferences(): Promise<PreferencesView> {
    return invoke("get_preferences");
  },

  /// Pass `combo=null` to revert to the built-in default
  /// (`Ctrl+Alt+Space`). Returns the new effective preferences.
  setDockToggleHotkey(combo: string | null): Promise<PreferencesView> {
    return invoke("set_dock_toggle_hotkey", { combo });
  },

  /// Validate that `combo` parses correctly. Resolves with the trimmed
  /// canonical input on success, rejects with a user-facing message
  /// otherwise.
  validateHotkeyCombo(combo: string): Promise<string> {
    return invoke("validate_hotkey_combo", { combo });
  },
};

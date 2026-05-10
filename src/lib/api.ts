// Thin wrappers around `invoke()` that give us typed names + a single place to
// debounce, retry, or log Tauri calls in the future.

import { invoke } from "@tauri-apps/api/core";
import type {
  ActivationResult,
  CreateProjectInput,
  EnumeratedWindow,
  EventEntry,
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
};

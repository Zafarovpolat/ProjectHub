// Runes-based global store. Svelte 5 lets us export `$state` from a module so
// every component sees the same reactive object.

import { api } from "./api";
import type { ProjectView } from "./types";

interface DockState {
  projects: ProjectView[];
  activeId: string | null;
  loading: boolean;
  error: string | null;
}

export const dock = $state<DockState>({
  projects: [],
  activeId: null,
  loading: false,
  error: null,
});

export async function refreshProjects() {
  dock.loading = true;
  dock.error = null;
  try {
    dock.projects = await api.listProjects();
  } catch (err) {
    dock.error = String(err);
  } finally {
    dock.loading = false;
  }
}

export async function activate(id: string) {
  try {
    const result = await api.activateProject(id);
    dock.activeId = result.project.id;
    // Refresh in background so `last_activated_at` reflects the new value.
    void refreshProjects();
    return result;
  } catch (err) {
    dock.error = String(err);
    throw err;
  }
}

export async function remove(id: string) {
  await api.deleteProject(id);
  if (dock.activeId === id) dock.activeId = null;
  await refreshProjects();
}

<script lang="ts">
  import { onMount } from "svelte";
  import { emit, listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import type { PreferencesView, ProjectView } from "$lib/types";
  import { api } from "$lib/api";
  import HotkeyInput from "$lib/components/HotkeyInput.svelte";
  import WindowPicker from "$lib/components/WindowPicker.svelte";
  import {
    Trash2,
    ChevronUp,
    ChevronDown,
    Hash,
    AlertTriangle,
    Plus,
    X,
  } from "lucide-svelte";

  let projects = $state<ProjectView[]>([]);
  let prefs = $state<PreferencesView | null>(null);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);
  let pendingDeleteId = $state<string | null>(null);
  let expandedId = $state<string | null>(null);
  /// Project ID whose add-window picker is currently open. Only one
  /// picker can be open at a time to keep the UI uncluttered.
  let addingForId = $state<string | null>(null);
  let pickerSelection = $state<number[]>([]);

  async function refresh() {
    try {
      const [list, p] = await Promise.all([
        api.listProjects(),
        api.getPreferences(),
      ]);
      projects = list;
      prefs = p;
      loading = false;
    } catch (e) {
      errorMessage = String(e);
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") {
        if (pendingDeleteId) {
          pendingDeleteId = null;
        } else {
          void getCurrentWindow().close();
        }
      }
    }
    window.addEventListener("keydown", onKey);
    // The backend pruner emits `project:changed` whenever a window's
    // live state flips or a ref is auto-removed. Re-fetch so badges
    // and counts stay in sync without manual refresh.
    const unlistenChanged = listen("project:changed", () => {
      void refresh();
    });
    return () => {
      window.removeEventListener("keydown", onKey);
      void unlistenChanged.then((fn) => fn());
    };
  });

  async function deleteProject(id: string) {
    try {
      await api.deleteProject(id);
      await emit("project:changed");
      pendingDeleteId = null;
      await refresh();
    } catch (e) {
      errorMessage = String(e);
    }
  }

  async function commitAddWindows(id: string) {
    if (pickerSelection.length === 0) {
      addingForId = null;
      return;
    }
    try {
      await api.addWindowsToProject(id, pickerSelection);
      await emit("project:changed");
      pickerSelection = [];
      addingForId = null;
      expandedId = id;
      await refresh();
    } catch (e) {
      errorMessage = String(e);
    }
  }

  async function removeWindow(projectId: string, windowId: string) {
    try {
      await api.removeWindowFromProject(projectId, windowId);
      await emit("project:changed");
      await refresh();
    } catch (e) {
      errorMessage = String(e);
    }
  }

  async function setHotkeyCombo(id: string, combo: string | null) {
    try {
      // Sending `null` clears the custom combo and falls back to the
      // slot-index hotkey (which may also be null).
      await api.updateProject({ id, hotkey_combo: combo });
      await emit("project:changed");
      await refresh();
    } catch (e) {
      errorMessage = String(e);
    }
  }

  async function setDockToggleHotkey(combo: string | null) {
    try {
      const next = await api.setDockToggleHotkey(combo);
      prefs = next;
    } catch (e) {
      errorMessage = String(e);
    }
  }

  async function moveUp(idx: number) {
    if (idx === 0) return;
    const next = projects.slice();
    [next[idx - 1], next[idx]] = [next[idx], next[idx - 1]];
    projects = next;
    try {
      await api.reorderProjects(next.map((p) => p.id));
      await emit("project:changed");
    } catch (e) {
      errorMessage = String(e);
      await refresh();
    }
  }
  async function moveDown(idx: number) {
    if (idx === projects.length - 1) return;
    const next = projects.slice();
    [next[idx], next[idx + 1]] = [next[idx + 1], next[idx]];
    projects = next;
    try {
      await api.reorderProjects(next.map((p) => p.id));
      await emit("project:changed");
    } catch (e) {
      errorMessage = String(e);
      await refresh();
    }
  }

  async function setHotkey(id: string, value: number | null) {
    try {
      await api.updateProject({ id, hotkey_index: value });
      await emit("project:changed");
      await refresh();
    } catch (e) {
      errorMessage = String(e);
    }
  }
</script>

<svelte:head>
  <title>Manage projects — ProjectHub</title>
</svelte:head>

<main
  class="flex h-screen w-screen flex-col gap-5 bg-zinc-950 p-6 text-zinc-100"
>
  <header>
    <h1 class="text-base font-semibold">Manage projects</h1>
    <p class="mt-0.5 text-[11px] text-zinc-500">
      Reorder, customise hotkeys, add or remove windows, and pick the dock-toggle combo.
    </p>
  </header>

  {#if prefs}
    <section class="rounded-xl border border-white/8 bg-white/3 p-3">
      <div class="flex items-center justify-between gap-3">
        <div class="flex min-w-0 flex-col">
          <span class="text-[11px] uppercase tracking-wide text-zinc-500">
            Dock toggle hotkey
          </span>
          <span class="text-[11px] text-zinc-500">
            Global combo that shows or hides the dock.
            {#if !prefs.dock_toggle_is_custom}
              <span class="text-zinc-600">(using default)</span>
            {/if}
          </span>
        </div>
        <HotkeyInput
          value={prefs.dock_toggle_hotkey}
          showReset={prefs.dock_toggle_is_custom}
          onchange={(combo) => setDockToggleHotkey(combo)}
        />
      </div>
    </section>
  {/if}

  {#if errorMessage}
    <p class="rounded border border-red-500/30 bg-red-500/10 px-2 py-1.5 text-xs text-red-300">
      {errorMessage}
    </p>
  {/if}

  <section class="flex flex-1 min-h-0 flex-col gap-2 overflow-y-auto pr-1">
    {#if loading}
      <p class="text-xs text-zinc-500">Loading…</p>
    {:else if projects.length === 0}
      <p class="text-xs text-zinc-500">No projects yet.</p>
    {:else}
      {#each projects as p, idx (p.id)}
        {@const offlineCount = p.windows.filter((w) => !w.live).length}
        <article
          class="flex flex-col gap-2 rounded-xl border border-white/8 bg-white/3 p-3"
        >
          <div class="flex items-center gap-3">
            <span
              class="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg text-xs font-semibold"
              style:background-color="color-mix(in srgb, {p.color} 18%, transparent)"
              style:color="color-mix(in srgb, {p.color} 88%, white 12%)"
              style:border="1px solid color-mix(in srgb, {p.color} 24%, transparent)"
            >
              {p.initials}
            </span>
            <div class="min-w-0 flex-1">
              <div class="truncate text-sm font-medium">{p.name}</div>
              <button
                type="button"
                class="-mx-1 -my-0.5 flex items-center gap-1.5 rounded px-1 py-0.5 text-[11px] text-zinc-500 hover:bg-white/5 hover:text-zinc-300"
                onclick={() => (expandedId = expandedId === p.id ? null : p.id)}
                aria-expanded={expandedId === p.id}
              >
                <span>
                  {p.windows.length} window{p.windows.length === 1 ? "" : "s"}
                </span>
                {#if offlineCount > 0}
                  <span
                    class="inline-flex items-center gap-1 rounded-sm bg-orange-500/15 px-1.5 py-[1px] text-[10px] font-medium text-orange-300"
                    title="{offlineCount} window{offlineCount === 1 ? '' : 's'} appear closed — auto-removed after ~15s"
                  >
                    <AlertTriangle size={10} />
                    {offlineCount} closed
                  </span>
                {/if}
              </button>
            </div>

            <div class="flex items-center gap-1">
              <button
                type="button"
                class="flex h-7 w-7 items-center justify-center rounded-md text-zinc-400 hover:bg-white/8 hover:text-zinc-100 disabled:opacity-30"
                disabled={idx === 0}
                onclick={() => moveUp(idx)}
                aria-label="Move up"
              >
                <ChevronUp size={14} />
              </button>
              <button
                type="button"
                class="flex h-7 w-7 items-center justify-center rounded-md text-zinc-400 hover:bg-white/8 hover:text-zinc-100 disabled:opacity-30"
                disabled={idx === projects.length - 1}
                onclick={() => moveDown(idx)}
                aria-label="Move down"
              >
                <ChevronDown size={14} />
              </button>
              <button
                type="button"
                class="flex h-7 w-7 items-center justify-center rounded-md text-zinc-400 hover:bg-red-500/10 hover:text-red-300"
                onclick={() => (pendingDeleteId = p.id)}
                aria-label="Delete project"
              >
                <Trash2 size={14} />
              </button>
            </div>
          </div>

          <div class="flex flex-col gap-2 border-t border-white/5 pt-2 text-[11px] text-zinc-400">
            <div class="flex items-center gap-2">
              <Hash size={12} class="text-zinc-500" />
              <span>Slot:</span>
              <div class="flex flex-wrap gap-1">
                {#each Array(9) as _, n}
                  {@const num = n + 1}
                  {@const taken = projects.some(
                    (q) => q.id !== p.id && q.hotkey_index === num,
                  )}
                  <button
                    type="button"
                    class="h-6 w-6 rounded border text-[10px] font-mono transition-colors disabled:cursor-not-allowed"
                    style:background-color={p.hotkey_index === num
                      ? "color-mix(in srgb, " + p.color + " 22%, transparent)"
                      : "transparent"}
                    style:border-color={p.hotkey_index === num
                      ? p.color
                      : "rgba(255,255,255,0.1)"}
                    style:color={p.hotkey_index === num
                      ? p.color
                      : taken
                        ? "rgb(63,63,70)"
                        : "rgb(161,161,170)"}
                    disabled={taken}
                    title={taken ? "Used by another project" : `Ctrl+Alt+${num}`}
                    onclick={() => setHotkey(p.id, num)}
                  >
                    {num}
                  </button>
                {/each}
                <button
                  type="button"
                  class="h-6 px-2 rounded border border-dashed border-white/12 text-[10px] text-zinc-500 hover:text-zinc-200"
                  onclick={() => setHotkey(p.id, null)}
                  title="Clear slot hotkey"
                >
                  clear
                </button>
              </div>
            </div>
            <div class="flex items-center justify-between gap-2">
              <span class="text-zinc-500">
                Custom hotkey
                <span class="text-zinc-600">(overrides the slot above)</span>
              </span>
              <HotkeyInput
                value={p.hotkey_combo}
                placeholder="Click and press combo…"
                showReset={!!p.hotkey_combo}
                onchange={(combo) => setHotkeyCombo(p.id, combo)}
              />
            </div>
          </div>

          {#if expandedId === p.id}
            <div class="flex flex-col gap-2 rounded-md border border-white/5 bg-black/20 p-1.5">
              {#if p.windows.length > 0}
                <ul class="flex flex-col gap-1">
                  {#each p.windows as w (w.id)}
                    <li
                      class="group flex items-center justify-between gap-2 rounded px-2 py-1 text-[11px]"
                      class:opacity-60={!w.live}
                    >
                      <span
                        class="min-w-0 truncate"
                        class:line-through={!w.live}
                      >
                        {w.title_snapshot || w.title_pattern}
                      </span>
                      <span class="flex shrink-0 items-center gap-1">
                        {#if !w.live}
                          <span
                            class="inline-flex items-center gap-1 rounded-sm border border-orange-500/30 bg-orange-500/10 px-1.5 py-[1px] text-[10px] font-medium text-orange-300"
                            title="Missed {w.missed_ticks} pruner tick{w.missed_ticks === 1 ? '' : 's'}"
                          >
                            <AlertTriangle size={10} />
                            closed
                          </span>
                        {/if}
                        <button
                          type="button"
                          class="flex h-6 w-6 items-center justify-center rounded text-zinc-500 opacity-0 transition group-hover:opacity-100 hover:bg-red-500/10 hover:text-red-300"
                          onclick={() => removeWindow(p.id, w.id)}
                          aria-label="Remove window"
                          title="Remove from project"
                        >
                          <X size={12} />
                        </button>
                      </span>
                    </li>
                  {/each}
                </ul>
              {:else}
                <p class="px-2 py-1 text-[11px] text-zinc-500">No windows yet.</p>
              {/if}
              {#if addingForId === p.id}
                <div class="flex flex-col gap-2 rounded border border-white/5 bg-white/2 p-2">
                  <div class="flex items-center justify-between gap-2">
                    <span class="text-[11px] uppercase tracking-wide text-zinc-500">
                      Add windows to {p.name}
                    </span>
                    <button
                      type="button"
                      class="rounded px-2 py-0.5 text-[11px] text-zinc-400 hover:text-zinc-100"
                      onclick={() => {
                        addingForId = null;
                        pickerSelection = [];
                      }}
                    >
                      Cancel
                    </button>
                  </div>
                  <div class="h-56">
                    <WindowPicker
                      bind:selected={pickerSelection}
                      onchange={(hwnds) => (pickerSelection = hwnds)}
                    />
                  </div>
                  <div class="flex items-center justify-end gap-2">
                    <button
                      type="button"
                      class="rounded border border-white/16 bg-white/8 px-3 py-1 text-[11px] text-zinc-100 hover:bg-white/12 disabled:opacity-40"
                      disabled={pickerSelection.length === 0}
                      onclick={() => commitAddWindows(p.id)}
                    >
                      Add {pickerSelection.length || ""} window{pickerSelection.length === 1 ? "" : "s"}
                    </button>
                  </div>
                </div>
              {:else}
                <button
                  type="button"
                  class="flex items-center gap-1.5 self-start rounded-md border border-dashed border-white/12 px-2 py-1 text-[11px] text-zinc-400 hover:bg-white/4 hover:text-zinc-100"
                  onclick={() => {
                    addingForId = p.id;
                    pickerSelection = [];
                  }}
                >
                  <Plus size={12} />
                  Add windows
                </button>
              {/if}
            </div>
          {/if}

          {#if pendingDeleteId === p.id}
            <div class="mt-1 flex items-center justify-between rounded-md bg-red-500/10 px-2 py-1.5 text-[11px]">
              <span class="text-red-300">Delete <b>{p.name}</b>?</span>
              <div class="flex gap-1">
                <button
                  type="button"
                  class="rounded px-2 py-0.5 text-zinc-400 hover:text-zinc-100"
                  onclick={() => (pendingDeleteId = null)}
                >
                  Cancel
                </button>
                <button
                  type="button"
                  class="rounded border border-red-500/30 bg-red-500/20 px-2 py-0.5 text-red-200 hover:bg-red-500/30"
                  onclick={() => deleteProject(p.id)}
                >
                  Delete
                </button>
              </div>
            </div>
          {/if}
        </article>
      {/each}
    {/if}
  </section>

  <footer class="flex items-center justify-end">
    <button
      type="button"
      class="rounded-lg border border-white/16 bg-white/8 px-3 py-1.5 text-xs text-zinc-100 hover:bg-white/12"
      onclick={() => getCurrentWindow().close()}
    >
      Done
    </button>
  </footer>
</main>

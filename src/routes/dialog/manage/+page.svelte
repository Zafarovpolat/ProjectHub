<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import type { ProjectView } from "$lib/types";
  import { Trash2, ChevronUp, ChevronDown, Hash } from "lucide-svelte";

  let projects = $state<ProjectView[]>([]);
  let loading = $state(true);
  let errorMessage = $state<string | null>(null);
  let pendingDeleteId = $state<string | null>(null);

  async function refresh() {
    try {
      const list = await invoke<ProjectView[]>("list_projects");
      projects = list;
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
    return () => window.removeEventListener("keydown", onKey);
  });

  async function deleteProject(id: string) {
    try {
      await invoke<boolean>("delete_project", { id });
      await emit("project:changed");
      pendingDeleteId = null;
      await refresh();
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
      await invoke("reorder_projects", { order: next.map((p) => p.id) });
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
      await invoke("reorder_projects", { order: next.map((p) => p.id) });
      await emit("project:changed");
    } catch (e) {
      errorMessage = String(e);
      await refresh();
    }
  }

  async function setHotkey(id: string, value: number | null) {
    try {
      await invoke<ProjectView>("update_project", {
        input: { id, hotkey_index: value },
      });
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
      Reorder, assign hotkeys, or remove projects.
    </p>
  </header>

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
              <div class="text-[11px] text-zinc-500">
                {p.windows.length} window{p.windows.length === 1 ? "" : "s"}
              </div>
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

          <div class="flex items-center gap-2 text-[11px] text-zinc-400">
            <Hash size={12} class="text-zinc-500" />
            <span>Hotkey:</span>
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
                title="Clear hotkey"
              >
                clear
              </button>
            </div>
          </div>

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

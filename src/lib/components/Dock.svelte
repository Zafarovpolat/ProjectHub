<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { Plus, Settings } from "lucide-svelte";

  import ProjectCard from "./ProjectCard.svelte";
  import AddProjectDialog from "./AddProjectDialog.svelte";
  import { activate, dock, refreshProjects } from "$lib/store.svelte";
  import type { ActivationResult } from "$lib/types";

  let addOpen = $state(false);
  let lastSwitchedAt = $state<number | null>(null);

  onMount(() => {
    void refreshProjects();

    // Refresh "X min ago" labels every 20s.
    const tick = setInterval(() => {
      lastSwitchedAt = Date.now();
    }, 20_000);

    // Hotkey activations from Rust come through this event.
    const unlistenP = listen<ActivationResult>("project:activated", (e) => {
      dock.activeId = e.payload.project.id;
      void refreshProjects();
    });

    return () => {
      clearInterval(tick);
      void unlistenP.then((fn) => fn());
    };
  });

  async function onCardActivate(id: string) {
    try {
      await activate(id);
    } catch {
      /* error captured in store */
    }
  }
</script>

<aside
  class="dock-surface relative flex h-screen w-full flex-col overflow-hidden rounded-2xl"
>
  <!-- Drag handle / title -->
  <header
    data-tauri-drag-region
    class="flex items-center justify-between px-4 py-3 select-none"
  >
    <div data-tauri-drag-region class="flex items-center gap-2">
      <span
        class="h-2 w-2 rounded-full bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.6)]"
        aria-hidden="true"
      ></span>
      <h1 class="text-[11px] font-semibold uppercase tracking-[0.18em] text-zinc-300">
        ProjectHub
      </h1>
    </div>
    <button
      type="button"
      class="text-zinc-500 hover:text-zinc-300"
      aria-label="Settings (coming soon)"
      title="Settings — coming in v0.2"
    >
      <Settings size={14} />
    </button>
  </header>

  <!-- Project list -->
  <div class="flex-1 overflow-y-auto px-3 pb-3">
    {#if dock.loading && dock.projects.length === 0}
      <p class="px-2 py-1 text-xs text-zinc-500">Loading projects…</p>
    {:else if dock.projects.length === 0}
      <div class="mt-4 flex flex-col items-center gap-3 px-3 text-center">
        <p class="text-xs leading-relaxed text-zinc-400">
          No projects yet.<br />
          Add your first one to bundle a set of open windows.
        </p>
        <button
          type="button"
          class="rounded-lg border border-white/12 bg-white/8 px-3 py-1.5 text-xs font-medium text-zinc-100 hover:bg-white/12"
          onclick={() => (addOpen = true)}
        >
          Add project
        </button>
      </div>
    {:else}
      <div class="flex flex-col gap-1.5">
        {#each dock.projects as project (project.id)}
          <ProjectCard
            {project}
            active={dock.activeId === project.id}
            onactivate={() => onCardActivate(project.id)}
          />
        {/each}
      </div>
    {/if}

    {#if dock.error}
      <p class="mt-3 rounded-md border border-red-500/30 bg-red-500/10 px-2 py-1.5 text-[11px] text-red-300">
        {dock.error}
      </p>
    {/if}
  </div>

  <!-- Add button -->
  {#if dock.projects.length > 0}
    <div class="px-3 pb-3">
      <button
        type="button"
        class="ghost-btn flex w-full items-center justify-center gap-1.5 rounded-[var(--radius-card)] py-3 text-xs"
        onclick={() => (addOpen = true)}
        aria-label="Add project"
      >
        <Plus size={14} />
        <span>Add project</span>
      </button>
    </div>
  {/if}

  <!-- Footer -->
  <footer
    class="border-t border-white/4 px-4 py-2 text-[10px] text-zinc-500 select-none flex items-center justify-between"
  >
    <span>{dock.projects.length} active</span>
    <span class="font-mono text-zinc-600">⌃⌥Space</span>
  </footer>
</aside>

<AddProjectDialog bind:open={addOpen} onclose={() => (addOpen = false)} />

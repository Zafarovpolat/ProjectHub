<script lang="ts">
  import type { EnumeratedWindow } from "$lib/types";
  import { api } from "$lib/api";

  interface Props {
    selected: number[];
    onchange: (hwnds: number[]) => void;
  }

  let { selected = $bindable([]), onchange }: Props = $props();

  let windows = $state<EnumeratedWindow[]>([]);
  let loading = $state(false);
  let filter = $state("");

  $effect(() => {
    void refresh();
  });

  async function refresh() {
    loading = true;
    try {
      windows = await api.listOpenWindows();
    } finally {
      loading = false;
    }
  }

  function toggle(hwnd: number) {
    const next = selected.includes(hwnd)
      ? selected.filter((h) => h !== hwnd)
      : [...selected, hwnd];
    selected = next;
    onchange(next);
  }

  function exeName(path: string): string {
    if (!path) return "";
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] ?? path;
  }

  const filtered = $derived(
    filter.trim().length === 0
      ? windows
      : windows.filter((w) =>
          (w.title + " " + w.exe_path)
            .toLowerCase()
            .includes(filter.trim().toLowerCase())
        )
  );
</script>

<div class="flex h-full min-h-0 flex-col gap-2">
  <div class="flex items-center justify-between gap-2">
    <input
      type="text"
      placeholder="Filter windows…"
      bind:value={filter}
      class="min-w-0 flex-1 rounded-lg border border-white/10 bg-white/4 px-3 py-1.5 text-xs text-zinc-100 placeholder:text-zinc-500 focus:border-white/24 focus:outline-none"
    />
    <button
      type="button"
      onclick={refresh}
      class="shrink-0 rounded-lg border border-white/10 bg-white/4 px-2.5 py-1.5 text-xs text-zinc-200 hover:bg-white/8"
    >
      Refresh
    </button>
  </div>

  {#if loading}
    <p class="text-xs text-zinc-500">Scanning windows…</p>
  {:else if filtered.length === 0}
    <p class="text-xs text-zinc-500">No matching windows.</p>
  {:else}
    <ul class="flex flex-1 min-h-0 flex-col gap-1 overflow-y-auto pr-1">
      {#each filtered as w (w.hwnd)}
        {@const checked = selected.includes(w.hwnd)}
        <li>
          <label
            class="flex items-center gap-2 rounded-lg border border-transparent px-2 py-1.5 text-xs hover:border-white/10 hover:bg-white/4"
            class:active-row={checked}
          >
            <input
              type="checkbox"
              {checked}
              onchange={() => toggle(w.hwnd)}
              class="h-3.5 w-3.5 accent-indigo-400"
            />
            <span class="flex min-w-0 flex-col">
              <span class="truncate text-zinc-200">{w.title}</span>
              <span class="meta-text truncate font-mono text-[10px]">
                {exeName(w.exe_path)}{w.minimized ? " · min" : ""}
              </span>
            </span>
          </label>
        </li>
      {/each}
    </ul>
  {/if}
</div>

<style>
  .active-row {
    border-color: rgba(255, 255, 255, 0.12) !important;
    background: rgba(255, 255, 255, 0.06) !important;
  }
</style>

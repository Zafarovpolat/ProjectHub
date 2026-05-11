<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { emit } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";

  import WindowPicker from "$lib/components/WindowPicker.svelte";

  let name = $state("");
  let palette = $state<string[]>([]);
  let pickedColor = $state<string | null>(null);
  let selectedHwnds = $state<number[]>([]);
  let submitting = $state(false);
  let errorMessage = $state<string | null>(null);

  onMount(() => {
    void invoke<string[]>("palette_colors").then((cols) => {
      palette = cols;
    });

    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") void close();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  async function close() {
    await getCurrentWindow().close();
  }

  async function submit(e: Event) {
    e.preventDefault();
    if (!name.trim()) {
      errorMessage = "Name is required";
      return;
    }
    submitting = true;
    errorMessage = null;
    try {
      await invoke("create_project", {
        input: {
          name: name.trim(),
          window_hwnds: selectedHwnds,
          color: pickedColor,
        },
      });
      await emit("project:created");
      await close();
    } catch (err) {
      errorMessage = String(err);
      submitting = false;
    }
  }
</script>

<svelte:head>
  <title>New project — ProjectHub</title>
</svelte:head>

<main
  class="flex h-screen w-screen flex-col gap-5 bg-zinc-950 p-6 text-zinc-100"
>
  <header>
    <h1 class="text-base font-semibold">New project</h1>
    <p class="mt-0.5 text-[11px] text-zinc-500">
      Bundle a set of open windows under a shortcut.
    </p>
  </header>

  <form class="flex flex-1 min-h-0 flex-col gap-5" onsubmit={submit}>
    <label class="flex flex-col gap-1.5">
      <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
        Name
      </span>
      <input
        type="text"
        bind:value={name}
        placeholder="Devin Task, Клиент X…"
        class="rounded-lg border border-white/10 bg-white/4 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:border-white/24 focus:outline-none"
      />
    </label>

    <div class="flex flex-col gap-1.5">
      <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
        Accent
      </span>
      <div class="flex flex-wrap gap-2">
        {#each palette as col}
          <button
            type="button"
            aria-label="Use accent {col}"
            class="h-6 w-6 rounded-full border transition-transform"
            class:scale-110={pickedColor === col}
            style:background-color={col}
            style:border-color={pickedColor === col ? "white" : "rgba(255,255,255,0.1)"}
            onclick={() => (pickedColor = col)}
          ></button>
        {/each}
        <button
          type="button"
          class="h-6 px-3 rounded-full border border-dashed border-white/16 text-[11px] text-zinc-400 hover:text-zinc-100"
          onclick={() => (pickedColor = null)}
        >
          auto
        </button>
      </div>
    </div>

    <div class="flex flex-1 min-h-0 flex-col gap-1.5">
      <div class="flex items-center justify-between">
        <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
          Attach windows
        </span>
        <span class="text-[11px] text-zinc-500">{selectedHwnds.length} selected</span>
      </div>
      <div class="flex-1 min-h-0">
        <WindowPicker
          bind:selected={selectedHwnds}
          onchange={(h) => (selectedHwnds = h)}
        />
      </div>
    </div>

    {#if errorMessage}
      <p class="text-xs text-red-400">{errorMessage}</p>
    {/if}

    <footer class="flex items-center justify-end gap-2">
      <button
        type="button"
        class="rounded-lg px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-100"
        onclick={close}
      >
        Cancel
      </button>
      <button
        type="submit"
        disabled={submitting}
        class="rounded-lg border border-white/16 bg-white/8 px-3 py-1.5 text-xs font-medium text-zinc-100 hover:bg-white/12 disabled:opacity-50"
      >
        {submitting ? "Saving…" : "Create"}
      </button>
    </footer>
  </form>
</main>

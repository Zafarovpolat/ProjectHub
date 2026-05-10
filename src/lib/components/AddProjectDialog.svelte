<script lang="ts">
  import WindowPicker from "./WindowPicker.svelte";
  import { api } from "$lib/api";
  import { refreshProjects } from "$lib/store.svelte";

  interface Props {
    open: boolean;
    onclose: () => void;
  }

  let { open = $bindable(false), onclose }: Props = $props();

  let name = $state("");
  let selectedHwnds = $state<number[]>([]);
  let palette = $state<string[]>([]);
  let pickedColor = $state<string | null>(null);
  let submitting = $state(false);
  let errorMessage = $state<string | null>(null);

  $effect(() => {
    if (open && palette.length === 0) {
      void api.paletteColors().then((cols) => {
        palette = cols;
      });
    }
    if (!open) {
      // Reset on close.
      name = "";
      selectedHwnds = [];
      pickedColor = null;
      errorMessage = null;
    }
  });

  async function submit(event: Event) {
    event.preventDefault();
    if (!name.trim()) {
      errorMessage = "Name is required";
      return;
    }
    submitting = true;
    errorMessage = null;
    try {
      await api.createProject({
        name: name.trim(),
        window_hwnds: selectedHwnds,
        color: pickedColor,
      });
      await refreshProjects();
      onclose();
    } catch (err) {
      errorMessage = String(err);
    } finally {
      submitting = false;
    }
  }
</script>

{#if open}
  <!-- svelte-ignore a11y_click_events_have_key_events -->
  <!-- svelte-ignore a11y_no_static_element_interactions -->
  <div
    class="fixed inset-0 z-50 flex items-center justify-center bg-black/50 backdrop-blur-sm"
    onclick={onclose}
    onkeydown={(e) => e.key === "Escape" && onclose()}
    role="presentation"
    tabindex="-1"
  >
    <!-- svelte-ignore a11y_click_events_have_key_events -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <form
      class="dock-surface w-[420px] max-w-[92vw] rounded-2xl p-5 flex flex-col gap-4"
      onclick={(e) => e.stopPropagation()}
      onsubmit={submit}
      aria-label="Add project"
    >
      <header class="flex items-center justify-between">
        <h2 class="text-sm font-semibold text-zinc-100">New project</h2>
        <button
          type="button"
          class="text-zinc-400 hover:text-zinc-200"
          onclick={onclose}
          aria-label="Close"
        >
          ✕
        </button>
      </header>

      <label class="flex flex-col gap-1.5">
        <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
          Name
        </span>
        <input
          type="text"
          bind:value={name}
          placeholder="Devin Task, Клиент X…"
          class="rounded-lg border border-white/8 bg-white/4 px-3 py-2 text-sm text-zinc-100 placeholder:text-zinc-500 focus:border-white/16 focus:outline-none"
        />
      </label>

      <div class="flex flex-col gap-1.5">
        <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
          Accent
        </span>
        <div class="flex flex-wrap gap-1.5">
          {#each palette as col}
            <button
              type="button"
              aria-label="Use accent {col}"
              class="h-5 w-5 rounded-full border transition-transform"
              class:scale-110={pickedColor === col}
              style:background-color={col}
              style:border-color={pickedColor === col ? "white" : "rgba(255,255,255,0.1)"}
              onclick={() => (pickedColor = col)}
            ></button>
          {/each}
          <button
            type="button"
            class="h-5 px-2 rounded-full border border-dashed border-white/12 text-[10px] text-zinc-400 hover:text-zinc-200"
            onclick={() => (pickedColor = null)}
          >
            auto
          </button>
        </div>
      </div>

      <div class="flex flex-col gap-1.5">
        <div class="flex items-center justify-between">
          <span class="text-[11px] font-medium uppercase tracking-wide text-zinc-500">
            Attach windows
          </span>
          <span class="text-[11px] text-zinc-500">{selectedHwnds.length} selected</span>
        </div>
        <WindowPicker
          bind:selected={selectedHwnds}
          onchange={(h) => (selectedHwnds = h)}
        />
      </div>

      {#if errorMessage}
        <p class="text-xs text-red-400">{errorMessage}</p>
      {/if}

      <footer class="flex items-center justify-end gap-2">
        <button
          type="button"
          class="rounded-lg px-3 py-1.5 text-xs text-zinc-400 hover:text-zinc-200"
          onclick={onclose}
        >
          Cancel
        </button>
        <button
          type="submit"
          disabled={submitting}
          class="rounded-lg border border-white/12 bg-white/8 px-3 py-1.5 text-xs font-medium text-zinc-100 hover:bg-white/12 disabled:opacity-50"
        >
          {submitting ? "Saving…" : "Create"}
        </button>
      </footer>
    </form>
  </div>
{/if}

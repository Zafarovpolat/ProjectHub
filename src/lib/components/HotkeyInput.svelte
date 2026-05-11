<script lang="ts">
  import { api } from "$lib/api";

  interface Props {
    /// Current combo (e.g. `"Ctrl+Alt+KeyM"`) or `null` for no binding.
    value: string | null;
    /// Placeholder text shown when `value` is null and input is idle.
    placeholder?: string;
    /// Called whenever the user picks a new combo. The combo has already
    /// been validated against the backend; pass `null` to clear.
    onchange?: (combo: string | null) => void;
    /// Optional label rendered above the input (use for accessibility).
    label?: string;
    /// Whether to render the "Reset to default" affordance.
    showReset?: boolean;
  }

  let {
    value,
    placeholder = "Click and press a combo…",
    onchange,
    label,
    showReset = false,
  }: Props = $props();

  let listening = $state(false);
  let error = $state<string | null>(null);

  /// Convert a `KeyboardEvent` into the "Ctrl+Alt+Foo" combo string the
  /// backend expects. We intentionally match `tauri-plugin-global-shortcut`'s
  /// `Code` debug names (`KeyM`, `Digit5`, `Space`, `F1`, ...) so the
  /// round-trip "press → store → re-register" produces a working hotkey.
  function comboFromEvent(e: KeyboardEvent): string | null {
    const parts: string[] = [];
    if (e.ctrlKey) parts.push("Ctrl");
    if (e.altKey) parts.push("Alt");
    if (e.shiftKey) parts.push("Shift");
    if (e.metaKey) parts.push("Super");
    // `event.code` follows the same naming as `Code` in the backend
    // ("KeyM", "Digit5", "F1", "Space", "ArrowLeft"...). We just need to
    // exclude the bare modifier keys themselves so "Ctrl" alone doesn't
    // resolve to "Ctrl+ControlLeft".
    const modifierCodes = new Set([
      "ControlLeft",
      "ControlRight",
      "AltLeft",
      "AltRight",
      "ShiftLeft",
      "ShiftRight",
      "MetaLeft",
      "MetaRight",
    ]);
    if (modifierCodes.has(e.code)) return null;
    parts.push(e.code);
    return parts.join("+");
  }

  async function commit(combo: string) {
    try {
      const canonical = await api.validateHotkeyCombo(combo);
      error = null;
      listening = false;
      onchange?.(canonical);
    } catch (err) {
      error = String(err);
    }
  }

  function onkeydown(e: KeyboardEvent) {
    if (!listening) return;
    if (e.key === "Escape") {
      e.preventDefault();
      listening = false;
      error = null;
      return;
    }
    e.preventDefault();
    e.stopPropagation();
    const combo = comboFromEvent(e);
    if (!combo) return; // bare modifier — keep waiting
    void commit(combo);
  }

  function reset() {
    error = null;
    onchange?.(null);
  }

  function prettify(combo: string): string {
    return combo
      .split("+")
      .map((p) => {
        const t = p.trim();
        if (t.toLowerCase().startsWith("digit")) return t.slice(5);
        if (t.toLowerCase().startsWith("key")) return t.slice(3);
        return t;
      })
      .join(" + ");
  }
</script>

<div class="flex flex-col gap-1">
  {#if label}
    <span class="text-[11px] uppercase tracking-wide text-zinc-500">{label}</span>
  {/if}
  <div class="flex items-center gap-2">
    <button
      type="button"
      onclick={() => {
        listening = !listening;
        error = null;
      }}
      {onkeydown}
      class="combo-button"
      class:listening
      aria-label="Set hotkey"
    >
      {#if listening}
        <span class="italic text-zinc-400">Press combo… (Esc to cancel)</span>
      {:else if value}
        <span class="text-zinc-100">{prettify(value)}</span>
      {:else}
        <span class="text-zinc-500">{placeholder}</span>
      {/if}
    </button>
    {#if showReset && value}
      <button
        type="button"
        onclick={reset}
        class="reset-button"
        aria-label="Clear hotkey"
      >
        Reset
      </button>
    {/if}
  </div>
  {#if error}
    <span class="text-[11px] text-rose-400">{error}</span>
  {/if}
</div>

<style>
  .combo-button {
    min-width: 8rem;
    border-radius: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    padding: 0.375rem 0.625rem;
    font-size: 0.75rem;
    text-align: left;
  }
  .combo-button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
  .combo-button.listening {
    border-color: rgba(129, 140, 248, 0.6);
    background: rgba(129, 140, 248, 0.12);
  }
  .reset-button {
    border-radius: 0.5rem;
    border: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(255, 255, 255, 0.04);
    padding: 0.375rem 0.625rem;
    font-size: 0.7rem;
    color: rgb(212 212 216);
  }
  .reset-button:hover {
    background: rgba(255, 255, 255, 0.08);
  }
</style>

<script lang="ts">
  import type { ProjectView } from "$lib/types";
  import { formatRelative, now } from "$lib/time.svelte";

  interface Props {
    project: ProjectView;
    active?: boolean;
    onactivate: () => void;
  }

  let { project, active = false, onactivate }: Props = $props();

  const accent = $derived(project.color);

  /// Pretty-printed hotkey hint shown on the right of the card. Prefer
  /// the user's custom combo when set; otherwise fall back to the
  /// implicit slot-based combo so the legacy display keeps working.
  const hotkeyText = $derived(
    project.hotkey_combo
      ? prettifyCombo(project.hotkey_combo)
      : project.hotkey_index
        ? `Ctrl Alt ${project.hotkey_index}`
        : null
  );

  const windowCount = $derived(project.windows.length);

  /// `now()` is reactive — re-running this `$derived` once per ~30 s so
  /// the "5m ago" label stays fresh without page reload.
  const subtitle = $derived(
    `${windowCount} window${windowCount === 1 ? "" : "s"} · ${
      active ? "active" : formatRelative(project.last_activated_at, now())
    }`
  );

  /// Convert a stored combo like "Ctrl+Alt+Digit1" or "Ctrl+Shift+KeyM"
  /// into the same loose layout the legacy slot pill uses (space-sep,
  /// stripped key-prefixes). Display only — the canonical combo is
  /// what gets persisted.
  function prettifyCombo(combo: string): string {
    return combo
      .split("+")
      .map((part) => {
        const trimmed = part.trim();
        if (trimmed.toLowerCase().startsWith("digit")) return trimmed.slice(5);
        if (trimmed.toLowerCase().startsWith("key")) return trimmed.slice(3);
        if (trimmed.toLowerCase() === "ctrl" || trimmed.toLowerCase() === "control")
          return "Ctrl";
        if (trimmed.toLowerCase() === "alt") return "Alt";
        if (trimmed.toLowerCase() === "shift") return "Shift";
        if (trimmed.toLowerCase() === "win" || trimmed.toLowerCase() === "super")
          return "Win";
        return trimmed;
      })
      .join(" ");
  }
</script>

<button
  type="button"
  class="card-row group w-full text-left"
  class:active
  style="--accent-color: {accent};"
  onclick={onactivate}
  aria-label="Activate {project.name}"
>
  <span class="accent-rail" aria-hidden="true"></span>

  <span class="icon-tile" aria-hidden="true">{project.initials}</span>

  <span class="min-w-0 flex flex-col">
    <span class="truncate text-[13px] font-medium text-zinc-100">
      {project.name}
    </span>
    <span class="meta-text truncate">{subtitle}</span>
  </span>

  {#if hotkeyText}
    <span class="hotkey-pill">{hotkeyText}</span>
  {/if}
</button>

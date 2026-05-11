<script lang="ts">
  import type { ProjectView } from "$lib/types";

  interface Props {
    project: ProjectView;
    active?: boolean;
    onactivate: () => void;
  }

  let { project, active = false, onactivate }: Props = $props();

  const accent = $derived(project.color);

  function formatRelative(ts: string | null): string {
    if (!ts) return "never";
    const seconds = Math.max(0, Math.round((Date.now() - new Date(ts).getTime()) / 1000));
    if (seconds < 30) return "just now";
    if (seconds < 60) return `${seconds}s ago`;
    const minutes = Math.round(seconds / 60);
    if (minutes < 60) return `${minutes}m ago`;
    const hours = Math.round(minutes / 60);
    if (hours < 24) return `${hours}h ago`;
    const days = Math.round(hours / 24);
    return `${days}d ago`;
  }

  const hotkeyText = $derived(
    project.hotkey_index ? `Ctrl Alt ${project.hotkey_index}` : null
  );
  const windowCount = $derived(project.windows.length);
  const subtitle = $derived(
    active
      ? `${windowCount} window${windowCount === 1 ? "" : "s"} · active`
      : `${windowCount} window${windowCount === 1 ? "" : "s"} · ${formatRelative(project.last_activated_at)}`
  );
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

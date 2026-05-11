<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";

  let { children } = $props();

  onMount(() => {
    // Disable native WebView context menu (print / inspect / etc.) — looks
    // unprofessional in a desktop app and the items don't even do useful
    // things inside Tauri's webview. We can build our own custom menu in
    // v0.2 if we want right-click affordances on project cards.
    const onContextMenu = (e: MouseEvent) => e.preventDefault();
    // Block common WebView2 shortcuts that "leak" browser UI in release.
    const onKey = (e: KeyboardEvent) => {
      if (
        e.key === "F5" ||
        (e.ctrlKey && (e.key === "r" || e.key === "R")) ||
        (e.ctrlKey && (e.key === "p" || e.key === "P"))
      ) {
        e.preventDefault();
      }
    };
    document.addEventListener("contextmenu", onContextMenu);
    document.addEventListener("keydown", onKey);
    return () => {
      document.removeEventListener("contextmenu", onContextMenu);
      document.removeEventListener("keydown", onKey);
    };
  });
</script>

{@render children?.()}

// Shared reactive clock — anything that wants to re-render on a regular
// cadence (e.g. "5m ago" labels) reads `now()` and is automatically
// rerun every `TICK_MS` ms.
//
// We deliberately use a single shared rune instead of per-component
// `setInterval` so that opening 9 project cards doesn't spin up 9
// timers. There are only ever a handful of consumers, and the rune is
// cheap to read.

const TICK_MS = 30_000;

let _now = $state(Date.now());

if (typeof window !== "undefined") {
  setInterval(() => {
    _now = Date.now();
  }, TICK_MS);
}

/// Reactive "now" — reads as the current ms-since-epoch and re-runs
/// dependent `$derived` expressions every `TICK_MS`.
export function now(): number {
  return _now;
}

export function formatRelative(ts: string | null, reference: number = now()): string {
  if (!ts) return "never";
  const seconds = Math.max(0, Math.round((reference - new Date(ts).getTime()) / 1000));
  if (seconds < 30) return "just now";
  if (seconds < 60) return `${seconds}s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 60) return `${minutes}m ago`;
  const hours = Math.round(minutes / 60);
  if (hours < 24) return `${hours}h ago`;
  const days = Math.round(hours / 24);
  return `${days}d ago`;
}

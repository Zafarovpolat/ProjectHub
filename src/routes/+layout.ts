// Tauri doesn't have a Node.js server to do proper SSR, so we use
// adapter-static with a fallback to index.html (SPA mode).
// See: https://svelte.dev/docs/kit/single-page-apps
// See: https://v2.tauri.app/start/frontend/sveltekit/
export const ssr = false;
export const prerender = true;
// Emit each route as `<route>/index.html` so Tauri can serve dialog windows
// via URLs without `.html` suffixes (e.g. `/dialog/add-project`).
export const trailingSlash = "always";

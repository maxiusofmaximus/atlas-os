// OpenCode OS — SvelteKit root layout config.
// SSR must be disabled for Tauri (RFC 25 §3.3).
// Prerendering is also off — the HUD is a live CSR app.
export const ssr = false;
export const prerender = false;
export const trailingSlash = 'ignore';

// Atlas OS — SvelteKit config (Tauri 2 CSR mode)
// See RFC 25 §3.3 — adapter-static, Svelte 5 runes, no SSR.
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  compilerOptions: {
    runes: true,
  },
  kit: {
    adapter: adapter({
      pages: 'build',
      assets: 'build',
      fallback: 'index.html',
      precompress: false,
      strict: false,
    }),
    // Tauri environment variables injected by tauri-build
    env: { publicPrefix: 'TAURI_' },
    // No SSR in Tauri webview (RFC 25 §3.3).
    // +layout.ts sets `ssr = false` programmatically.
    alias: {
      $lib: './src/lib',
      $components: './src/lib/components',
      $stores: './src/lib/stores',
    },
  },
};

export default config;

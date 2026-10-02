// Atlas OS — Vite config
// See RFC 25 §3.3 — SvelteKit + Tauri 2 internal host.
import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vitest/config';
import { readFileSync } from 'node:fs';

const pkg = JSON.parse(readFileSync(new URL('./package.json', import.meta.url), 'utf-8'));
process.env.VITE_OC_VERSION ??= pkg.version;

// Tauri 2 injects env vars at build time via tauri-build.
// Hot reload inside Tauri dev webview requires strictPort + fixed host.
const host = process.env.TAURI_DEV_HOST ?? 'localhost';
const port = Number(process.env.TAURI_DEV_PORT ?? 5173);

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    host,
    port,
    strictPort: true,
    fs: { strict: false },
    watch: {
      // src-tauri/target holds large build artifacts locked by the
      // running Rust binary; vite would otherwise try to watch exe/dll
      // files and fail with EBUSY on Windows.
      ignored: ['**/src-tauri/target/**', '**/.svelte-kit/**', '**/build/**'],
    },
  },
  envPrefix: ['TAURI_', 'VITE_'],
  build: {
    target: 'esnext',
    minify: 'esbuild',
    sourcemap: false,
  },
  test: {
    globals: true,
    environment: 'jsdom',
    include: ['src/**/*.{test,spec}.{ts,js}'],
  },
});

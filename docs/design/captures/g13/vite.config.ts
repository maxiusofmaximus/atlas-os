import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { fileURLToPath } from 'node:url';

const r = (p: string) => fileURLToPath(new URL(p, import.meta.url));

export default defineConfig({
  root: r('.'),
  base: './',
  plugins: [svelte()],
  resolve: {
    alias: {
      '$stores/hud': r('./hud-stub.ts'),
    },
  },
  build: { outDir: r('dist'), emptyOutDir: true },
});

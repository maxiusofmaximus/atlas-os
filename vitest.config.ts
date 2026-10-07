import { defineConfig, mergeConfig } from 'vitest/config';
import viteConfig from './vite.config';

export default mergeConfig(
  viteConfig,
  defineConfig({
    ssr: {
      resolve: {
        conditions: ['browser', 'module', 'import', 'default'],
      },
    },
  }),
);

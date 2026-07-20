// OpenCode OS — load HUD URL via Tauri invoke (or fallback to env).
// Phase 0: the HUD URL only becomes known after the Rust core binds the
// axum server to an ephemeral port. We ask the Rust side for it. If not
// running under Tauri (e.g. `pnpm dev` without the desktop shell), the
// URL is undefined and the HUD store does not connect.
import type { PageLoad } from './$types';

export const load: PageLoad = async () => {
  let hudUrl: string | undefined;
  if (typeof window !== 'undefined') {
    try {
      const tauri = await import('@tauri-apps/api/core');
      hudUrl = await tauri.invoke<string>('hud_url').catch(() => undefined);
    } catch {
      // Not running inside Tauri; HUD store will not connect.
    }
  }
  return { hudUrl };
};

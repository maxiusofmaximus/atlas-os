// OpenCode OS — SvelteKit ambient types.
// See https://kit.svelte.dev/docs/types#app

declare global {
  namespace App {
    // interface Error {}
    // interface Locals {}
    // interface PageData {}
    // interface PageState {}
    // interface Platform {}
  }
  interface ImportMetaEnv {
    readonly VITE_OC_VERSION?: string;
    readonly VITE_HUD_URL?: string;
    readonly TAURI_PLATFORM?: string;
    readonly TAURI_PLATFORM_VERSION?: string;
    readonly TAURI_PLATFORM_ARCH?: string;
  }
  interface ImportMeta {
    readonly env: ImportMetaEnv;
  }
}

export {};

<script lang="ts">
  // OpenCode OS — HUD Mission Control landing page (Phase 0).
  // See RFC 24 for the full design. Phase 0 only shows:
  //  - HUD URL (where the axum WS server is bound)
  //  - Latest journal entries (live stream from kernel bus)
  //  - Profile + version info
  import { onMount } from 'svelte';
  import { hud } from '$stores/hud';
  import type { PageData } from './$types';

  const { data } = $props<{ data: PageData }>();

  onMount(() => {
    if (data.hudUrl) {
      hud.connect(data.hudUrl);
    }
    return () => hud.disconnect();
  });
</script>

<main>
  <header>
    <h1>OpenCode OS</h1>
    <span class="version">v{import.meta.env.VITE_OC_VERSION ?? '0.1.0'}</span>
  </header>

  <section class="hud-health">
    <h2>HUD Mission Control</h2>
    <p>
      WS status:
      <span class="status" data-state={$hud.connected ? 'online' : 'offline'}>
        {$hud.connected ? 'connected' : 'disconnected'}
      </span>
    </p>
    <p class="hud-url">
      URL:
      <code>{$hud.url || data.hudUrl || 'waiting…'}</code>
    </p>
  </section>

  <section class="journal">
    <h2>Journal tail</h2>
    {#if $hud.events.length === 0}
      <p class="empty">No events yet. Try: <code>opencode mission new "hello world"</code>.</p>
    {:else}
      <ul>
        {#each $hud.events as evt (evt.id)}
          <li>
            <time>{evt.ts}</time>
            <span class="kind">{evt.kind}</span>
            <span class="payload">{JSON.stringify(evt.payload).slice(0, 120)}</span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>
</main>

<style>
  :global(html) {
    font-family:
      'Inter',
      system-ui,
      -apple-system,
      sans-serif;
    background: #0d1117;
    color: #c9d1d9;
  }
  main {
    max-width: 1100px;
    margin: 0 auto;
    padding: 1.5rem;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  h1 {
    font-size: 1.6rem;
    margin: 0;
  }
  .version {
    opacity: 0.6;
    font-family: 'Fira Code', monospace;
  }
  section {
    margin-bottom: 2rem;
    border: 1px solid #30363d;
    background: #161b22;
    border-radius: 8px;
    padding: 1rem 1.25rem;
  }
  h2 {
    font-size: 1.1rem;
    margin-top: 0;
    color: #58a6ff;
  }
  .status[data-state='online'] {
    color: #3fb950;
  }
  .status[data-state='offline'] {
    color: #f85149;
  }
  code {
    background: #21262d;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-family: 'Fira Code', 'JetBrains Mono', monospace;
  }
  .journal ul {
    list-style: none;
    padding-left: 0;
    margin: 0;
  }
  .journal li {
    display: grid;
    grid-template-columns: 14rem 14rem 1fr;
    gap: 1rem;
    font-family: 'Fira Code', monospace;
    font-size: 0.85rem;
    padding: 0.35rem 0;
    border-bottom: 1px solid #21262d;
  }
  .journal time {
    opacity: 0.55;
  }
  .kind {
    color: #79c0ff;
  }
  .payload {
    opacity: 0.9;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    opacity: 0.6;
  }
</style>

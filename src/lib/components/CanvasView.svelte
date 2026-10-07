<!-- RFC 65 §3 (P2) — CanvasView: the persisted MissionGraph for a chosen
     mission. Reuses `<GraphView>` (RFC 28 §C item 7 / `GET /graph/:id`) with a
     mission picker fed by `/tail/missions`. Read-only. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchMissions, type MissionRow } from '$stores/hud';
  import GraphView from '$lib/components/GraphView.svelte';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let missions = $state<MissionRow[]>([]);
  let selected = $state<string>('');
  let error = $state<string | null>(null);

  async function load(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      missions = await fetchMissions(hudUrl);
      if (!selected && missions[0]) selected = missions[0].id;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="canvas">
  <header>
    <h3>Canvas — mission graph</h3>
    <select bind:value={selected} disabled={missions.length === 0} aria-label="Mission">
      {#if missions.length === 0}
        <option value="">no missions</option>
      {/if}
      {#each missions as m (m.id)}
        <option value={m.id}>{m.label} · {m.id.slice(0, 8)}</option>
      {/each}
    </select>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if selected}
    <GraphView {hudUrl} missionId={selected} />
  {:else}
    <p class="empty">
      No mission selected. Create one with <code>atlas mission new "&lt;prompt&gt;"</code>.
    </p>
  {/if}
</section>

<style>
  .canvas {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .canvas header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
  }
  .canvas h3 {
    margin: 0;
    font-size: 1rem;
  }
  select {
    background: var(--a-bg);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    font-size: 0.78rem;
    max-width: 55%;
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.82rem;
    margin: 0;
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
</style>

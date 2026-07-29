<!-- RFC 28 §C item 7 — HUD graph view component.
     Renders the persisted `MissionGraph` for a single mission_id
     (fetched live from `GET /graph/:mission_id`) as a flat list of
     nodes tagged by provenance colour, plus a compact edge table for
     the RFC 19 DFA decorations. Intentionally non-interactive: the
     Planner owns the write side; this panel is read-only telemetry
     that operators can open from any mission card.

     Phase 1.5c ships this component with a stub harness (manual
     fetch+render). Phase 2 will wire it onto the Kernel Bus to
     re-fetch when a `MissionGraphUpdated` event fires. -->

<script lang="ts">
  import type { MissionGraph, Provenance, NodeKind } from '$stores/hud';
  import { fetchGraph } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
    missionId: string;
  }

  let { hudUrl, missionId }: Props = $props();

  let graph = $state<MissionGraph | null>(null);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function load(): Promise<void> {
    if (!hudUrl) {
      error = 'HUD URL not available';
      return;
    }
    loading = true;
    error = null;
    try {
      graph = await fetchGraph(hudUrl, missionId);
    } catch (e) {
      graph = null;
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    // Re-run when missionId or hudUrl flips. Using $effect so the
    // initial mount also triggers one fetch (the deps are read).
    if (hudUrl && missionId) {
      void load();
    }
  });

  function provenanceColor(p: Provenance): string {
    switch (p) {
      case 'EXTRACTED':
        return 'color: var(--graph-prov-extracted, #0a7);';
      case 'INFERRED':
        return 'color: var(--graph-prov-inferred, #07a);';
      case 'AMBIGUOUS':
        return 'color: var(--graph-prov-ambiguous, #b71);';
    }
  }

  function kindColor(k: NodeKind): string {
    switch (k) {
      case 'mission':
        return 'background: var(--graph-kind-mission, #1f6feb);';
      case 'engine_state':
        return 'background: var(--graph-kind-engine, #6e7681);';
      case 'skill':
        return 'background: var(--graph-kind-skill, #8957e5);';
      case 'external':
        return 'background: var(--graph-kind-external, #da3633);';
    }
  }

  function firstWord(s: string): string {
    return s.split(/[\s:]+/).pop() ?? s;
  }
</script>

<section class="graphview" aria-label="Mission graph">
  <header>
    <h3>Mission graph</h3>
    <code class="mission-id">{missionId}</code>
    <button type="button" onclick={load} disabled={loading || !hudUrl}>
      {loading ? 'Loading…' : 'Reload'}
    </button>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if graph}
    <p class="summary">
      {graph.nodes.length} nodes · {graph.edges.length} edges
    </p>

    <ul class="nodes">
      {#each graph.nodes as node (node.id)}
        <li class="node" style={kindColor(node.kind)}>
          <span class="kind">{node.kind}</span>
          <span class="label">{firstWord(node.label)}</span>
          <span class="prove" style={provenanceColor(node.provenance)}>
            {node.provenance}
          </span>
        </li>
      {/each}
    </ul>

    {#if graph.edges.length > 0}
      <table class="edges">
        <thead>
          <tr>
            <th>src</th>
            <th>kind</th>
            <th>dst</th>
            <th>precondition</th>
            <th>guard</th>
            <th>visits</th>
          </tr>
        </thead>
        <tbody>
          {#each graph.edges as edge (edge.id)}
            <tr>
              <td>{firstWord(edge.src)}</td>
              <td><code>{edge.kind}</code></td>
              <td>{firstWord(edge.dst)}</td>
              <td>{edge.precondition ?? '—'}</td>
              <td>{edge.guard ?? '—'}</td>
              <td class="num">{edge.visit_count}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {:else}
      <p class="empty">No edges persisted.</p>
    {/if}
  {:else if !loading && !error}
    <p class="empty">No graph persisted for this mission yet.</p>
  {/if}
</section>

<style>
  .graphview {
    border: 1px solid var(--graphview-border, #30363d);
    border-radius: 6px;
    padding: 0.75rem 1rem;
    font-family: var(--font-mono, ui-monospace, SFMono-Regular, Menlo, monospace);
    font-size: 0.85rem;
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin-bottom: 0.5rem;
  }
  header h3 {
    margin: 0;
    font-size: 0.95rem;
  }
  .mission-id {
    font-size: 0.75rem;
    color: var(--graphview-muted, #8b949e);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 auto;
  }
  button {
    font: inherit;
    padding: 2px 10px;
    border-radius: 4px;
    border: 1px solid var(--graphview-border, #30363d);
    background: var(--graphview-btn-bg, transparent);
    color: var(--graphview-fg, inherit);
    cursor: pointer;
  }
  button:disabled {
    cursor: default;
    opacity: 0.5;
  }
  .error {
    color: var(--graphview-error, #f85149);
    margin: 0.5rem 0;
  }
  .summary {
    margin: 0.25rem 0 0.5rem;
    color: var(--graphview-muted, #8b949e);
    font-size: 0.8rem;
  }
  .nodes {
    list-style: none;
    padding: 0;
    margin: 0 0 0.5rem;
    display: flex;
    flex-wrap: wrap;
    gap: 4px;
  }
  .node {
    display: inline-flex;
    align-items: center;
    gap: 6px;
    padding: 2px 8px;
    border-radius: 4px;
    color: #fff;
    font-size: 0.75rem;
  }
  .node .kind {
    font-size: 0.65rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    opacity: 0.8;
  }
  .node .label {
    font-weight: 600;
  }
  .node .prove {
    background: rgba(0, 0, 0, 0.35);
    padding: 1px 5px;
    border-radius: 3px;
    font-size: 0.6rem;
    letter-spacing: 0.04em;
  }
  .edges {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }
  .edges th,
  .edges td {
    text-align: left;
    padding: 2px 6px;
    border-bottom: 1px solid var(--graphview-border, #21262d);
  }
  .edges td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
  }
  .empty {
    color: var(--graphview-muted, #8b949e);
    font-style: italic;
    margin: 0.25rem 0;
  }
</style>

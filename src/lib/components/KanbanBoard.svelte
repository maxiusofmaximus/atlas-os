<!-- RFC 65 §3 (P0) — KanbanBoard: missions as cards across four operator columns.
     Reads `GET /tail/missions` (polling) and groups by status. Pure projection:
     no drag-and-drop persistence yet (P0 is visibility; drag write-back is P2).
     Empty/error states are explicit so a fresh HUD never looks broken. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchMissions,
    kanbanColumnOf,
    KANBAN_COLUMNS,
    type MissionRow,
    type KanbanColumn,
  } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let missions = $state<MissionRow[]>([]);
  let error = $state<string | null>(null);

  const COLUMN_LABEL: Record<KanbanColumn, string> = {
    pending: 'Pending',
    running: 'Running',
    done: 'Done',
    failed: 'Failed',
  };

  const grouped = $derived.by(() => {
    const g: Record<KanbanColumn, MissionRow[]> = {
      pending: [],
      running: [],
      done: [],
      failed: [],
    };
    for (const m of missions) {
      g[kanbanColumnOf(m.status)].push(m);
    }
    return g;
  });

  function colMissions(col: KanbanColumn): MissionRow[] {
    return grouped[col] ?? [];
  }

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      missions = await fetchMissions(hudUrl);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 10000);
    return () => clearInterval(timer);
  });

  function shortId(id: string): string {
    return id.length > 8 ? id.slice(0, 8) : id;
  }
</script>

<section class="kanban">
  <header>
    <h3>Missions</h3>
    <span class="count">{missions.length}</span>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if missions.length === 0}
    <p class="empty">
      No missions yet. Try <code>atlas mission new "&lt;prompt&gt;"</code>.
    </p>
  {:else}
    <div class="board">
      {#each KANBAN_COLUMNS as col (col)}
        <div class="column">
          <div class="col-head">
            <span class="dot {col}"></span>
            <span class="col-label">{COLUMN_LABEL[col]}</span>
            <span class="col-count">{colMissions(col).length}</span>
          </div>
          <ul class="cards">
            {#each colMissions(col) as m (m.id)}
              <li class="card">
                <span class="card-label">{m.label}</span>
                <span class="card-id">{shortId(m.id)}</span>
                <span class="card-status">{m.status}</span>
              </li>
            {/each}
          </ul>
        </div>
      {/each}
    </div>
  {/if}
</section>

<style>
  .kanban {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .kanban header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .kanban h3 {
    margin: 0;
    font-size: 1rem;
  }
  .count {
    font-size: 0.78rem;
    color: #6e7681;
  }
  .empty {
    color: #8b949e;
    font-size: 0.85rem;
    margin: 0;
  }
  .board {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.6rem;
  }
  .column {
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 5px;
    padding: 0.5rem;
    min-height: 3rem;
  }
  .col-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.75rem;
    color: #8b949e;
    text-transform: uppercase;
    letter-spacing: 0.04em;
    margin-bottom: 0.4rem;
  }
  .col-count {
    margin-left: auto;
    color: #6e7681;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #6e7681;
  }
  .dot.pending {
    background: #6e7681;
  }
  .dot.running {
    background: #58a6ff;
  }
  .dot.done {
    background: #56d364;
  }
  .dot.failed {
    background: #f85149;
  }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .card {
    background: #161b22;
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.4rem 0.5rem;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .card-label {
    font-size: 0.82rem;
    color: #c9d1d9;
  }
  .card-id {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.68rem;
    color: #6e7681;
  }
  .card-status {
    font-size: 0.68rem;
    color: #8b949e;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

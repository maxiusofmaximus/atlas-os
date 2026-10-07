<!-- RFC 65 §3 (P1) — HealthKPIs: agent-session telemetry, capability run states,
     swarm-registry states and supervisor heartbeat liveness. Reads
     `GET /hud/health` (polling) plus the live `agent_heartbeat` WS tail.
     Read-only; empty/error states are explicit. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import {
    hud,
    fetchHealth,
    projectLatestHeartbeat,
    type HealthResponse,
    type StatusCount,
    type StateCount,
  } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  const HEARTBEAT_STALE_MS = 15000;

  let data = $state<HealthResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  const heartbeatTs = $derived(projectLatestHeartbeat($hud.events));
  const heartbeatAgeMs = $derived(
    heartbeatTs ? Date.now() - Date.parse(heartbeatTs) : Number.POSITIVE_INFINITY,
  );
  const heartbeatLive = $derived(heartbeatAgeMs < HEARTBEAT_STALE_MS);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      data = await fetchHealth(hudUrl, 20);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 10000);
    return () => clearInterval(timer);
  });

  function total(counts: Array<StatusCount | StateCount>): number {
    return counts.reduce((sum, c) => sum + c.count, 0);
  }
</script>

<section class="health">
  <header>
    <h3>Health KPIs</h3>
    <span class="heartbeat" data-live={heartbeatLive ? 'yes' : 'no'}>
      {#if heartbeatTs}
        heartbeat {heartbeatLive ? 'live' : 'stale'} · {Math.round(heartbeatAgeMs / 1000)}s ago
      {:else}
        no heartbeat seen
      {/if}
    </span>
  </header>

  {#if error}
    <p class="error">
      Error: {error}
      <button type="button" class="retry" onclick={() => void refresh()}>Retry</button>
    </p>
  {:else if !data}
    {#if loading}
      <ul class="skeleton" aria-hidden="true">
        {#each [0, 1, 2] as n (n)}
          <li></li>
        {/each}
      </ul>
    {:else}
      <p class="empty">No health data yet.</p>
    {/if}
  {:else}
    <div class="kpis">
      <div class="kpi">
        <span class="kpi-label">Agent events</span>
        <span class="kpi-value">{data.agent_events.total.toLocaleString()}</span>
        <span class="kpi-sub">{data.agent_events.by_type.length} types</span>
      </div>
      <div class="kpi">
        <span class="kpi-label">Agent runs</span>
        <span class="kpi-value">{total(data.agent_runs)}</span>
        <span class="kpi-sub">{data.agent_runs.length} states</span>
      </div>
      <div class="kpi">
        <span class="kpi-label">Swarm agents</span>
        <span class="kpi-value">{total(data.swarm_agents)}</span>
        <span class="kpi-sub">{data.swarm_agents.length} states</span>
      </div>
    </div>

    <div class="breakdowns">
      <div class="breakdown">
        <h4>Events by type</h4>
        {#if data.agent_events.by_type.length === 0}
          <p class="empty">No agent-session telemetry yet.</p>
        {:else}
          <ul class="bars">
            {#each data.agent_events.by_type as t (t.event_type)}
              <li>
                <span class="bar-label">{t.event_type}</span>
                <span class="bar-count">{t.count}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <div class="breakdown">
        <h4>Run states</h4>
        {#if data.agent_runs.length === 0}
          <p class="empty">No agent runs yet.</p>
        {:else}
          <ul class="bars">
            {#each data.agent_runs as r (r.status)}
              <li>
                <span class="bar-label">{r.status}</span>
                <span class="bar-count">{r.count}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
      <div class="breakdown">
        <h4>Swarm states</h4>
        {#if data.swarm_agents.length === 0}
          <p class="empty">No swarm agents yet.</p>
        {:else}
          <ul class="bars">
            {#each data.swarm_agents as s (s.state)}
              <li>
                <span class="bar-label">{s.state}</span>
                <span class="bar-count">{s.count}</span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>

    <h4>Recent agent events</h4>
    {#if data.agent_events.recent.length === 0}
      <p class="empty">No recent events.</p>
    {:else}
      <ul class="events">
        {#each data.agent_events.recent as e (e.id)}
          <li>
            <time datetime={new Date(e.ts * 1000).toISOString()}
              >{new Date(e.ts * 1000).toLocaleTimeString()}</time
            >
            <span class="evt-type">{e.event_type}</span>
            <span class="evt-agent">{e.agent}</span>
            {#if e.pane_id}<span class="evt-pane">pane {e.pane_id}</span>{/if}
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .health {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .health header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .health h3 {
    margin: 0;
    font-size: 1rem;
  }
  .health h4 {
    margin: 0.4rem 0 0 0;
    font-size: 0.82rem;
    color: var(--a-info);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .heartbeat {
    font-size: 0.72rem;
    color: var(--a-text-muted);
  }
  .heartbeat[data-live='yes'] {
    color: var(--a-ok);
  }
  .heartbeat[data-live='no'] {
    color: var(--a-warn);
  }
  .kpis {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.6rem;
  }
  .kpi {
    background: var(--a-bg);
    border: 1px solid var(--a-surface-2);
    border-radius: 5px;
    padding: 0.5rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .kpi-label {
    font-size: 0.7rem;
    color: var(--a-text-muted);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .kpi-value {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 1rem;
    color: var(--a-text);
  }
  .kpi-sub {
    font-size: 0.68rem;
    color: var(--a-text-faint);
  }
  .breakdowns {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.6rem;
  }
  .breakdown {
    background: var(--a-bg);
    border: 1px solid var(--a-surface-2);
    border-radius: 5px;
    padding: 0.5rem 0.6rem;
  }
  .bars {
    list-style: none;
    margin: 0.3rem 0 0 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.74rem;
  }
  .bars li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
  }
  .bar-label {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-info);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .bar-count {
    color: var(--a-text-muted);
  }
  .events {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.74rem;
  }
  .events li {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
  }
  .events time {
    color: var(--a-text-faint);
    font-family: 'SF Mono', Consolas, monospace;
  }
  .evt-type {
    color: var(--a-info);
  }
  .evt-agent {
    color: var(--a-text-muted);
  }
  .evt-pane {
    margin-left: auto;
    color: var(--a-text-faint);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.8rem;
    margin: 0.2rem 0 0 0;
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .error .retry {
    margin-left: 0.5rem;
    background: transparent;
    border: 1px solid var(--a-border);
    color: var(--a-primary);
    border-radius: 4px;
    font-size: 0.72rem;
    padding: 0.1rem 0.5rem;
    cursor: pointer;
  }
  .skeleton {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .skeleton li {
    height: 3.2rem;
    border-radius: 6px;
    background: var(--a-surface-2);
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton li {
      animation: none;
    }
  }
</style>

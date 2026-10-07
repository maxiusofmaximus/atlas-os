<!-- RFC 65 §3 (P1) — CostDashboard: window totals + per-model roll-up + pending
     provider resets. Reads `GET /hud/cost?window=N` (polling). Read-only; the
     window selector just re-queries. Empty/error states are explicit so a fresh
     HUD never looks broken. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchCost, formatUsd, type CostResponse } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  const WINDOWS = [100, 200, 500] as const;

  let windowSize = $state<number>(200);
  let data = $state<CostResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      data = await fetchCost(hudUrl, windowSize);
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

  function pickWindow(n: number): void {
    if (n === windowSize) return;
    windowSize = n;
    void refresh();
  }
</script>

<section class="cost">
  <header>
    <h3>Cost &amp; Res</h3>
    <span class="window-picker" role="group" aria-label="Window">
      {#each WINDOWS as w (w)}
        <button
          type="button"
          class:active={w === windowSize}
          onclick={() => pickWindow(w)}
          data-window={w}>{w}</button
        >
      {/each}
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
      <p class="empty">No cost data yet.</p>
    {/if}
  {:else}
    <div class="kpis">
      <div class="kpi" data-kpi="cumulative">
        <span class="kpi-label">Cumulative</span>
        <span class="kpi-value" data-level={data.pressure.level}
          >{formatUsd(data.cumulative_usd)}</span
        >
        <span class="kpi-sub"
          >{data.pressure.level} · warn {formatUsd(data.pressure.warn_usd)} / crit {formatUsd(
            data.pressure.crit_usd,
          )}</span
        >
      </div>
      <div class="kpi" data-kpi="window">
        <span class="kpi-label">Window cost ({data.window})</span>
        <span class="kpi-value">{formatUsd(data.totals.cost_usd)}</span>
        <span class="kpi-sub">{data.totals.invocations} invocations</span>
      </div>
      <div class="kpi" data-kpi="tokens">
        <span class="kpi-label">Tokens in / out</span>
        <span class="kpi-value"
          >{data.totals.tokens_in.toLocaleString()} / {data.totals.tokens_out.toLocaleString()}</span
        >
        <span class="kpi-sub">mean latency {Math.round(data.totals.mean_latency_ms)} ms</span>
      </div>
    </div>

    <h4>By model</h4>
    {#if data.by_model.length === 0}
      <p class="empty">No model invocations in this window.</p>
    {:else}
      <table class="models">
        <thead>
          <tr>
            <th>Model</th>
            <th>Provider</th>
            <th class="num">Calls</th>
            <th class="num">Cost</th>
            <th class="num">Tokens in</th>
            <th class="num">Tokens out</th>
            <th class="num">Latency</th>
          </tr>
        </thead>
        <tbody>
          {#each data.by_model as m (m.model_id + '/' + m.provider)}
            <tr>
              <td class="model-id">{m.model_id}</td>
              <td class="provider">{m.provider}</td>
              <td class="num">{m.invocations}</td>
              <td class="num">{formatUsd(m.cost_usd)}</td>
              <td class="num">{m.tokens_in.toLocaleString()}</td>
              <td class="num">{m.tokens_out.toLocaleString()}</td>
              <td class="num">{Math.round(m.mean_latency_ms)} ms</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}

    <h4>Pending resets <span class="count">{data.pending_resets.length}</span></h4>
    {#if data.pending_resets.length === 0}
      <p class="empty">No provider rate/spend limits pending.</p>
    {:else}
      <ul class="resets">
        {#each data.pending_resets as r (`${r.provider}/${r.model}`)}
          <li>
            <code>{r.provider}/{r.model}</code>
            <span class="reset-kind">{r.error_type ?? 'limit'}</span>
            <span class="reset-code">HTTP {r.status_code}</span>
            <time datetime={new Date(r.resets_at_ms).toISOString()}
              >resets {new Date(r.resets_at_ms).toLocaleString()}</time
            >
          </li>
        {/each}
      </ul>
    {/if}
  {/if}
</section>

<style>
  .cost {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .cost header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .cost h3 {
    margin: 0;
    font-size: 1rem;
  }
  .cost h4 {
    margin: 0.4rem 0 0 0;
    font-size: 0.82rem;
    color: var(--a-info);
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .count {
    font-size: 0.78rem;
    color: var(--a-text-faint);
  }
  .window-picker {
    display: inline-flex;
    gap: 0.25rem;
  }
  .window-picker button {
    background: transparent;
    border: 1px solid var(--a-border);
    color: var(--a-text-muted);
    border-radius: 4px;
    font-size: 0.72rem;
    padding: 0.1rem 0.45rem;
    cursor: pointer;
  }
  .window-picker button.active {
    color: var(--a-info);
    border-color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 12%, transparent);
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
  .kpi-value[data-level='warn'] {
    color: var(--a-warn);
  }
  .kpi-value[data-level='critical'] {
    color: var(--a-err);
  }
  .kpi-sub {
    font-size: 0.68rem;
    color: var(--a-text-faint);
  }
  table.models {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.76rem;
  }
  table.models th,
  table.models td {
    text-align: left;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid var(--a-surface-2);
  }
  table.models th {
    color: var(--a-text-muted);
    font-weight: 500;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .num {
    text-align: right;
    font-family: 'SF Mono', Consolas, monospace;
  }
  .model-id {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-info);
  }
  .provider {
    color: var(--a-text-muted);
  }
  .resets {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.76rem;
  }
  .resets li {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .reset-kind {
    color: var(--a-warn);
  }
  .reset-code {
    color: var(--a-text-faint);
  }
  .resets time {
    color: var(--a-text-muted);
    margin-left: auto;
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

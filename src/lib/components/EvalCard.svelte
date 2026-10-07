<!-- RFC 20 Fase 22 (EVAL.2) — HUD card for the evaluation harness.
     Polls `GET /hud/eval/summary` and renders the normalized metrics:
     pass rate, tokens/solved, $/solved and the failure-kind histogram, plus
     the per harness × model breakdown (the unit of measure). Pure read-only. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchEvalSummary, type EvalSummaryResponse } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<EvalSummaryResponse | null>(null);
  let error = $state<string | null>(null);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      data = await fetchEvalSummary(hudUrl, 20);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 15000);
    return () => clearInterval(timer);
  });

  function pct(v: number): string {
    return `${(v * 100).toFixed(1)}%`;
  }

  function num(v: number): string {
    return v.toLocaleString();
  }
</script>

<section class="eval-card">
  <header>
    <h3>Evaluation</h3>
    {#if data}
      <span class="tag">{data.summary.runs} runs</span>
    {:else}
      <span class="tag idle">idle</span>
    {/if}
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if !data || data.summary.total === 0}
    <p class="empty">
      No eval data yet. Try <code>atlas eval run golden</code> or
      <code>atlas eval import &lt;job-dir&gt;</code>.
    </p>
  {:else}
    <dl class="metrics">
      <div>
        <dt>pass rate</dt>
        <dd
          class={data.summary.pass_rate >= 0.8 ? 'good' : data.summary.pass_rate < 0.5 ? 'bad' : ''}
        >
          {pct(data.summary.pass_rate)}
        </dd>
      </div>
      <div>
        <dt>tokens/solved</dt>
        <dd>{num(Math.round(data.summary.tokens_per_solved))}</dd>
      </div>
      <div>
        <dt>$ / solved</dt>
        <dd>${data.summary.cost_per_solved.toFixed(4)}</dd>
      </div>
      <div>
        <dt>cases</dt>
        <dd>{data.summary.passed}/{data.summary.total}</dd>
      </div>
    </dl>

    {#if Object.keys(data.summary.failure_kinds).length > 0}
      <div class="kinds">
        <span class="label">failure kinds:</span>
        {#each Object.entries(data.summary.failure_kinds) as [kind, n] (kind)}
          <span class="kind">{kind} · {n}</span>
        {/each}
      </div>
    {/if}

    {#if data.groups.length > 1}
      <table class="groups">
        <thead>
          <tr>
            <th>harness / model</th>
            <th>pass</th>
            <th>tok/solved</th>
            <th>$/solved</th>
          </tr>
        </thead>
        <tbody>
          {#each data.groups as g (g.key)}
            <tr>
              <td><code>{g.key}</code></td>
              <td>{pct(g.summary.pass_rate)}</td>
              <td>{num(Math.round(g.summary.tokens_per_solved))}</td>
              <td>${g.summary.cost_per_solved.toFixed(4)}</td>
            </tr>
          {/each}
        </tbody>
      </table>
    {/if}
  {/if}
</section>

<style>
  .eval-card {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .eval-card header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .eval-card h3 {
    margin: 0;
    font-size: 1rem;
  }
  .tag {
    font-size: 0.78rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 12%, transparent);
  }
  .tag.idle {
    color: var(--a-text-faint);
    background: var(--a-surface-2);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.85rem;
    margin: 0;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(4, 1fr);
    gap: 0.4rem;
    margin: 0;
  }
  .metrics dt {
    font-size: 0.72rem;
    color: var(--a-text-faint);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .metrics dd {
    margin: 0.1rem 0 0 0;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.95rem;
  }
  .metrics dd.good {
    color: var(--a-ok);
  }
  .metrics dd.bad {
    color: var(--a-err);
  }
  .kinds {
    display: flex;
    flex-wrap: wrap;
    gap: 0.4rem;
    align-items: center;
  }
  .kinds .label {
    font-size: 0.72rem;
    color: var(--a-text-faint);
    text-transform: uppercase;
  }
  .kind {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.78rem;
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
    padding: 0.05rem 0.4rem;
    border-radius: 4px;
  }
  .groups {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.8rem;
  }
  .groups th,
  .groups td {
    text-align: left;
    padding: 0.2rem 0.4rem;
    border-bottom: 1px solid var(--a-surface-2);
  }
  .groups th {
    color: var(--a-text-faint);
    font-weight: 500;
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
</style>

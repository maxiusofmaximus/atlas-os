<!-- RFC 28 §A — HUD card for the Karpathy autoresearch loop.
     Subscribes to the `autoresearch:<run_id>` telemetry channel
     surfaced by `journal::autoresearch::AutoresearchAction::PublishTelemetry`.
     Renders baseline, current best metric, step progress, mini
     sparkline of candidates, and Stop/Pause buttons that emit a
     `session/cancel`. The supervisor (running on the Rust host)
     owns the keep/reset verdict; the HUD is pure telemetry.

     Phase 1.5b §A-3 ships this card with a stub harness — the
     real telemetry wires onto the kernel bus once supervisor mode
     `MissionPhase::Autoresearch` is integrated into the runner. -->

<script lang="ts">
  import type { AutoresearchSnapshot, AutoresearchCancel } from '$stores/hud';
  import { postAutoresearchCancel } from '$stores/hud';

  interface Props {
    snapshot: AutoresearchSnapshot | null;
    candidates: Array<{ step: number; metric_after: number; kept: boolean }>;
    hudUrl: string | null;
  }

  let { snapshot, candidates, hudUrl }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  const outcomeLabel: Record<string, string> = {
    running: 'running',
    improved: 'improved',
    plateau: 'plateau',
    timeout: 'timeout',
    aborted: 'aborted',
  };

  function fmtMetric(v: number | null | undefined): string {
    if (v == null) return '—';
    return v.toFixed(6);
  }

  function pct(steps: number, max: number): number {
    if (max <= 0) return 0;
    return Math.min(100, Math.round((steps / max) * 100));
  }

  function outcomeRunning(snapshot: AutoresearchSnapshot | null): boolean {
    return snapshot != null && snapshot.outcome === 'running';
  }

  async function onStop(): Promise<void> {
    if (!snapshot) return;
    if (!hudUrl) {
      error = 'no HUD URL yet';
      return;
    }
    busy = true;
    error = null;
    const req: AutoresearchCancel = {
      run_id: snapshot.id,
      outcome: 'aborted',
    };
    try {
      await postAutoresearchCancel(hudUrl, req);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  // Mini-sparkline: 0–height path; scaled to snapshot range.
  function sparkline(
    pts: Array<{ metric_after: number; kept: boolean }>,
    baseline: number,
  ): string {
    if (pts.length === 0) return '';
    const all = pts.map((p) => p.metric_after).concat([baseline]);
    const lo = Math.min(...all);
    const hi = Math.max(...all);
    const w = 120;
    const h = 32;
    const range = hi - lo || 1;
    const dx = pts.length > 1 ? w / (pts.length - 1) : w;
    const coords = pts.map((p, i) => {
      const x = i * dx;
      const y = h - ((p.metric_after - lo) / range) * h;
      return `${x.toFixed(1)},${y.toFixed(1)}`;
    });
    return `M ${coords.join(' L ')}`;
  }
</script>

<section class="autoresearch-card" data-outcome={snapshot?.outcome ?? 'idle'}>
  <header>
    <h3>Autoresearch</h3>
    {#if snapshot}
      <span class="outcome tag-{snapshot.outcome}">
        {outcomeLabel[snapshot.outcome] ?? snapshot.outcome}
      </span>
    {:else}
      <span class="outcome tag-idle">idle</span>
    {/if}
  </header>

  {#if !snapshot}
    <p class="empty">
      No active run. Try
      <code>opencode mission new --autoresearch --metric "rg -c 'error' src"</code>.
    </p>
  {:else}
    <dl class="metrics">
      <div>
        <dt>baseline</dt>
        <dd>{fmtMetric(snapshot.baseline_metric)}</dd>
      </div>
      <div>
        <dt>best</dt>
        <dd
          class={snapshot.best_metric != null && snapshot.best_metric < snapshot.baseline_metric
            ? 'better'
            : ''}
        >
          {fmtMetric(snapshot.best_metric)}
        </dd>
      </div>
      <div>
        <dt>step</dt>
        <dd>{snapshot.step_count} / {snapshot.max_steps}</dd>
      </div>
    </dl>

    <div
      class="progress"
      role="progressbar"
      aria-valuenow={snapshot.step_count}
      aria-valuemax={snapshot.max_steps}
    >
      <div class="bar" style:width="{pct(snapshot.step_count, snapshot.max_steps)}%"></div>
    </div>

    {#if candidates.length > 0}
      <svg class="spark" viewBox="0 0 120 32" preserveAspectRatio="none" aria-hidden="true">
        <path
          d={sparkline(candidates, snapshot.baseline_metric)}
          stroke="#58a6ff"
          fill="none"
          stroke-width="1.5"
        />
      </svg>
    {:else}
      <p class="hint">Awaiting first candidate…</p>
    {/if}

    <p class="cmd">
      <span class="label">metric:</span>
      <code>{snapshot.metric_command}</code>
    </p>

    {#if outcomeRunning(snapshot)}
      <div class="actions">
        <button type="button" disabled={busy} onclick={onStop}>
          {busy ? 'stopping…' : 'Stop'}
        </button>
      </div>
    {/if}

    {#if error}
      <p class="error">Error: {error}</p>
    {/if}
  {/if}
</section>

<style>
  .autoresearch-card {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .autoresearch-card header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .autoresearch-card h3 {
    margin: 0;
    font-size: 1rem;
  }
  .outcome {
    font-size: 0.78rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    color: #8b949e;
    background: #21262d;
  }
  .outcome.tag-running {
    color: #58a6ff;
    background: rgba(88, 166, 255, 0.12);
  }
  .outcome.tag-improved {
    color: #56d364;
    background: rgba(86, 211, 100, 0.12);
  }
  .outcome.tag-plateau {
    color: #f0883e;
    background: rgba(240, 136, 62, 0.12);
  }
  .outcome.tag-timeout {
    color: #d29922;
    background: rgba(210, 153, 34, 0.12);
  }
  .outcome.tag-aborted {
    color: #f85149;
    background: rgba(248, 81, 73, 0.12);
  }
  .outcome.tag-idle {
    color: #6e7681;
  }
  .empty {
    color: #8b949e;
    font-size: 0.85rem;
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem;
    margin: 0;
  }
  .metrics dt {
    font-size: 0.72rem;
    color: #6e7681;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .metrics dd {
    margin: 0.1rem 0 0 0;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.9rem;
  }
  .metrics dd.better {
    color: #56d364;
  }
  .progress {
    height: 6px;
    background: #0d1117;
    border-radius: 3px;
    overflow: hidden;
  }
  .progress .bar {
    height: 100%;
    background: #58a6ff;
    transition: width 200ms ease-out;
  }
  .spark {
    width: 100%;
    height: 32px;
    display: block;
  }
  .hint,
  .cmd {
    margin: 0;
    font-size: 0.78rem;
    color: #8b949e;
  }
  .cmd code {
    color: #c9d1d9;
    background: #0d1117;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .actions button {
    padding: 0.4rem 0.9rem;
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .actions button:hover:not(:disabled) {
    background: #30363d;
  }
  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

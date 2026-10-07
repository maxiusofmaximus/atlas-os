<!-- RFC 20 Fase 23 (v3.1.2.3) — HUD card for the proactive turn engine.
     Polls `GET /hud/availability` and shows whether a proactive turn can start
     (RunNow / WaitUntil / Blocked), the persisted policy and the pending
     mission. Pure read-only. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchAvailability, type AvailabilityResponse } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<AvailabilityResponse | null>(null);
  let error = $state<string | null>(null);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      data = await fetchAvailability(hudUrl);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 15000);
    return () => clearInterval(timer);
  });

  type Tone = 'run' | 'wait' | 'blocked' | 'off';

  function tone(d: AvailabilityResponse | null): Tone {
    if (!d || !d.enabled) return 'off';
    if (d.availability === 'RunNow') return 'run';
    if (d.availability === 'Blocked') return 'blocked';
    return 'wait';
  }

  function label(d: AvailabilityResponse | null): string {
    if (!d) return 'loading…';
    if (!d.enabled) return 'disabled';
    if (d.availability === 'RunNow') return 'RUN NOW';
    if (d.availability === 'Blocked') return 'BLOCKED';
    if (d.availability && typeof d.availability === 'object') {
      return `wait until ${new Date(d.availability.WaitUntil).toLocaleTimeString()}`;
    }
    return 'unknown';
  }
</script>

<section class="avail-card" data-tone={tone(data)}>
  <header>
    <h3>Proactive turn</h3>
    <span class="state tag-{tone(data)}">{label(data)}</span>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if data}
    <dl class="metrics">
      <div>
        <dt>eta</dt>
        <dd>{Math.round(data.policy.eta_ms / 60000)} min</dd>
      </div>
      <div>
        <dt>weight ≥</dt>
        <dd>{data.policy.weight_threshold.toFixed(2)}</dd>
      </div>
      <div>
        <dt>pending</dt>
        <dd>{data.pending_mission ? data.pending_mission.slice(0, 8) : '—'}</dd>
      </div>
    </dl>
    {#if data.pending_mission}
      <p class="cmd">
        <span class="label">next mission:</span> <code>{data.pending_mission}</code>
      </p>
    {/if}
  {/if}
</section>

<style>
  .avail-card {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .avail-card header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .avail-card h3 {
    margin: 0;
    font-size: 1rem;
  }
  .state {
    font-size: 0.78rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    color: var(--a-text-muted);
    background: var(--a-surface-2);
  }
  .state.tag-run {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
  }
  .state.tag-wait {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
  }
  .state.tag-blocked {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
  }
  .state.tag-off {
    color: var(--a-text-faint);
  }
  .metrics {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
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
    font-size: 0.9rem;
  }
  .cmd {
    margin: 0;
    font-size: 0.78rem;
    color: var(--a-text-muted);
  }
  .cmd code {
    color: var(--a-text);
    background: var(--a-bg);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
</style>

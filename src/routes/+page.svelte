<script lang="ts">
  // OpenCode OS — HUD Mission Control landing page (Phase 1).
  // See RFC 24 for the full design. Phase 1 renders:
  //  - HUD URL + WS status (where the axum WS server is bound)
  //  - Live journal stream (kernel bus events)
  //  - Nine tail boxes polling the axum tail routes (RFC 24 §2)
  //    every 5s. Each box shows the latest 20 rows; clicking a row
  //    opens the JSON payload in a side drawer (Phase 2).
  import { onMount } from 'svelte';
  import { hud, fetchTail, phaseColor, type TailKind, type StepPhaseTag } from '$stores/hud';
  import type { PageData } from './$types';

  const { data } = $props<{ data: PageData }>();

  type TailBox = {
    kind: TailKind;
    title: string;
    rows: Array<Record<string, unknown>>;
    loading: boolean;
    error: string | null;
  };

  // Order mirrors the RFC 24 §3 left-to-right reading order of the
  // Mission Control deck: prompt → planning → coding → validation →
  // repair → learning → supervisor → skills. RFC 27 §B/§G add the
  // two orchestration boxes (model swaps + step pills) at the tail.
  const tailKinds: Array<{ kind: TailKind; title: string }> = [
    { kind: 'verdicts', title: 'Prompt verdicts' },
    { kind: 'consolidated', title: 'Consolidated missions' },
    { kind: 'plans', title: 'Plans' },
    { kind: 'diffs', title: 'Code diffs' },
    { kind: 'validation_reports', title: 'Validation reports' },
    { kind: 'repairs', title: 'Repair runs' },
    { kind: 'patterns', title: 'Learned patterns' },
    { kind: 'checkpoints', title: 'Mission checkpoints' },
    { kind: 'skills', title: 'Skill manifests' },
    { kind: 'model_swaps', title: 'Model swaps' },
    { kind: 'step_states', title: 'Step pills' },
  ];

  let tails = $state<Record<string, TailBox>>(
    Object.fromEntries(
      tailKinds.map((t) => [
        t.kind,
        { kind: t.kind, title: t.title, rows: [], loading: false, error: null } satisfies TailBox,
      ]),
    ),
  );

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  async function refreshOne(box: TailBox): Promise<TailBox> {
    if (!data.hudUrl) return { ...box, error: 'no HUD URL yet' };
    try {
      const rows = await fetchTail<Record<string, unknown>>(data.hudUrl, box.kind, 20);
      return { ...box, rows, loading: false, error: null };
    } catch (err) {
      return { ...box, loading: false, error: err instanceof Error ? err.message : String(err) };
    }
  }

  async function refreshAll() {
    const entries = await Promise.all(
      Object.values(tails).map(async (box) => {
        const next = await refreshOne({ ...box, loading: true });
        return [box.kind, next] as const;
      }),
    );
    tails = Object.fromEntries(entries) as Record<string, TailBox>;
  }

  onMount(() => {
    if (data.hudUrl) {
      hud.connect(data.hudUrl);
    }
    void refreshAll();
    pollTimer = setInterval(() => void refreshAll(), 5000);
    return () => {
      hud.disconnect();
      if (pollTimer) clearInterval(pollTimer);
    };
  });

  function rowId(row: Record<string, unknown>): string {
    return String(
      row.id ??
        row.mission_id ??
        row.learn_id ??
        row.skill_id ??
        row.event_id ??
        Object.keys(row).join('|'),
    );
  }

  function rowSummary(row: Record<string, unknown>): string {
    const pick = row.label ?? row.prompt ?? row.summary ?? row.phase ?? row.kind ?? row.title ?? '';
    return String(pick).slice(0, 80);
  }

  /// RFC 27 §G — color lookup for a step pill. `row.phase` is typed as
  /// `unknown` (it came through the generic tail), so funnel through a
  /// narrowing guard instead of a `as StepPhaseTag` cast that Svelte's
  /// template parser rejects.
  function pillColor(row: Record<string, unknown>): string {
    return phaseColor(typeof row.phase === 'string' ? (row.phase as StepPhaseTag) : 'pending');
  }
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
    <h2>Journal tail (live)</h2>
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

  <section class="tails">
    <h2>Pipeline tails</h2>
    <div class="tails-grid">
      {#each Object.values(tails) as box (box.kind)}
        <article class="tail-box" data-kind={box.kind}>
          <header>
            <h3>{box.title}</h3>
            <span class="count">{box.rows.length}</span>
          </header>
          {#if box.error}
            <p class="error">{box.error}</p>
          {:else if box.rows.length === 0}
            <p class="empty">No rows yet.</p>
          {:else if box.kind === 'step_states'}
            {#each box.rows as row (rowId(row))}
              <p class="step-row">
                <code>{String(row.step_id ?? '').slice(0, 8)}</code>
                <span class="pill" data-phase={pillColor(row)}>{row.phase}</span>
              </p>
            {/each}
          {:else if box.kind === 'model_swaps'}
            {#each box.rows as row (rowId(row))}
              <p class="swap-row">
                <code>{row.prev_model_id}</code>
                <span aria-hidden="true">→</span>
                <code class="swap-new">{row.new_model_id}</code>
                <span class="swap-init" data-by={row.initiator}>{row.initiator}</span>
              </p>
            {/each}
          {:else}
            <ul>
              {#each box.rows as row (rowId(row))}
                <li>
                  <code>{rowId(row).slice(0, 8)}</code>
                  <span>{rowSummary(row)}</span>
                </li>
              {/each}
            </ul>
          {/if}
        </article>
      {/each}
    </div>
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
  .tails-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
  }
  .tail-box {
    border: 1px solid #30363d;
    background: #0d1117;
    border-radius: 6px;
    padding: 0.6rem 0.75rem;
    min-height: 140px;
  }
  .tail-box header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin: 0 0 0.4rem 0;
  }
  .tail-box h3 {
    font-size: 0.85rem;
    color: #79c0ff;
    margin: 0;
  }
  .tail-box .count {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .tail-box ul {
    list-style: none;
    padding: 0;
    margin: 0;
    font-family: 'Fira Code', monospace;
    font-size: 0.72rem;
  }
  .tail-box li {
    display: flex;
    gap: 0.4rem;
    padding: 0.18rem 0;
    border-bottom: 1px solid #161b22;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tail-box .error {
    color: #f85149;
    font-size: 0.75rem;
  }
</style>

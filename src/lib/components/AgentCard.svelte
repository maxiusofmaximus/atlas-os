<script lang="ts">
  import { onMount } from 'svelte';
  import {
    hud,
    agentRunId,
    projectAgentSteps,
    agentRunTotals,
    agentStepColor,
    fetchTail,
    approveApproval,
    denyApproval,
    AGENT_STEP_KIND,
    type AgentStepPayload,
    type HudEvent,
  } from '$stores/hud';

  type AgentStatusTag =
    | 'Queued'
    | 'Reading'
    | 'Planning'
    | 'Coding'
    | 'Reviewing'
    | 'Idle'
    | 'Paused'
    | 'DoomLoop'
    | 'Error'
    | 'Success'
    | 'Unknown';

  type Tone = 'faint' | 'info' | 'warn' | 'err' | 'ok' | 'muted' | 'violet';

  const STATUS: Record<AgentStatusTag, { glyph: string; tone: Tone; label: string }> = {
    Queued: { glyph: '○', tone: 'faint', label: 'queued' },
    Reading: { glyph: '●', tone: 'info', label: 'reading' },
    Planning: { glyph: '●', tone: 'info', label: 'planning' },
    Coding: { glyph: '●', tone: 'info', label: 'coding' },
    Reviewing: { glyph: '◐', tone: 'warn', label: 'reviewing' },
    Idle: { glyph: '◌', tone: 'muted', label: 'idle' },
    Paused: { glyph: '❙❙', tone: 'warn', label: 'paused' },
    DoomLoop: { glyph: '⚠', tone: 'err', label: 'doom_loop' },
    Error: { glyph: '✖', tone: 'err', label: 'error' },
    Success: { glyph: '✓', tone: 'ok', label: 'success' },
    Unknown: { glyph: '?', tone: 'faint', label: 'unknown' },
  };

  const TERMINAL: AgentStatusTag[] = ['Success', 'Error'];
  const HEARTBEAT_STALE_MS = 30_000;

  interface DiffPayload {
    agent_id: string;
    files: string[];
    lines_added: number;
    lines_removed: number;
  }

  interface Props {
    hudUrl?: string | null;
    agentId?: string | null;
    approvalId?: string | null;
  }

  const { hudUrl, agentId = null, approvalId = null }: Props = $props();

  let tailSteps = $state<AgentStepPayload[]>([]);
  let loading = $state(true);
  let loadError = $state(false);
  let expanded = $state(false);
  let layer3 = $state(false);
  let actionNote = $state<string | null>(null);
  let now = $state(Date.now());

  function lastEvent(events: HudEvent[], kind: string): HudEvent | null {
    for (let i = events.length - 1; i >= 0; i--) {
      const e = events[i];
      if (e && e.kind === kind) return e;
    }
    return null;
  }

  function countEvents(events: HudEvent[], kind: string): number {
    let n = 0;
    for (const e of events) if (e.kind === kind) n++;
    return n;
  }

  function payloadOf<T>(e: HudEvent | null): T | null {
    return e ? (e.payload as T) : null;
  }

  function normalizeStatus(raw: unknown): AgentStatusTag | null {
    if (typeof raw !== 'string' || raw.length === 0) return null;
    const map: Record<string, AgentStatusTag> = {
      queued: 'Queued',
      reading: 'Reading',
      planning: 'Planning',
      coding: 'Coding',
      reviewing: 'Reviewing',
      idle: 'Idle',
      paused: 'Paused',
      doomloop: 'DoomLoop',
      error: 'Error',
      success: 'Success',
      unknown: 'Unknown',
    };
    return map[raw.toLowerCase().replace(/[_\s]/g, '')] ?? 'Unknown';
  }

  function deriveFromSteps(list: AgentStepPayload[]): AgentStatusTag {
    if (list.length === 0) return 'Idle';
    const last = list[list.length - 1];
    if (!last) return 'Idle';
    if (last.verdict === 'fail') return 'Error';
    if (last.action === 'done' && last.verdict !== 'fail') return 'Success';
    return 'Coding';
  }

  function formatDuration(ms: number): string {
    if (!Number.isFinite(ms) || ms < 0) return '—';
    const s = Math.round(ms / 1000);
    if (s < 60) return `${s}s`;
    const m = Math.floor(s / 60);
    const r = s % 60;
    return r > 0 ? `${m}m ${r}s` : `${m}m`;
  }

  function shortId(id: string): string {
    return id.length > 12 ? `${id.slice(0, 12)}…` : id;
  }

  function tailRunId(rows: AgentStepPayload[]): string | null {
    return rows.length > 0 ? (rows[0]?.run_id ?? null) : null;
  }

  function tailStepsForRun(rows: AgentStepPayload[], id: string | null): AgentStepPayload[] {
    if (!id) return [];
    return rows
      .filter((r) => r.run_id === id)
      .slice()
      .reverse();
  }

  async function refreshTail(): Promise<void> {
    if (!hudUrl) {
      loading = false;
      return;
    }
    try {
      tailSteps = await fetchTail<AgentStepPayload>(hudUrl, 'agent_steps', 100);
      loadError = false;
    } catch {
      loadError = true;
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refreshTail();
    const poll = setInterval(() => void refreshTail(), 5000);
    const clock = setInterval(() => (now = Date.now()), 5000);
    return () => {
      clearInterval(poll);
      clearInterval(clock);
    };
  });

  const liveRunId = $derived(agentRunId($hud.events));
  const runId = $derived(liveRunId ?? tailRunId(tailSteps));
  const steps = $derived(
    liveRunId ? projectAgentSteps($hud.events, liveRunId) : tailStepsForRun(tailSteps, runId),
  );
  const totals = $derived(agentRunTotals(steps));

  const runStepEvents = $derived(
    $hud.events.filter(
      (e) => e.kind === AGENT_STEP_KIND && (e.payload as AgentStepPayload | null)?.run_id === runId,
    ),
  );
  const startedAt = $derived(runStepEvents[0]?.ts ?? null);
  const lastStepTs = $derived(runStepEvents[runStepEvents.length - 1]?.ts ?? null);
  const heartbeatEvent = $derived(lastEvent($hud.events, 'agent_heartbeat'));
  const lastHeartbeatTs = $derived(heartbeatEvent?.ts ?? null);

  const lastActivityMs = $derived(
    Math.max(
      lastStepTs ? Date.parse(lastStepTs) : 0,
      lastHeartbeatTs ? Date.parse(lastHeartbeatTs) : 0,
    ),
  );
  const elapsedMs = $derived(
    startedAt ? (lastStepTs ? Date.parse(lastStepTs) : now) - Date.parse(startedAt) : 0,
  );

  const wsStatus = $derived(
    normalizeStatus(
      payloadOf<{ status?: string }>(lastEvent($hud.events, 'agent_status_changed'))?.status,
    ),
  );
  const baseStatus = $derived(wsStatus ?? deriveFromSteps(steps));
  const stale = $derived(
    runId !== null &&
      lastActivityMs > 0 &&
      now - lastActivityMs > HEARTBEAT_STALE_MS &&
      !TERMINAL.includes(baseStatus),
  );
  const status = $derived(stale ? 'Unknown' : baseStatus);
  const tone = $derived(STATUS[status].tone);
  const glyph = $derived(STATUS[status].glyph);
  const statusLabel = $derived(STATUS[status].label);

  const doomCount = $derived(countEvents($hud.events, 'doom_loop_detected'));
  const drift = $derived(
    payloadOf<{ drift?: string }>(lastEvent($hud.events, 'goal_drift_detected'))?.drift ?? null,
  );
  const diff = $derived(payloadOf<DiffPayload>(lastEvent($hud.events, 'agent_diff')));
  const files = $derived(diff?.files ?? []);
  const model = $derived(
    payloadOf<{ new_model_id?: string }>(lastEvent($hud.events, 'model_swapped'))?.new_model_id ??
      null,
  );
  const skill = $derived(
    payloadOf<{ skill_id?: string }>(lastEvent($hud.events, 'skill_activated'))?.skill_id ?? null,
  );
  const phase = $derived(
    payloadOf<{ phase?: string }>(lastEvent($hud.events, 'step_phase_changed'))?.phase ?? null,
  );
  const checkpoint = $derived(
    payloadOf<{ checkpoint_id?: string }>(lastEvent($hud.events, 'journal_checkpoint'))
      ?.checkpoint_id ?? null,
  );
  const toolCalls = $derived(steps.filter((s) => s.action === 'run_command').length);
  const collapsed = $derived(TERMINAL.includes(status) && !expanded);
  const disconnected = $derived(!$hud.connected);
  const partial = $derived(disconnected && steps.length > 0);

  const actions = $derived<string[]>(
    status === 'DoomLoop'
      ? ['recover', 'override', 'stop']
      : status === 'Paused'
        ? ['resume', 'stop', 'steer']
        : status === 'Queued'
          ? ['stop']
          : status === 'Success' || status === 'Error'
            ? ['reason', 'demo']
            : status === 'Idle' || status === 'Unknown'
              ? ['reason']
              : ['pause', 'stop', 'steer', 'fork'],
  );

  function actionLabel(name: string): string {
    const map: Record<string, string> = {
      recover: 'Recover',
      override: 'Override',
      stop: 'Stop',
      resume: 'Resume',
      pause: 'Pause',
      steer: 'Steer',
      fork: 'Fork',
      reason: 'Reason',
      demo: 'Demo',
    };
    return map[name] ?? name;
  }

  function act(name: string): void {
    actionNote = null;
    if ((name === 'approve' || name === 'deny') && approvalId && hudUrl) {
      const call = name === 'approve' ? approveApproval : denyApproval;
      void call(hudUrl, approvalId).catch(() => {
        actionNote = `${name} failed`;
      });
      return;
    }
    dispatchEvent(
      new CustomEvent('atlas:agent-action', {
        detail: { action: name, runId, agentId },
        bubbles: true,
      }),
    );
    actionNote = `${name} → host`;
  }
</script>

<section
  class="agent-card"
  role="article"
  tabindex="-1"
  aria-label={`Agent ${runId ? shortId(runId) : 'none'} · ${statusLabel}`}
>
  <header class="card-head">
    <div class="identity">
      <span class={`spine tone-${tone}`} aria-hidden="true">{glyph}</span>
      <h3>Agent</h3>
      <span class={`status tone-${tone}`}>{statusLabel}</span>
    </div>
    <div class="head-meta">
      <span class="field"><span class="k">model</span><span class="v">{model ?? '—'}</span></span>
      <span class="field"
        ><span class="k">agent</span><span class="v">{agentId ? shortId(agentId) : '—'}</span></span
      >
      <span class="field"
        ><span class="k">time</span><span class="v">{formatDuration(elapsedMs)}</span></span
      >
    </div>
  </header>

  {#if disconnected}
    <p class="offline" role="status">
      sin conexión · reconectando{#if lastStepTs}
        · último sync {lastStepTs.slice(11, 19)}{/if}
    </p>
  {/if}
  {#if partial}
    <p class="partial" role="status">
      parcial · stream caído, mostrando últimos {steps.length} pasos
    </p>
  {/if}

  {#if loading && !runId}
    <div class="loading" aria-busy="true" aria-label="cargando agente">
      <span class="skel skel-title"></span>
      <span class="skel skel-line"></span>
      <span class="skel skel-line short"></span>
    </div>
  {:else if loadError && !runId}
    <div class="error" role="alert">
      <span>no se pudo cargar el agente</span>
      <button type="button" onclick={() => void refreshTail()}>Retry</button>
    </div>
  {:else if !runId || steps.length === 0}
    <p class="empty">
      No agent run yet. Try <code>atlas agent "&lt;task&gt;" --verify</code>.
    </p>
  {:else}
    <article class="card-body">
      {#if collapsed}
        <div class="layer-0">
          <span class="worked">Worked for {formatDuration(elapsedMs)}</span>
          <span class="statline-sm"
            >{totals.tokens_in}↑ {totals.tokens_out}↓ tok · {toolCalls} tools</span
          >
          <button
            type="button"
            class="ghost"
            aria-expanded={expanded}
            onclick={() => (expanded = true)}
          >
            Show detail
          </button>
        </div>
      {/if}

      {#if !collapsed}
        <div class="statline">
          <span class="chip"><span class="k">id</span><code>{shortId(runId)}</code></span>
          <span class="chip"
            ><span class="k">tokens</span>{totals.tokens_in}↑ / {totals.tokens_out}↓</span
          >
          <span class="chip"><span class="k">cost</span>${totals.cost_usd.toFixed(4)}</span>
          <span class="chip"><span class="k">steps</span>{steps.length}</span>
          <span class="chip"><span class="k">tools</span>{toolCalls}</span>
          <span class="chip"><span class="k">elapsed</span>{formatDuration(elapsedMs)}</span>
          {#if TERMINAL.includes(status)}
            <button
              type="button"
              class="ghost"
              aria-expanded={expanded}
              onclick={() => (expanded = false)}
            >
              Collapse
            </button>
          {/if}
        </div>

        <div class="layer-2" data-layer="2">
          <span class="field"
            ><span class="k">mission_ref</span><span class="v">{phase ?? '—'}</span></span
          >
          <span class="field"
            ><span class="k">files</span><span class="v">{files.length}</span></span
          >
          <span class="field"
            ><span class="k">diff</span><span class="v"
              >+{diff?.lines_added ?? 0} / −{diff?.lines_removed ?? 0}</span
            ></span
          >
          <span class="field"
            ><span class="k">skill</span><span class="v">{skill ?? '—'}</span></span
          >
          <span class="field"><span class="k">mode</span><span class="v">—</span></span>
          <span class="field"
            ><span class="k">heartbeat</span><span class="v"
              >{lastHeartbeatTs ? lastHeartbeatTs.slice(11, 19) : '—'}</span
            ></span
          >
          <span class="field"><span class="k">worktree</span><span class="v">—</span></span>
          <span class="field"
            ><span class="k">doom_loop</span><span class="v">{doomCount}</span></span
          >
          <span class="field"
            ><span class="k">goal_drift</span><span class="v">{drift ?? '—'}</span></span
          >
          <span class="field"
            ><span class="k">checkpoint</span><span class="v"
              >{checkpoint ? shortId(checkpoint) : '—'}</span
            ></span
          >
          <span class="field"><span class="k">confidence</span><span class="v">—</span></span>
          <span class="field"><span class="k">demo</span><span class="v">—</span></span>
        </div>

        <div class="actions" data-actions={actions.join(' ')}>
          {#each actions as a (a)}
            <button type="button" class="act" onclick={() => act(a)}>{actionLabel(a)}</button>
          {/each}
          {#if actionNote}<span class="note" role="status">{actionNote}</span>{/if}
        </div>

        <button
          type="button"
          class="ghost layer-toggle"
          aria-expanded={layer3}
          onclick={() => (layer3 = !layer3)}
        >
          {layer3 ? 'Hide trail' : 'Show trail'}
        </button>

        {#if layer3}
          <div class="layer-3" data-layer="3">
            <ol class="steps">
              {#each steps as s (s.step)}
                <li class:last={s.step === steps.length - 1}>
                  <span class="dot {agentStepColor(s.action, s.verdict)}"></span>
                  <div class="body">
                    <div class="line">
                      <span class="action">{s.action}</span>
                      {#if s.verdict}
                        <span class="verdict {s.verdict}">{s.verdict}</span>
                      {/if}
                      <span class="tokens">{s.tokens_in}↑ {s.tokens_out}↓</span>
                    </div>
                    {#if s.observation}
                      <pre class="obs">{s.observation.slice(0, 240)}</pre>
                    {/if}
                  </div>
                </li>
              {/each}
            </ol>
          </div>
        {/if}
      {/if}
    </article>
  {/if}
</section>

<style>
  .agent-card {
    border: 1px solid var(--a-border);
    border-radius: 8px;
    background: var(--a-surface);
    padding: 0.85rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .agent-card:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .card-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 1rem;
    flex-wrap: wrap;
  }
  .identity {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .identity h3 {
    margin: 0;
    font-size: 1rem;
    color: var(--a-text);
  }
  .spine {
    font-size: 0.9rem;
    line-height: 1;
  }
  .status {
    font-size: 0.78rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
  }
  .head-meta {
    display: flex;
    gap: 0.75rem;
    flex-wrap: wrap;
  }
  .field {
    display: inline-flex;
    align-items: baseline;
    gap: 0.35rem;
    font-size: 0.75rem;
  }
  .field .k {
    color: var(--a-text-faint);
    text-transform: uppercase;
    letter-spacing: 0.03em;
    font-size: 0.66rem;
  }
  .field .v {
    color: var(--a-text-muted);
  }
  .tone-info {
    color: var(--a-info);
  }
  .tone-warn {
    color: var(--a-warn);
  }
  .tone-err {
    color: var(--a-err);
  }
  .tone-ok {
    color: var(--a-ok);
  }
  .tone-violet {
    color: var(--a-violet);
  }
  .tone-muted {
    color: var(--a-text-muted);
  }
  .tone-faint {
    color: var(--a-text-faint);
  }
  .status.tone-info {
    background: color-mix(in srgb, var(--a-info) 14%, transparent);
  }
  .status.tone-warn {
    background: color-mix(in srgb, var(--a-warn) 14%, transparent);
  }
  .status.tone-err {
    background: color-mix(in srgb, var(--a-err) 14%, transparent);
  }
  .status.tone-ok {
    background: color-mix(in srgb, var(--a-ok) 14%, transparent);
  }
  .status.tone-faint,
  .status.tone-muted {
    background: var(--a-surface-2);
  }
  .offline,
  .partial {
    margin: 0;
    font-size: 0.75rem;
    padding: 0.3rem 0.55rem;
    border-radius: 4px;
  }
  .offline {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
  }
  .partial {
    color: var(--a-text-muted);
    background: var(--a-surface-2);
  }
  .loading {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .skel {
    display: block;
    height: 0.8rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--a-text-faint) 22%, transparent);
  }
  .skel-title {
    width: 40%;
  }
  .skel-line {
    width: 90%;
  }
  .skel-line.short {
    width: 60%;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.85rem;
    color: var(--a-err);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.85rem;
    margin: 0;
  }
  .card-body {
    display: flex;
    flex-direction: column;
    gap: 0.55rem;
  }
  .layer-0 {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    flex-wrap: wrap;
  }
  .worked {
    font-size: 0.85rem;
    color: var(--a-text);
  }
  .statline-sm {
    font-size: 0.75rem;
    color: var(--a-text-faint);
  }
  .statline {
    display: flex;
    gap: 0.5rem;
    flex-wrap: wrap;
    align-items: center;
  }
  .chip {
    display: inline-flex;
    gap: 0.3rem;
    align-items: baseline;
    font-size: 0.75rem;
    color: var(--a-text-muted);
    background: var(--a-surface-2);
    padding: 0.15rem 0.5rem;
    border-radius: 4px;
  }
  .chip .k {
    color: var(--a-text-faint);
    font-size: 0.66rem;
    text-transform: uppercase;
  }
  .layer-2 {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(11rem, 1fr));
    gap: 0.3rem 0.9rem;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    flex-wrap: wrap;
    align-items: center;
  }
  .act {
    font-size: 0.78rem;
    padding: 0.2rem 0.6rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
  .act:hover {
    border-color: var(--a-primary);
  }
  .act:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .ghost {
    font-size: 0.75rem;
    background: none;
    border: none;
    color: var(--a-primary);
    cursor: pointer;
    padding: 0;
  }
  .ghost:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .note {
    font-size: 0.72rem;
    color: var(--a-text-faint);
  }
  .layer-toggle {
    align-self: flex-start;
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    max-height: 22rem;
    overflow-y: auto;
  }
  .steps li {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-top: 0.35rem;
    flex: 0 0 auto;
    background: var(--a-text-faint);
  }
  .dot.green {
    background: var(--a-ok);
  }
  .dot.blue {
    background: var(--a-info);
  }
  .dot.amber {
    background: var(--a-warn);
  }
  .dot.red {
    background: var(--a-err);
  }
  .body {
    flex: 1 1 auto;
    min-width: 0;
  }
  .line {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    font-size: 0.82rem;
  }
  .action {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-text);
  }
  .verdict {
    font-size: 0.7rem;
    padding: 0.02rem 0.4rem;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .verdict.pass {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
  }
  .verdict.fail {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
  }
  .verdict.unknown {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
  }
  .tokens {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--a-text-faint);
  }
  .obs {
    margin: 0.15rem 0 0 0;
    padding: 0.3rem 0.5rem;
    background: var(--a-bg);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.72rem;
    color: var(--a-text-muted);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 6rem;
    overflow-y: auto;
  }
</style>

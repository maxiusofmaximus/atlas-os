<script lang="ts">
  import { hud, type HudEvent } from '$stores/hud';

  interface Props {
    hudUrl?: string | null;
    maxEvents?: number;
    onSelect?: (event: HudEvent) => void;
  }

  const { hudUrl = null, maxEvents = 50, onSelect }: Props = $props();

  const ANNOUNCE_THROTTLE_MS = 1000;

  type EventTone = 'ok' | 'warn' | 'err' | 'violet' | 'info' | 'unknown';

  const KNOWN_KINDS = new Set([
    'task_received',
    'mission_consolidated',
    'mission_locked',
    'plan_generated',
    'agent_status_changed',
    'agent_diff',
    'agent_tokens',
    'agent_step',
    'agent_heartbeat',
    'approval_request',
    'approval_decision',
    'doom_loop_detected',
    'goal_drift_detected',
    'journal_checkpoint',
    'cost_threshold_crossed',
    'worktree_dirty',
    'skill_activated',
    'artifact_preview_opened',
    'research_completed',
    'hud_served',
    'mission_steered',
    'model_swapped',
    'step_phase_changed',
    'autoresearch_cancelled',
    'spend_limit_observed',
    'hardware_snapshot',
  ]);

  function eventTone(kind: string): EventTone {
    if (!KNOWN_KINDS.has(kind)) return 'unknown';
    switch (kind) {
      case 'doom_loop_detected':
      case 'goal_drift_detected':
      case 'autoresearch_cancelled':
        return 'err';
      case 'approval_request':
      case 'approval_decision':
      case 'cost_threshold_crossed':
      case 'worktree_dirty':
      case 'spend_limit_observed':
        return 'warn';
      case 'agent_diff':
      case 'mission_consolidated':
        return 'ok';
      case 'research_completed':
      case 'model_swapped':
      case 'skill_activated':
        return 'violet';
      default:
        return 'info';
    }
  }

  const GLYPHS: Record<EventTone, string> = {
    ok: '✓',
    warn: '!',
    err: '⚠',
    violet: '✦',
    info: '•',
    unknown: '?',
  };

  function pick(payload: unknown): Record<string, unknown> {
    return (payload ?? {}) as Record<string, unknown>;
  }

  function actorOf(p: Record<string, unknown>): string {
    const a = p.agent_id ?? p.agent ?? p.mission_id ?? p.run_id ?? p.approval_id;
    return typeof a === 'string' && a.length > 0 ? a : 'system';
  }

  function textOf(kind: string, p: Record<string, unknown>): string {
    switch (kind) {
      case 'doom_loop_detected':
        return `doom loop detected (count ${p.count ?? '?'})`;
      case 'goal_drift_detected':
        return `goal drift detected (${p.drift ?? '?'})`;
      case 'approval_request':
        return `approval requested: ${p.action ?? 'action'}`;
      case 'approval_decision':
        return `approval ${p.decision ?? 'decided'}`;
      case 'agent_status_changed':
        return `status ${p.status ?? 'unknown'}`;
      case 'agent_diff':
        return `diff ${p.files ?? '?'} files (+${p.lines_added ?? 0}/-${p.lines_removed ?? 0})`;
      case 'agent_tokens':
        return `tokens in ${p.tokens_in ?? 0} / out ${p.tokens_out ?? 0}`;
      case 'agent_step':
        return `step ${p.step ?? '?'}: ${p.action ?? 'action'}`;
      case 'agent_heartbeat':
        return 'heartbeat';
      case 'task_received':
        return 'task received';
      case 'mission_consolidated':
        return `mission consolidated (${p.confidence ?? '?'})`;
      case 'mission_locked':
        return 'mission locked';
      case 'plan_generated':
        return 'plan generated';
      case 'journal_checkpoint':
        return 'journal checkpoint';
      case 'cost_threshold_crossed':
        return `cost crossed ${p.threshold ?? '?'} (${p.cumulative ?? '?'})`;
      case 'worktree_dirty':
        return `worktree dirty: ${p.path ?? '?'}`;
      case 'skill_activated':
        return `skill activated: ${p.skill_id ?? '?'}`;
      case 'artifact_preview_opened':
        return `artifact opened: ${p.artifact ?? '?'}`;
      case 'research_completed':
        return 'research completed';
      case 'hud_served':
        return `hud served on ${p.hud_port ?? '?'}`;
      case 'mission_steered':
        return `steer: ${p.message ?? ''}`;
      case 'model_swapped':
        return `model ${p.prev_model_id ?? '?'} to ${p.new_model_id ?? '?'}`;
      case 'step_phase_changed':
        return `phase ${p.phase ?? '?'}`;
      case 'autoresearch_cancelled':
        return `autoresearch ${p.outcome ?? 'cancelled'}`;
      case 'spend_limit_observed':
        return `spend limit: ${p.provider ?? '?'}/${p.model ?? '?'}`;
      case 'hardware_snapshot':
        return `ram ${p.ram_used_mb ?? '?'}/${p.ram_total_mb ?? '?'} MB`;
      default:
        return 'unknown event';
    }
  }

  function shortId(id: string): string {
    return id.length > 8 ? id.slice(0, 8) : id;
  }

  let canvasSync = $state(false);
  let activeIndex = $state(-1);
  let announced = $state('');
  let lastAnnounceAt = 0;
  let listEl = $state<HTMLUListElement | undefined>(undefined);

  const connected = $derived($hud.connected);
  const all = $derived($hud.events);
  const visible = $derived([...all].slice(-maxEvents).reverse());
  const truncated = $derived(all.length >= maxEvents);
  const loading = $derived(all.length === 0 && !connected && hudUrl !== null);

  const rows = $derived(
    visible.map((evt) => {
      const p = pick(evt.payload);
      const tone = eventTone(evt.kind);
      return { evt, tone, glyph: GLYPHS[tone], actor: actorOf(p), text: textOf(evt.kind, p) };
    }),
  );

  $effect(() => {
    const latest = all[all.length - 1];
    if (!latest) return;
    const now = Date.now();
    if (now - lastAnnounceAt < ANNOUNCE_THROTTLE_MS) return;
    lastAnnounceAt = now;
    const p = pick(latest.payload);
    announced = `${shortId(actorOf(p))} ${textOf(latest.kind, p)}`;
  });

  function select(evt: HudEvent): void {
    onSelect?.(evt);
  }

  function retry(): void {
    if (hudUrl) hud.connect(hudUrl);
  }

  function focusRow(index: number): void {
    if (rows.length === 0) return;
    const clamped = Math.max(0, Math.min(index, rows.length - 1));
    activeIndex = clamped;
    const buttons = listEl?.querySelectorAll<HTMLButtonElement>('.row-btn');
    buttons?.[clamped]?.focus();
  }

  function onRowKey(e: KeyboardEvent, index: number): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      focusRow(index + 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      focusRow(index - 1);
    } else if (e.key === 'Enter') {
      const row = rows[index];
      if (!row) return;
      e.preventDefault();
      select(row.evt);
    }
  }
</script>

<section class="activity-spine" aria-label="Activity Spine">
  <header>
    <span class="dot" class:on={connected} aria-hidden="true"></span>
    <span class="state">{connected ? 'live' : 'off'}</span>
    <button
      type="button"
      class="canvas-toggle"
      aria-pressed={canvasSync}
      onclick={() => (canvasSync = !canvasSync)}
    >
      ⇄ canvas
    </button>
  </header>

  {#if !connected}
    <p class="disconnected" role="alert">
      <span>stream down · reconnecting 1s</span>
      <button type="button" class="retry" onclick={retry} disabled={!hudUrl}>Retry</button>
    </p>
  {/if}

  {#if truncated}
    <p class="partial">showing last {maxEvents}</p>
  {/if}

  <span class="sr-only" aria-live="polite">{announced}</span>

  {#if loading}
    <ul class="rows skeleton" aria-hidden="true">
      {#each [0, 1, 2, 3] as n (n)}
        <li class="row"><span class="bar"></span></li>
      {/each}
    </ul>
  {:else if rows.length === 0}
    <p class="empty">Sin actividad todavía</p>
  {:else}
    <ul class="rows" bind:this={listEl}>
      {#each rows as row, i (row.evt.id)}
        <li class="row-item">
          <button
            type="button"
            class="row-btn tone-{row.tone}"
            class:active={i === activeIndex}
            onclick={() => select(row.evt)}
            onkeydown={(e) => onRowKey(e, i)}
          >
            <span class="glyph" aria-hidden="true">{row.glyph}</span>
            <span class="actor">{shortId(row.actor)}</span>
            <span class="text">{row.text}</span>
            <span class="ts">{row.evt.ts}</span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .activity-spine {
    border: 1px solid var(--a-border);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .dot {
    width: 0.55rem;
    height: 0.55rem;
    border-radius: 50%;
    background: var(--a-text-faint);
  }
  .dot.on {
    background: var(--a-primary);
    animation: live-pulse 2s ease-in-out infinite;
  }
  .state {
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--a-text-muted);
  }
  .canvas-toggle {
    margin-left: auto;
    padding: 0.2rem 0.5rem;
    background: var(--a-surface-2);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.72rem;
  }
  .canvas-toggle[aria-pressed='true'] {
    color: var(--a-primary);
    border-color: color-mix(in srgb, var(--a-primary) 40%, transparent);
  }
  .canvas-toggle:focus-visible,
  .retry:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .disconnected {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    margin: 0;
    font-size: 0.75rem;
    color: var(--a-err);
  }
  .retry {
    padding: 0.15rem 0.5rem;
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
    color: var(--a-err);
    border: 1px solid color-mix(in srgb, var(--a-err) 35%, transparent);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.72rem;
  }
  .partial {
    margin: 0;
    font-size: 0.7rem;
    color: var(--a-text-faint);
  }
  .empty {
    margin: 0;
    font-size: 0.85rem;
    color: var(--a-text-muted);
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
  }
  .row-item {
    display: flex;
  }
  .row-btn {
    display: grid;
    grid-template-columns: 1.1rem auto 1fr auto;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    padding: 0.3rem 0.45rem;
    background: transparent;
    color: var(--a-text);
    border: 1px solid transparent;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.78rem;
  }
  .row-btn:hover,
  .row-btn.active {
    background: var(--a-surface-2);
    border-color: var(--a-border);
  }
  .row-btn:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: -1px;
  }
  .glyph {
    text-align: center;
  }
  .actor {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.68rem;
    color: var(--a-text-faint);
  }
  .text {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .ts {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.66rem;
    color: var(--a-text-faint);
    white-space: nowrap;
  }
  .tone-ok {
    color: var(--a-ok);
  }
  .tone-warn {
    color: var(--a-warn);
  }
  .tone-err {
    color: var(--a-err);
  }
  .tone-violet {
    color: var(--a-violet);
  }
  .tone-info {
    color: var(--a-text-muted);
  }
  .tone-unknown {
    color: var(--a-text-faint);
  }
  .tone-err .glyph {
    font-weight: 700;
  }
  .skeleton .row {
    padding: 0.3rem 0.45rem;
  }
  .bar {
    display: block;
    height: 0.8rem;
    border-radius: 4px;
    background: var(--a-surface-2);
    animation: skeleton-shimmer 1.4s ease-in-out infinite;
  }
  .sr-only {
    position: absolute;
    width: 1px;
    height: 1px;
    padding: 0;
    margin: -1px;
    overflow: hidden;
    clip: rect(0, 0, 0, 0);
    white-space: nowrap;
    border: 0;
  }
  @keyframes live-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
  @keyframes skeleton-shimmer {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .dot.on,
    .bar {
      animation: none;
    }
  }
</style>

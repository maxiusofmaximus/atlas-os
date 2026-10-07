<script lang="ts">
  import { hud, fetchMissions, type MissionRow } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
    activeMissionId?: string | null;
    onselect?: (missionId: string) => void;
    onnew?: () => void;
  }

  const { hudUrl, activeMissionId = null, onselect, onnew }: Props = $props();

  type Severity = 'err' | 'warn' | 'info' | 'ok' | 'faint' | 'muted';

  function rollup(status: string): { sev: Severity; glyph: string; label: string } {
    const s = status.toLowerCase();
    if (s.includes('doom')) return { sev: 'err', glyph: '⚠', label: 'blocked' };
    if (s.includes('fail') || s.includes('error') || s.includes('abort'))
      return { sev: 'err', glyph: '✖', label: 'failed' };
    if (s.includes('paus') || s.includes('review'))
      return { sev: 'warn', glyph: '‖', label: 'paused' };
    if (
      s.includes('done') ||
      s.includes('success') ||
      s.includes('complet') ||
      s.includes('consolidat')
    )
      return { sev: 'ok', glyph: '✓', label: 'done' };
    if (s.includes('run') || s.includes('read') || s.includes('plan') || s.includes('cod'))
      return { sev: 'info', glyph: '●', label: 'running' };
    if (s.includes('queue') || s.includes('pend') || s.includes('new') || s.includes('receiv'))
      return { sev: 'faint', glyph: '○', label: 'queued' };
    if (s.includes('idle')) return { sev: 'muted', glyph: '◌', label: 'idle' };
    return { sev: 'faint', glyph: '?', label: 'unknown' };
  }

  let missions = $state<MissionRow[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);
  let navEl: HTMLElement | undefined = $state();

  async function load(url: string | null, silent = false): Promise<void> {
    if (!url) {
      missions = [];
      loading = false;
      error = 'no HUD URL yet';
      return;
    }
    if (!silent) loading = true;
    error = null;
    try {
      missions = await fetchMissions(url);
      loading = false;
    } catch (err) {
      missions = [];
      loading = false;
      error = err instanceof Error ? err.message : String(err);
    }
  }

  function onKeydown(e: KeyboardEvent): void {
    if (!navEl) return;
    const items = Array.from(navEl.querySelectorAll<HTMLButtonElement>('.item'));
    if (items.length === 0) return;
    const idx = items.indexOf(e.currentTarget as HTMLButtonElement);
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      items[Math.min(idx + 1, items.length - 1)]?.focus();
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      items[Math.max(idx - 1, 0)]?.focus();
    } else if (e.key === 'Home') {
      e.preventDefault();
      items[0]?.focus();
    } else if (e.key === 'End') {
      e.preventDefault();
      items[items.length - 1]?.focus();
    }
  }

  const REFRESH_MS = 4000;

  $effect(() => {
    void load(hudUrl);
    const timer = setInterval(() => void load(hudUrl, true), REFRESH_MS);
    return () => clearInterval(timer);
  });
</script>

<nav class="mission-rail" data-region="mission-rail" aria-label="Missions" bind:this={navEl}>
  <header class="rail-head">
    <h2>Missions</h2>
    <button class="new" type="button" onclick={() => onnew?.()}>+ New</button>
  </header>

  <p class="conn" data-state={$hud.connected ? 'online' : 'offline'}>
    <span class="dot" aria-hidden="true"></span>
    {$hud.connected ? 'live' : 'reconnecting'}
  </p>

  {#if loading}
    <ul class="skeleton-list">
      {#each [1, 2, 3] as n (n)}
        <li class="skeleton" aria-hidden="true"></li>
      {/each}
    </ul>
  {:else if error}
    <p class="error">
      no se pudo cargar misiones · <button type="button" onclick={() => load(hudUrl)}>Retry</button>
    </p>
  {:else if missions.length === 0}
    <p class="empty">
      Sin misiones. <button type="button" onclick={() => onnew?.()}>+ New Mission</button>
    </p>
  {:else}
    <ul class="missions">
      {#each missions as m (m.id)}
        {@const r = rollup(m.status)}
        <li>
          <button
            class="item"
            type="button"
            data-sev={r.sev}
            aria-current={m.id === activeMissionId ? 'true' : undefined}
            onclick={() => onselect?.(m.id)}
            onkeydown={onKeydown}
          >
            <span class="spine" aria-hidden="true"></span>
            <span class="name">{m.label}</span>
            <span class="badge" data-sev={r.sev} aria-label={r.label}>
              <span aria-hidden="true">{r.glyph}</span>
              {r.label}
            </span>
          </button>
        </li>
      {/each}
    </ul>
  {/if}
</nav>

<style>
  .mission-rail {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    min-width: 200px;
    border-right: 1px solid var(--a-border);
    padding-right: 0.75rem;
  }
  .rail-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    gap: 0.5rem;
  }
  .rail-head h2 {
    font-size: 0.85rem;
    margin: 0;
    color: var(--a-info);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .new {
    background: transparent;
    color: var(--a-primary);
    border: 1px solid var(--a-border-ui);
    border-radius: 6px;
    padding: 0.15rem 0.5rem;
    font-size: 0.75rem;
    cursor: pointer;
  }
  .new:hover {
    background: var(--a-surface-2);
  }
  .conn {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    margin: 0;
    font-size: 0.68rem;
    color: var(--a-text-muted);
    text-transform: lowercase;
  }
  .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--a-text-faint);
  }
  .conn[data-state='online'] .dot {
    background: var(--a-ok);
  }
  .conn[data-state='offline'] .dot {
    background: var(--a-text-faint);
  }
  .skeleton-list,
  .missions {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .skeleton {
    height: 2.1rem;
    border-radius: 6px;
    background: var(--a-surface-2);
  }
  .item {
    display: grid;
    grid-template-columns: 3px 1fr auto;
    align-items: center;
    gap: 0.5rem;
    width: 100%;
    text-align: left;
    background: transparent;
    border: 1px solid transparent;
    border-radius: 6px;
    padding: 0.35rem 0.5rem;
    color: var(--a-text);
    cursor: pointer;
    font-size: 0.82rem;
  }
  .item:hover {
    background: var(--a-surface-2);
  }
  .item:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .item[aria-current='true'] {
    background: color-mix(in srgb, var(--a-primary) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-primary) 35%, transparent);
  }
  .spine {
    width: 3px;
    height: 1.4rem;
    border-radius: 2px;
    background: var(--a-text-faint);
  }
  .name {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .badge {
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    font-size: 0.68rem;
    color: var(--a-text-muted);
  }
  .item[data-sev='err'] .spine {
    background: var(--a-err);
  }
  .item[data-sev='warn'] .spine {
    background: var(--a-warn);
  }
  .item[data-sev='info'] .spine {
    background: var(--a-info);
  }
  .item[data-sev='ok'] .spine {
    background: var(--a-ok);
  }
  .item[data-sev='faint'] .spine {
    background: var(--a-text-faint);
  }
  .item[data-sev='muted'] .spine {
    background: var(--a-text-muted);
  }
  .badge[data-sev='err'] {
    color: var(--a-err);
  }
  .badge[data-sev='warn'] {
    color: var(--a-warn);
  }
  .badge[data-sev='info'] {
    color: var(--a-info);
  }
  .badge[data-sev='ok'] {
    color: var(--a-ok);
  }
  .badge[data-sev='faint'],
  .badge[data-sev='muted'] {
    color: var(--a-text-faint);
  }
  .error,
  .empty {
    margin: 0;
    font-size: 0.78rem;
    color: var(--a-text-muted);
  }
  .error {
    color: var(--a-err);
  }
  .error button,
  .empty button {
    background: transparent;
    border: 1px solid var(--a-border-ui);
    border-radius: 6px;
    color: var(--a-primary);
    cursor: pointer;
    font-size: 0.72rem;
    padding: 0.1rem 0.4rem;
  }
  .new:focus-visible,
  .error button:focus-visible,
  .empty button:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
</style>

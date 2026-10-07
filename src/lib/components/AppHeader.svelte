<script lang="ts">
  import { hud, type RemoteAccessStatus } from '$stores/hud';

  interface MissionRef {
    id: string;
    rollup: string;
  }

  interface Props {
    version: string;
    mission?: MissionRef | null;
    remote?: RemoteAccessStatus | null;
  }

  const { version, mission = null, remote = null }: Props = $props();

  const connected = $derived($hud.connected);
  const busState = $derived(connected ? 'connected' : 'disconnected');
  const busGlyph = $derived(connected ? '●' : '○');
  const busLabel = $derived(connected ? 'connected' : 'disconnected');
  const lastFrameTs = $derived(
    $hud.events.length > 0 ? ($hud.events[$hud.events.length - 1]?.ts ?? null) : null,
  );
</script>

<header class="app-header">
  <span class="brand">
    <span class="brand-mark" aria-hidden="true"></span>
    <h1 class="brand-name">Atlas OS</h1>
    <span class="brand-version">v{version}</span>
  </span>

  <span class="active-mission" data-state={mission ? 'set' : 'none'}>
    <span class="rollup-dot" data-state={mission?.rollup ?? 'none'} aria-hidden="true"></span>
    <span class="mission-label">Mission:</span>
    <code class="mission-id">{mission?.id ?? '—'}</code>
  </span>

  <span class="bus" data-state={busState} role="status" aria-live="polite">
    <span class="glyph" aria-hidden="true">{busGlyph}</span>
    <span class="label">{busLabel}</span>
  </span>

  {#if remote}
    <span class="remote" data-state={remote.local_only ? 'local' : 'remote'}>
      {remote.local_only ? 'local-only' : 'remote'}
      {#if remote.oidc_configured}· OIDC{/if}
      {#if remote.token_configured}· bearer{/if}
    </span>
  {/if}

  <span class="leader-hint" aria-hidden="true">
    <kbd class="leader">:</kbd>
    <kbd>:v</kbd><kbd>:a</kbd><kbd>:n</kbd><kbd>:m</kbd><kbd>:d</kbd><kbd>:?</kbd>
  </span>

  {#if !connected}
    <p class="bus-banner" role="status">
      sin conexión — reconectando{#if lastFrameTs}
        · último frame {lastFrameTs.slice(11, 19)}{/if}
    </p>
  {/if}
</header>

<style>
  .app-header {
    display: flex;
    align-items: center;
    gap: 0.85rem;
    flex-wrap: wrap;
    background: var(--a-surface);
    border-bottom: 1px solid var(--a-border);
    padding: 0.5rem 0.85rem;
  }
  .brand {
    display: inline-flex;
    align-items: center;
    gap: 0.5rem;
  }
  .brand-mark {
    width: 3px;
    height: 1.2rem;
    border-radius: 2px;
    background: var(--a-primary);
  }
  .brand-name {
    margin: 0;
    font-size: 1.05rem;
    color: var(--a-text);
  }
  .brand-version {
    font-family: var(--a-mono);
    font-size: 0.72rem;
    color: var(--a-text-muted);
  }
  .active-mission {
    display: inline-flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.78rem;
  }
  .rollup-dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 50%;
    background: var(--a-text-faint);
  }
  .rollup-dot[data-state='ok'] {
    background: var(--a-ok);
  }
  .rollup-dot[data-state='info'] {
    background: var(--a-info);
  }
  .rollup-dot[data-state='warn'] {
    background: var(--a-warn);
  }
  .rollup-dot[data-state='err'] {
    background: var(--a-err);
  }
  .mission-label {
    color: var(--a-text-muted);
  }
  .mission-id {
    font-family: var(--a-mono);
    color: var(--a-text);
  }
  .bus {
    display: inline-flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.76rem;
  }
  .bus .glyph {
    line-height: 1;
  }
  .bus[data-state='connected'] {
    color: var(--a-ok);
  }
  .bus[data-state='connected'] .glyph {
    animation: bus-pulse 2s ease-in-out infinite;
  }
  .bus[data-state='disconnected'] {
    color: var(--a-err);
  }
  .bus[data-state='unknown'] {
    color: var(--a-text-faint);
  }
  .remote {
    font-family: var(--a-mono);
    font-size: 0.72rem;
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
    border: 1px solid var(--a-border);
    color: var(--a-text-muted);
  }
  .remote[data-state='remote'] {
    color: var(--a-info);
    border-color: var(--a-info);
  }
  .leader-hint {
    margin-left: auto;
    display: inline-flex;
    align-items: center;
    gap: 0.25rem;
    color: var(--a-text-faint);
    font-size: 0.68rem;
  }
  .leader-hint kbd {
    font-family: var(--a-mono);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.02rem 0.3rem;
  }
  .leader-hint .leader {
    color: var(--a-primary);
    border-color: var(--a-primary);
  }
  .bus-banner {
    flex-basis: 100%;
    margin: 0;
    font-size: 0.74rem;
    color: var(--a-err);
  }
  @keyframes bus-pulse {
    0%,
    100% {
      opacity: 1;
    }
    50% {
      opacity: 0.4;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .bus[data-state='connected'] .glyph {
      animation: none;
    }
  }
</style>

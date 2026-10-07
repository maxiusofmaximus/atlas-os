<script lang="ts">
  import { hud, projectPendingApprovals, approveApproval, denyApproval } from '$stores/hud';

  interface Props {
    hudUrl?: string | null;
    onResolved?: (id: string, decision: 'approve' | 'deny') => void;
  }

  const { hudUrl = null, onResolved }: Props = $props();

  const GATE_ACTIONS = new Set([
    'network_binding',
    'file_write_outside_repo',
    'install',
    'exec',
    'delete',
    'secret_access',
    'git_push',
  ]);

  type Channel = 'gate' | 'question' | 'unknown';

  function channelOf(action: string): Channel {
    if (!action || action.trim() === '') return 'unknown';
    return GATE_ACTIONS.has(action) ? 'gate' : 'question';
  }

  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);
  let selected = $state<Record<string, boolean>>({});

  const connected = $derived($hud.connected);
  const pending = $derived(projectPendingApprovals($hud.events));
  const gates = $derived(pending.filter((p) => channelOf(p.action) === 'gate'));
  const questions = $derived(pending.filter((p) => channelOf(p.action) === 'question'));
  const unknowns = $derived(pending.filter((p) => channelOf(p.action) === 'unknown'));
  const selectedIds = $derived(Object.keys(selected).filter((k) => selected[k]));
  const loading = $derived(pending.length === 0 && !connected && hudUrl !== null);

  async function answer(id: string, decision: 'approve' | 'deny'): Promise<void> {
    busy = id;
    error = null;
    try {
      if (decision === 'approve') {
        await approveApproval(hudUrl, id);
      } else {
        await denyApproval(hudUrl, id);
      }
      const next = { ...selected };
      delete next[id];
      selected = next;
      onResolved?.(id, decision);
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      busy = null;
    }
  }

  function toggleSelect(id: string): void {
    selected = { ...selected, [id]: !selected[id] };
  }

  function onRowKey(e: KeyboardEvent, id: string): void {
    if (e.key === 'a') {
      e.preventDefault();
      void answer(id, 'approve');
    } else if (e.key === 'd') {
      e.preventDefault();
      void answer(id, 'deny');
    } else if (e.key === ' ') {
      e.preventDefault();
      toggleSelect(id);
    }
  }

  function shortId(id: string): string {
    return id.length > 8 ? id.slice(0, 8) : id;
  }
</script>

<section class="approvals-dock" aria-label="Approvals Dock">
  <header>
    <h3>Approvals</h3>
    <span class="badge" class:pending={pending.length > 0}>{pending.length}</span>
  </header>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if !connected}
    <p class="frozen" role="alert">stream down · dock frozen</p>
  {/if}

  {#if loading}
    <ul class="rows skeleton" aria-hidden="true">
      {#each [0, 1] as n (n)}
        <li class="row"><span class="bar"></span></li>
      {/each}
    </ul>
  {:else if pending.length === 0}
    <p class="empty">No pending approvals.</p>
  {:else}
    {#if gates.length > 0}
      <div class="channel gate" aria-live="assertive">
        <h4>Gate · blocks the agent</h4>
        <ul class="rows">
          {#each gates as p (p.approval_id)}
            <li class="row">
              <button
                type="button"
                class="row-btn"
                aria-pressed={selected[p.approval_id] ?? false}
                onclick={() => toggleSelect(p.approval_id)}
                onkeydown={(e) => onRowKey(e, p.approval_id)}
              >
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">{p.action}</code>
                <span class="pattern">pattern {p.action}</span>
                <span class="scope">scope —</span>
              </button>
              <div class="buttons">
                <button
                  type="button"
                  class="apr"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'approve')}
                >
                  Apr
                </button>
                <button
                  type="button"
                  class="deny"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'deny')}
                >
                  Deny
                </button>
                <button type="button" class="steer" disabled title="needs H-02">Steer</button>
                <button type="button" class="fork" disabled title="needs H-02">Fork</button>
              </div>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if questions.length > 0}
      <div class="channel question" aria-live="polite">
        <h4>Question · async</h4>
        <ul class="rows">
          {#each questions as p (p.approval_id)}
            <li class="row">
              <button
                type="button"
                class="row-btn"
                aria-pressed={selected[p.approval_id] ?? false}
                onclick={() => toggleSelect(p.approval_id)}
                onkeydown={(e) => onRowKey(e, p.approval_id)}
              >
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">{p.action}</code>
                <span class="pattern">pattern {p.action}</span>
                <span class="scope">scope —</span>
              </button>
              <div class="buttons">
                <button
                  type="button"
                  class="apr"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'approve')}
                >
                  Apr
                </button>
                <button
                  type="button"
                  class="deny"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'deny')}
                >
                  Deny
                </button>
                <button type="button" class="steer" disabled title="needs H-02">Steer</button>
                <button type="button" class="fork" disabled title="needs H-02">Fork</button>
              </div>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if unknowns.length > 0}
      <div class="channel unknown" aria-live="assertive">
        <h4>Unknown · requires explicit decision</h4>
        <ul class="rows">
          {#each unknowns as p (p.approval_id)}
            <li class="row">
              <button
                type="button"
                class="row-btn"
                aria-pressed={selected[p.approval_id] ?? false}
                onclick={() => toggleSelect(p.approval_id)}
                onkeydown={(e) => onRowKey(e, p.approval_id)}
              >
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">unclassifiable</code>
                <span class="pattern">pattern —</span>
                <span class="scope">scope —</span>
              </button>
              <div class="buttons">
                <button
                  type="button"
                  class="apr"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'approve')}
                >
                  Apr
                </button>
                <button
                  type="button"
                  class="deny"
                  disabled={busy === p.approval_id}
                  onclick={() => void answer(p.approval_id, 'deny')}
                >
                  Deny
                </button>
                <button type="button" class="steer" disabled title="needs H-02">Steer</button>
                <button type="button" class="fork" disabled title="needs H-02">Fork</button>
              </div>
            </li>
          {/each}
        </ul>
      </div>
    {/if}

    {#if pending.length > 1}
      <div class="batch">
        <span class="count">{selectedIds.length} selected</span>
        <button
          type="button"
          class="approve-all"
          disabled
          title="batch approval requires H-02 (not in v1)"
        >
          Approve all
        </button>
        <span class="batch-note">batch disabled · needs H-02</span>
      </div>
    {/if}
  {/if}
</section>

<style>
  .approvals-dock {
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
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
  }
  h4 {
    margin: 0 0 0.3rem 0;
    font-size: 0.72rem;
    text-transform: uppercase;
    letter-spacing: 0.05em;
    color: var(--a-text-muted);
  }
  .badge {
    font-size: 0.75rem;
    padding: 0.05rem 0.5rem;
    border-radius: 999px;
    background: var(--a-surface-2);
    color: var(--a-text-faint);
  }
  .badge.pending {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 15%, transparent);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.85rem;
    margin: 0;
  }
  .frozen {
    margin: 0;
    font-size: 0.75rem;
    color: var(--a-warn);
  }
  .channel {
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .gate {
    border-left: 3px solid var(--a-warn);
    padding-left: 0.5rem;
  }
  .unknown {
    border-left: 3px solid var(--a-err);
    padding-left: 0.5rem;
  }
  .rows {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid var(--a-surface-2);
    border-radius: 5px;
    padding: 0.35rem 0.5rem;
  }
  .row-btn {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
    flex: 1 1 auto;
    text-align: left;
    background: transparent;
    color: var(--a-text);
    border: 1px solid transparent;
    border-radius: 4px;
    padding: 0.15rem 0.25rem;
    cursor: pointer;
  }
  .row-btn:hover {
    background: var(--a-surface-2);
  }
  .row-btn:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: -1px;
  }
  .row-btn[aria-pressed='true'] {
    border-color: color-mix(in srgb, var(--a-primary) 45%, transparent);
  }
  .actor {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.68rem;
    color: var(--a-text-faint);
  }
  .action {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.82rem;
    color: var(--a-warn);
  }
  .pattern,
  .scope {
    font-size: 0.66rem;
    color: var(--a-text-faint);
  }
  .buttons {
    display: flex;
    gap: 0.3rem;
    flex: 0 0 auto;
  }
  .buttons button {
    border: 1px solid transparent;
    border-radius: 5px;
    font-size: 0.76rem;
    padding: 0.22rem 0.55rem;
    cursor: pointer;
  }
  .apr {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-ok) 35%, transparent);
  }
  .deny {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-err) 35%, transparent);
  }
  .steer {
    color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 10%, transparent);
  }
  .fork {
    color: var(--a-violet);
    background: color-mix(in srgb, var(--a-violet) 10%, transparent);
  }
  .buttons button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .batch {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    border-top: 1px solid var(--a-surface-2);
    padding-top: 0.4rem;
    font-size: 0.72rem;
    color: var(--a-text-muted);
  }
  .approve-all {
    padding: 0.2rem 0.55rem;
    border-radius: 5px;
    border: 1px solid var(--a-border);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: not-allowed;
  }
  .batch-note {
    color: var(--a-text-faint);
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .skeleton .bar {
    display: block;
    height: 0.9rem;
    width: 100%;
    border-radius: 4px;
    background: var(--a-surface-2);
  }
  @media (prefers-reduced-motion: reduce) {
    .approvals-dock * {
      transition: none;
    }
  }
</style>

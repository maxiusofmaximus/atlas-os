<script lang="ts">
  import { hud, projectPendingApprovals, approveApproval, denyApproval } from '$stores/hud';
  import { SvelteMap, SvelteSet } from 'svelte/reactivity';

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
  let reason = $state('');
  let batchBusy = $state(false);
  let batchError = $state<string | null>(null);
  let conflictIds = $state<string[]>([]);
  let lastDecision = $state<'approve' | 'deny' | null>(null);

  const connected = $derived($hud.connected);
  const pending = $derived(projectPendingApprovals($hud.events));
  const gates = $derived(pending.filter((p) => channelOf(p.action) === 'gate'));
  const questions = $derived(pending.filter((p) => channelOf(p.action) === 'question'));
  const unknowns = $derived(pending.filter((p) => channelOf(p.action) === 'unknown'));
  const selectedIds = $derived(
    pending.filter((p) => selected[p.approval_id]).map((p) => p.approval_id),
  );
  const loading = $derived(pending.length === 0 && !connected && hudUrl !== null);

  function filesOf(id: string): string[] {
    const item = pending.find((p) => p.approval_id === id) as { files?: unknown } | undefined;
    const raw = item?.files;
    return Array.isArray(raw) ? raw.filter((f): f is string => typeof f === 'string') : [];
  }

  const fileConflicts = $derived.by(() => {
    const owner = new SvelteMap<string, string[]>();
    for (const id of selectedIds) {
      for (const file of filesOf(id)) {
        const ids = owner.get(file) ?? [];
        ids.push(id);
        owner.set(file, ids);
      }
    }
    const out: Record<string, string[]> = {};
    for (const [file, ids] of owner) {
      if (ids.length > 1) out[file] = ids;
    }
    return out;
  });

  const conflictingIds = $derived.by(() => {
    const set = new SvelteSet<string>();
    for (const ids of Object.values(fileConflicts)) {
      for (const id of ids) set.add(id);
    }
    return [...set];
  });

  const cleanCount = $derived(selectedIds.length - conflictingIds.length);
  const hasConflict = $derived(conflictingIds.length > 0);
  const canBatch = $derived(selectedIds.length > 0 && !hasConflict && !batchBusy && connected);

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

  function selectAll(): void {
    const next: Record<string, boolean> = {};
    for (const p of pending) next[p.approval_id] = true;
    selected = next;
  }

  function clearSelection(): void {
    selected = {};
  }

  async function runBatch(decision: 'approve' | 'deny'): Promise<void> {
    if (!canBatch) return;
    const ids = selectedIds;
    batchBusy = true;
    batchError = null;
    conflictIds = [];
    lastDecision = decision;
    try {
      const base = (hudUrl ?? '').replace(/\/$/, '');
      if (!base) {
        batchError = 'No se pudieron procesar: reintentar';
        return;
      }
      const payload: Record<string, unknown> = {
        decision,
        items: ids.map((id) => ({ approval_id: id, files: filesOf(id) })),
      };
      const trimmed = reason.trim();
      if (trimmed) payload.reason = trimmed;
      const res = await fetch(`${base}/hud/approvals/batch`, {
        method: 'POST',
        headers: { 'content-type': 'application/json' },
        body: JSON.stringify(payload),
      });
      if (res.status === 409) {
        conflictIds = ids;
        batchError = 'Conflicto: ya fue resuelta por otra sesión';
        return;
      }
      if (!res.ok) {
        batchError = `No se pudieron procesar: reintentar (${res.status})`;
        return;
      }
      selected = {};
      reason = '';
      for (const id of ids) onResolved?.(id, decision);
    } catch (e) {
      batchError = e instanceof Error ? e.message : String(e);
    } finally {
      batchBusy = false;
    }
  }

  function retryBatch(): void {
    if (lastDecision && selectedIds.length > 0) {
      void runBatch(lastDecision);
    } else {
      batchError = null;
    }
  }

  function onDockKey(e: KeyboardEvent): void {
    const el = e.target as HTMLElement | null;
    if (el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA')) return;
    if (e.key === 'Escape') {
      clearSelection();
      return;
    }
    if (e.shiftKey && (e.key === 'A' || e.key === 'a')) {
      e.preventDefault();
      selectAll();
      return;
    }
    if (e.key === 'a') {
      e.preventDefault();
      void runBatch('approve');
      return;
    }
    if (e.key === 'r') {
      e.preventDefault();
      void runBatch('deny');
    }
  }

  function shortId(id: string): string {
    return id.length > 8 ? id.slice(0, 8) : id;
  }
</script>

<section class="approvals-dock" data-region="approvals-dock" aria-label="Approvals Dock">
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

  {#if pending.length > 1}
    <div class="batch">
      <div class="batch-head">
        <span class="count">{selectedIds.length} seleccionadas</span>
        <button
          type="button"
          class="approve-all"
          disabled={!canBatch}
          onkeydown={onDockKey}
          onclick={() => void runBatch('approve')}
        >
          {batchBusy
            ? `Procesando ${selectedIds.length}…`
            : `Aprobar seleccionadas (${selectedIds.length})`}
        </button>
        <button
          type="button"
          class="deny-all"
          disabled={!canBatch}
          onkeydown={onDockKey}
          onclick={() => void runBatch('deny')}
        >
          {batchBusy
            ? `Procesando ${selectedIds.length}…`
            : `Denegar seleccionadas (${selectedIds.length})`}
        </button>
      </div>
      <label class="reason">
        <span>Motivo (opcional) — se guarda en el audit</span>
        <input bind:value={reason} placeholder="por qué…" />
      </label>
      {#if hasConflict}
        <p class="partial" role="status">
          {cleanCount} de {selectedIds.length} sin conflicto · {conflictingIds.length} en conflicto
          {#each Object.entries(fileConflicts) as [file, ids] (file)}
            <code>{file}</code>
            <span class="ids">({ids.map(shortId).join(', ')})</span>
          {/each}
        </p>
      {/if}
      {#if batchError}
        <p class="batch-error" role="alert">
          {batchError}
          <button type="button" class="retry" onclick={retryBatch}>Reintentar</button>
          {#if conflictIds.length > 0}
            <span class="ids">conflicto: {conflictIds.map(shortId).join(', ')}</span>
          {/if}
        </p>
      {/if}
    </div>
  {/if}

  {#if loading}
    <ul class="rows" aria-hidden="true">
      {#each [0, 1] as n (n)}
        <li class="row"><span class="bar"></span></li>
      {/each}
    </ul>
  {:else if pending.length === 0}
    <p class="empty">Sin aprobaciones pendientes</p>
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
                onkeydown={onDockKey}
                onclick={() => toggleSelect(p.approval_id)}
              >
                <span class="check" aria-hidden="true">{selected[p.approval_id] ? '☑' : '☐'}</span>
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">{p.action}</code>
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
                onkeydown={onDockKey}
                onclick={() => toggleSelect(p.approval_id)}
              >
                <span class="check" aria-hidden="true">{selected[p.approval_id] ? '☑' : '☐'}</span>
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">{p.action}</code>
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
                onkeydown={onDockKey}
                onclick={() => toggleSelect(p.approval_id)}
              >
                <span class="check" aria-hidden="true">{selected[p.approval_id] ? '☑' : '☐'}</span>
                <span class="actor">{shortId(p.agent_id)}</span>
                <code class="action">unclassifiable</code>
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
  .batch {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    border: 1px solid var(--a-surface-2);
    border-radius: 5px;
    padding: 0.4rem 0.5rem;
    background: var(--a-bg);
  }
  .batch-head {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .count {
    font-size: 0.72rem;
    color: var(--a-text-muted);
    margin-right: auto;
  }
  .approve-all,
  .deny-all {
    border: 1px solid transparent;
    border-radius: 5px;
    font-size: 0.74rem;
    padding: 0.22rem 0.55rem;
    cursor: pointer;
  }
  .approve-all {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-ok) 35%, transparent);
  }
  .deny-all {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-err) 35%, transparent);
  }
  .approve-all:disabled,
  .deny-all:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  .reason {
    display: flex;
    flex-direction: column;
    gap: 0.15rem;
    font-size: 0.68rem;
    color: var(--a-text-muted);
  }
  .reason input {
    background: var(--a-surface);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.28rem 0.5rem;
    font-size: 0.76rem;
  }
  .partial {
    margin: 0;
    font-size: 0.72rem;
    color: var(--a-warn);
  }
  .partial code {
    color: var(--a-warn);
    margin-left: 0.3rem;
  }
  .ids {
    color: var(--a-text-faint);
    font-size: 0.68rem;
  }
  .batch-error {
    margin: 0;
    font-size: 0.75rem;
    color: var(--a-err);
  }
  .batch-error .retry {
    margin-left: 0.5rem;
    background: transparent;
    border: 1px solid var(--a-border);
    color: var(--a-primary);
    border-radius: 4px;
    font-size: 0.72rem;
    padding: 0.1rem 0.5rem;
    cursor: pointer;
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
    display: grid;
    grid-template-columns: 1.1rem auto 1fr auto;
    align-items: center;
    gap: 0.4rem;
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
  .row-btn[aria-pressed='true'] {
    border-color: color-mix(in srgb, var(--a-primary) 45%, transparent);
  }
  .check {
    text-align: center;
    color: var(--a-primary);
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
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .row-btn:focus-visible,
  .buttons button:focus-visible,
  .approve-all:focus-visible,
  .deny-all:focus-visible,
  .reason input:focus-visible,
  .batch-error .retry:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  @media (prefers-reduced-motion: reduce) {
    .approvals-dock * {
      transition: none;
    }
  }
</style>

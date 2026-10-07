<!-- RFC 65 §3 (P1) — AuditTimeline: the append-only `audit_log` chain, newest
     first. Reads `GET /hud/audit` (polling). Read-only. Hash-chain
     verification (RFC 24 §10) is not implemented yet; this view does NOT claim
     it. Empty state is explicit because no audit writer has landed yet. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchAudit, type AuditResponse, type AuditEntry } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<AuditResponse | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      data = await fetchAudit(hudUrl, 50);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 10000);
    return () => clearInterval(timer);
  });

  function payloadSummary(entry: AuditEntry): string {
    const out = entry.outputs;
    if (out && typeof out === 'object' && Object.keys(out as object).length > 0) {
      return JSON.stringify(out).slice(0, 100);
    }
    const inp = entry.inputs;
    if (inp && typeof inp === 'object' && Object.keys(inp as object).length > 0) {
      return JSON.stringify(inp).slice(0, 100);
    }
    return '';
  }
</script>

<section class="audit">
  <header>
    <h3>Audit timeline</h3>
    <span class="count">{data?.count ?? 0}</span>
  </header>

  {#if error}
    <p class="error">
      Error: {error}
      <button type="button" class="retry" onclick={() => void refresh()}>Retry</button>
    </p>
  {:else if !data}
    {#if loading}
      <ul class="skeleton" aria-hidden="true">
        {#each [0, 1, 2] as n (n)}
          <li></li>
        {/each}
      </ul>
    {:else}
      <p class="empty">No audit data yet.</p>
    {/if}
  {:else if data.rows.length === 0}
    <p class="empty">
      No audit entries yet. The append-only <code>audit_log</code> chain has no writer in the current
      build; rows appear here once one lands. Hash-chain verification (RFC 24 §10) is not implemented,
      so this view does not claim integrity.
    </p>
  {:else}
    <ol class="timeline">
      {#each data.rows as e (e.seq)}
        <li>
          <span class="seq">#{e.seq}</span>
          <time datetime={e.ts}>{e.ts}</time>
          <span class="actor">{e.actor}</span>
          <span class="action">{e.action}</span>
          {#if payloadSummary(e)}<span class="payload">{payloadSummary(e)}</span>{/if}
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .audit {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .audit header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .audit h3 {
    margin: 0;
    font-size: 1rem;
  }
  .count {
    font-size: 0.78rem;
    color: var(--a-text-faint);
  }
  .timeline {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.76rem;
  }
  .timeline li {
    display: flex;
    gap: 0.6rem;
    align-items: baseline;
    border-bottom: 1px solid var(--a-surface-2);
    padding: 0.2rem 0;
  }
  .seq {
    color: var(--a-text-faint);
    font-family: 'SF Mono', Consolas, monospace;
    min-width: 3.5rem;
  }
  .timeline time {
    color: var(--a-text-muted);
    font-family: 'SF Mono', Consolas, monospace;
  }
  .actor {
    color: var(--a-violet);
  }
  .action {
    color: var(--a-info);
  }
  .payload {
    margin-left: auto;
    color: var(--a-text-faint);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    max-width: 40%;
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.82rem;
    margin: 0;
  }
  .error {
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .error .retry {
    margin-left: 0.5rem;
    background: transparent;
    border: 1px solid var(--a-border);
    color: var(--a-primary);
    border-radius: 4px;
    font-size: 0.72rem;
    padding: 0.1rem 0.5rem;
    cursor: pointer;
  }
  .skeleton {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .skeleton li {
    height: 1.6rem;
    border-radius: 4px;
    background: var(--a-surface-2);
    animation: shimmer 1.4s ease-in-out infinite;
  }
  @keyframes shimmer {
    0%,
    100% {
      opacity: 0.5;
    }
    50% {
      opacity: 1;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .skeleton li {
      animation: none;
    }
  }
</style>

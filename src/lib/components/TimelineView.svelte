<!-- RFC 65 §3 (P2) — TimelineView: a compact chronological strip of the
     `journal_events` stream (newest first) via `GET /hud/journal`. Read-only. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchJournalPage, type JournalObserverEntry } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let entries = $state<JournalObserverEntry[]>([]);
  let total = $state(0);
  let error = $state<string | null>(null);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      const page = await fetchJournalPage(hudUrl, { limit: 50, offset: 0 });
      entries = page.entries;
      total = page.total;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
    const timer = setInterval(() => void refresh(), 10000);
    return () => clearInterval(timer);
  });

  function summary(payload: unknown): string {
    if (payload == null) return '';
    if (typeof payload === 'string') return payload.slice(0, 90);
    return JSON.stringify(payload).slice(0, 90);
  }
</script>

<section class="timeline-view">
  <header>
    <h3>Timeline</h3>
    <span class="count">{entries.length} / {total}</span>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if entries.length === 0}
    <p class="empty">No journal events yet.</p>
  {:else}
    <ol class="rays">
      {#each entries as e (e.id)}
        <li>
          <span class="dot"></span>
          <span class="ray-ts">{e.ts}</span>
          <span class="ray-kind">{e.kind}</span>
          <span class="ray-payload">{summary(e.payload)}</span>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .timeline-view {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .timeline-view header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .timeline-view h3 {
    margin: 0;
    font-size: 1rem;
  }
  .count {
    font-size: 0.78rem;
    color: var(--a-text-faint);
  }
  .rays {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
  }
  .rays li {
    display: grid;
    grid-template-columns: 0.9rem 14rem 12rem 1fr;
    align-items: center;
    gap: 0.6rem;
    padding: 0.2rem 0;
    font-size: 0.74rem;
    border-left: 1px solid var(--a-border);
  }
  .dot {
    width: 7px;
    height: 7px;
    border-radius: 50%;
    background: var(--a-info);
    margin-left: -4px;
  }
  .ray-ts {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-text-faint);
  }
  .ray-kind {
    color: var(--a-info);
  }
  .ray-payload {
    color: var(--a-text-muted);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
</style>

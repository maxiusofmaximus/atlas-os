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
  let loading = $state(true);

  async function refresh(): Promise<void> {
    if (!hudUrl) {
      loading = false;
      return;
    }
    error = null;
    loading = true;
    try {
      const page = await fetchJournalPage(hudUrl, { limit: 50, offset: 0 });
      entries = page.entries;
      total = page.total;
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

  {#if loading && entries.length === 0 && !error}
    <div class="skeleton" aria-busy="true" aria-label="Loading timeline">
      <span class="skel"></span>
      <span class="skel"></span>
      <span class="skel short"></span>
    </div>
  {:else if error}
    <div class="error" role="alert">
      <span>Error: {error}</span>
      <button type="button" onclick={() => void refresh()}>Retry</button>
    </div>
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
  .skeleton {
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
  .skel.short {
    width: 60%;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .error button {
    font-size: 0.75rem;
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
</style>

<!-- RFC 19 §10 — Journal Observer HUD (research 33 SECTOR B 6.1).
     Read-only inspection view over `GET /hud/journal`: newest-first
     table of journal entries with kind/ts columns, an expandable full
     payload per row, a kind filter, and limit/offset pagination. The
     Phase 1 `/tail/*` routes stay as the newest-N projection feed;
     this panel is the complete audit view operators open from any
     mission card (RFC 24 §3). -->

<script lang="ts">
  import type { JournalObserverEntry, JournalPageParams } from '$stores/hud';
  import { fetchJournalPage } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  const DEFAULT_LIMIT = 20;

  let entries = $state<JournalObserverEntry[]>([]);
  let total = $state(0);
  let limit = $state(DEFAULT_LIMIT);
  let offset = $state(0);
  let kindInput = $state('');
  let appliedKind = $state('');
  let loading = $state(false);
  let error = $state<string | null>(null);
  let expanded = $state<Record<number, boolean>>({});
  let autoLoaded = $state(false);

  async function load(): Promise<void> {
    if (!hudUrl) {
      error = 'HUD URL not available';
      return;
    }
    loading = true;
    error = null;
    try {
      const params: JournalPageParams = { limit, offset };
      if (appliedKind) params.kind = appliedKind;
      const page = await fetchJournalPage(hudUrl, params);
      entries = page.entries;
      total = page.total;
    } catch (e) {
      entries = [];
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  }

  $effect(() => {
    if (hudUrl && !autoLoaded) {
      autoLoaded = true;
      void load();
    }
  });

  function rangeText(): string {
    if (total === 0) return '0 entries';
    return `${offset + 1}–${Math.min(offset + limit, total)} of ${total}`;
  }

  function applyFilter(): void {
    appliedKind = kindInput.trim();
    offset = 0;
    expanded = {};
    void load();
  }

  function clearFilter(): void {
    kindInput = '';
    appliedKind = '';
    offset = 0;
    expanded = {};
    void load();
  }

  function prevPage(): void {
    if (offset <= 0) return;
    offset = Math.max(0, offset - limit);
    void load();
  }

  function nextPage(): void {
    if (offset + limit >= total) return;
    offset += limit;
    void load();
  }

  function changeLimit(next: number): void {
    limit = next;
    offset = 0;
    void load();
  }

  function toggle(id: number): void {
    expanded = { ...expanded, [id]: !expanded[id] };
  }
</script>

<section class="journal-observer" aria-label="Journal Observer">
  <header>
    <h3>Journal Observer</h3>
    <span class="count">{rangeText()}</span>
    <button type="button" onclick={() => void load()} disabled={loading || !hudUrl}>
      {loading ? 'Loading…' : 'Reload'}
    </button>
  </header>

  <div class="filter">
    <label>
      <span>Kind</span>
      <input
        bind:value={kindInput}
        placeholder="task_received…"
        onkeydown={(e) => {
          if (e.key === 'Enter') applyFilter();
        }}
      />
    </label>
    <button type="button" onclick={applyFilter} disabled={loading || !hudUrl}>Apply</button>
    <button type="button" onclick={clearFilter} disabled={loading || !hudUrl || !appliedKind}>
      Clear
    </button>
    {#if appliedKind}
      <code class="applied-kind">{appliedKind}</code>
    {/if}
  </div>

  {#if error}
    <p class="error" role="alert">{error}</p>
  {/if}

  {#if entries.length === 0 && !loading && !error}
    <p class="empty">
      No journal entries yet. Run <code>atlas mission new "your prompt"</code> to seed the Journal.
    </p>
  {:else}
    <table class="entries">
      <thead>
        <tr>
          <th>id</th>
          <th>ts</th>
          <th>kind</th>
          <th>payload</th>
        </tr>
      </thead>
      <tbody>
        {#each entries as entry (entry.id)}
          <tr>
            <td class="num">{entry.id}</td>
            <td class="ts">{entry.ts}</td>
            <td><code class="kind">{entry.kind}</code></td>
            <td>
              <button type="button" class="toggle" onclick={() => toggle(entry.id)}>
                {expanded[entry.id] ? 'Collapse' : 'Expand'}
              </button>
              {#if expanded[entry.id]}
                <pre>{JSON.stringify(entry.payload, null, 2)}</pre>
              {/if}
            </td>
          </tr>
        {/each}
      </tbody>
    </table>

    <div class="pager">
      <label class="page-size">
        <span>Per page</span>
        <select
          value={limit}
          onchange={(e) => changeLimit(Number((e.target as HTMLSelectElement).value))}
        >
          <option value="10">10</option>
          <option value="20">20</option>
          <option value="50">50</option>
        </select>
      </label>
      <button type="button" onclick={prevPage} disabled={loading || offset <= 0}>Prev</button>
      <span class="range">{rangeText()}</span>
      <button type="button" onclick={nextPage} disabled={loading || offset + limit >= total}>
        Next
      </button>
    </div>
  {/if}
</section>

<style>
  .journal-observer {
    border: 1px solid var(--journal-border, #30363d);
    border-radius: 6px;
    background: var(--journal-bg, #161b22);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .journal-observer header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .journal-observer h3 {
    margin: 0;
    font-size: 1rem;
  }
  .count,
  .range {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .count {
    flex: 1 1 auto;
  }
  .journal-observer header button,
  .filter button,
  .pager button {
    padding: 0.3rem 0.6rem;
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }
  .journal-observer button:hover:not(:disabled) {
    background: #30363d;
  }
  .journal-observer button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .filter {
    display: flex;
    align-items: flex-end;
    gap: 0.4rem;
  }
  .filter label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: var(--journal-muted, #8b949e);
  }
  .filter input {
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-family: 'Fira Code', 'JetBrains Mono', monospace;
    font-size: 0.8rem;
  }
  .applied-kind {
    font-size: 0.7rem;
    color: #79c0ff;
  }
  .error {
    color: #f85149;
    font-size: 0.75rem;
    margin: 0;
  }
  .empty {
    color: var(--journal-muted, #8b949e);
    font-size: 0.85rem;
    margin: 0;
  }
  .empty code {
    background: #0d1117;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }
  .entries {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.75rem;
  }
  .entries th,
  .entries td {
    text-align: left;
    padding: 0.25rem 0.5rem;
    border-bottom: 1px solid #21262d;
    vertical-align: top;
  }
  .entries td.num {
    text-align: right;
    font-variant-numeric: tabular-nums;
    font-family: 'Fira Code', monospace;
  }
  .ts {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    color: var(--journal-muted, #8b949e);
    white-space: nowrap;
  }
  .kind {
    color: #79c0ff;
  }
  .toggle {
    padding: 0.15rem 0.5rem;
    background: transparent;
    color: #8b949e;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.7rem;
  }
  .entries pre {
    margin: 0.3rem 0 0 0;
    padding: 0.4rem 0.5rem;
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 4px;
    font-size: 0.7rem;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 16rem;
    overflow-y: auto;
  }
  .pager {
    display: flex;
    align-items: center;
    justify-content: flex-end;
    gap: 0.5rem;
  }
  .page-size {
    display: flex;
    align-items: center;
    gap: 0.3rem;
    font-size: 0.7rem;
    color: var(--journal-muted, #8b949e);
    margin-right: auto;
  }
  .page-size select {
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    font-size: 0.75rem;
  }
</style>

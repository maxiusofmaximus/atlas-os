<!-- RFC 65 §3 (P2) — WorktreesView: the git worktrees of a repository
     (`GET /hud/worktrees`, RFC 05 §4 isolation). Read-only and fail-safe: a
     non-repo / missing-git result is surfaced as a reason, never an empty list
     pretending success. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchWorktrees, type WorktreesResponse } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<WorktreesResponse | null>(null);
  let repoInput = $state('');
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      data = await fetchWorktrees(hudUrl, repoInput.trim() || undefined);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
  });
</script>

<section class="worktrees">
  <header>
    <h3>Worktrees</h3>
    <button type="button" onclick={() => void refresh()} disabled={loading || !hudUrl}>
      {loading ? 'Loading…' : 'Reload'}
    </button>
  </header>

  <label class="repo">
    <span>Repo (defaults to cwd)</span>
    <input bind:value={repoInput} placeholder="C:/path/to/repo" />
  </label>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if !data}
    <p class="empty">Loading…</p>
  {:else if !data.ok}
    <p class="empty">
      No worktrees: <span class="reason">{data.reason}</span>
      <br />Repo: <code>{data.repo || '(unknown)'}</code>
    </p>
  {:else if data.entries.length === 0}
    <p class="empty">Repo <code>{data.repo}</code> has no worktrees.</p>
  {:else}
    <table>
      <thead>
        <tr>
          <th>Path</th>
          <th>Branch</th>
          <th>State</th>
        </tr>
      </thead>
      <tbody>
        {#each data.entries as e (e.path)}
          <tr>
            <td class="path">{e.path}</td>
            <td class="branch">{e.branch ?? '—'}</td>
            <td class="state">{e.detached ? 'detached' : 'branch'}</td>
          </tr>
        {/each}
      </tbody>
    </table>
  {/if}
</section>

<style>
  .worktrees {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .worktrees header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .worktrees h3 {
    margin: 0;
    font-size: 1rem;
  }
  button {
    background: transparent;
    border: 1px solid var(--a-border);
    color: var(--a-text-muted);
    border-radius: 4px;
    font-size: 0.74rem;
    padding: 0.15rem 0.6rem;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .repo {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: var(--a-text-muted);
  }
  .repo input {
    background: var(--a-bg);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.76rem;
  }
  table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.74rem;
  }
  th,
  td {
    text-align: left;
    padding: 0.25rem 0.4rem;
    border-bottom: 1px solid var(--a-surface-2);
  }
  th {
    color: var(--a-text-muted);
    font-weight: 500;
    font-size: 0.68rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .path {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-info);
    word-break: break-all;
  }
  .branch {
    color: var(--a-text);
  }
  .state {
    color: var(--a-text-muted);
  }
  .reason {
    color: var(--a-warn);
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

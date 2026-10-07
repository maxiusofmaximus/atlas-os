<!-- RFC 67 §20 H-08 — sandbox frame (F11-c). Bridges the backend snapshot route
     `GET /hud/agent/{run_id}/snapshot` (response `{run_id, sandbox, root,
     file_count, files:[{path, sha256}]}`). A 404 carries a `{reason}` from the
     backend and is rendered as an explanatory empty state — the card never
     shows a generic error for a frame that simply does not exist. Only real
     transport/5xx failures use the error state. -->

<script lang="ts">
  import { onMount } from 'svelte';

  interface SnapshotFile {
    path: string;
    sha256: string;
  }

  interface SnapshotData {
    run_id: string;
    sandbox: string;
    root: string;
    file_count: number;
    files: SnapshotFile[];
  }

  interface Props {
    hudUrl?: string | null;
    runId?: string | null;
  }

  const { hudUrl, runId = null }: Props = $props();

  let data = $state<SnapshotData | null>(null);
  let unavailable = $state<string | null>(null);
  let loading = $state(true);
  let error = $state<string | null>(null);

  function endpoint(): string | null {
    if (!hudUrl || !runId) return null;
    return `${hudUrl.replace(/\/$/, '')}/hud/agent/${encodeURIComponent(runId)}/snapshot`;
  }

  async function load(): Promise<void> {
    const url = endpoint();
    if (!url) {
      loading = false;
      return;
    }
    loading = true;
    error = null;
    unavailable = null;
    try {
      const res = await fetch(url);
      if (res.status === 404) {
        const body = (await res.json().catch(() => null)) as { reason?: string } | null;
        unavailable = body?.reason ?? 'no sandbox frame for this run';
        data = null;
        return;
      }
      if (!res.ok) throw new Error(`HTTP ${res.status}`);
      data = (await res.json()) as SnapshotData;
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void load();
  });
</script>

<section class="sandbox-frame" aria-label="Sandbox frame">
  <header class="frame-head">
    <h4>Sandbox frame</h4>
    {#if data}<span class="count">{data.file_count} files · {data.sandbox}</span>{/if}
  </header>

  {#if loading && !data && !unavailable && !error}
    <div class="loading" aria-busy="true" aria-label="loading sandbox frame">
      <span class="skel"></span>
      <span class="skel short"></span>
    </div>
  {:else if unavailable}
    <div class="unavailable">
      <span class="glyph" aria-hidden="true">◌</span>
      <span class="reason">{unavailable}</span>
      <button type="button" class="retry" onclick={() => void load()}>Retry</button>
    </div>
  {:else if error}
    <div class="error" role="alert">
      <span>could not load sandbox frame</span>
      <button type="button" onclick={() => void load()}>Retry</button>
    </div>
  {:else if data && data.files.length === 0}
    <p class="empty">Sandbox frame is empty.</p>
  {:else if data}
    <ul class="files">
      {#each data.files as file (file.path)}
        <li>
          <span class="path">{file.path}</span>
          <code class="sha">{file.sha256.slice(0, 12)}</code>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .sandbox-frame {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
    border-top: 1px solid var(--a-border);
    padding-top: 0.55rem;
  }
  .frame-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
  }
  .frame-head h4 {
    margin: 0;
    font-size: 0.85rem;
    color: var(--a-text);
  }
  .count {
    font-size: 0.72rem;
    color: var(--a-text-faint);
  }
  .loading {
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .skel {
    display: block;
    height: 0.75rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--a-text-faint) 22%, transparent);
  }
  .skel.short {
    width: 60%;
  }
  .unavailable {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.8rem;
    color: var(--a-text-muted);
    background: var(--a-surface-2);
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
  }
  .unavailable .glyph {
    color: var(--a-text-faint);
  }
  .unavailable .reason {
    flex: 1 1 auto;
  }
  .empty {
    margin: 0;
    font-size: 0.8rem;
    color: var(--a-text-muted);
  }
  .error {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    font-size: 0.8rem;
    color: var(--a-err);
  }
  .retry,
  .error button {
    font-size: 0.75rem;
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
  .retry:focus-visible,
  .error button:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .files {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    max-height: 10rem;
    overflow-y: auto;
  }
  .files li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    font-size: 0.75rem;
    padding: 0.15rem 0.35rem;
    border-radius: 3px;
    background: var(--a-bg);
  }
  .files .path {
    color: var(--a-text);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .files .sha {
    color: var(--a-text-faint);
    font-family: 'SF Mono', Consolas, monospace;
    flex: 0 0 auto;
  }
</style>

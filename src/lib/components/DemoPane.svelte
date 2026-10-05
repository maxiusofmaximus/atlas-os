<!-- RFC 65 §9 (P3) — DemoPane: the demo artefacts produced by agent runs
     (screenshots / files / logs / preview URLs). Reads `GET /hud/demos`.
     Read-only. The Loom-style narrated video capture (RFC 24 §9 — TTS track,
     chapters, draw overlays) is NOT implemented; this shows the persisted
     subset and says so. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchDemos, type DemoArtifact } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let artifacts = $state<DemoArtifact[]>([]);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      const res = await fetchDemos(hudUrl, 50);
      artifacts = res.artifacts;
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

  function short(s: string): string {
    return s.length > 8 ? s.slice(0, 8) : s;
  }
</script>

<section class="demo-pane">
  <header>
    <h3>Demos</h3>
    <button type="button" onclick={() => void refresh()} disabled={loading || !hudUrl}>
      {loading ? 'Loading…' : 'Reload'}
    </button>
  </header>

  <p class="note">
    Narrated video (TTS track, chapters, draw overlays) is not implemented (RFC 24 §9). Below are
    the artefacts the capability layer persists per run (screenshots / files / preview URLs).
  </p>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else if artifacts.length === 0}
    <p class="empty">
      No demo artefacts yet. Run <code>atlas agent "&lt;task&gt;" --verify</code>.
    </p>
  {:else}
    <ul class="cards">
      {#each artifacts as a (a.id)}
        <li class="card" data-kind={a.kind}>
          <div class="row">
            <span class="kind">{a.kind}</span>
            <span class="run">run {short(a.run_id)}</span>
            <span class="verified" data-ok={a.verified ? 'yes' : 'no'}>
              {a.verified ? 'verified' : 'unverified'}
            </span>
          </div>
          {#if a.path}<code class="path">{a.path}</code>{/if}
          {#if a.sha256}<span class="sha">sha256 {short(a.sha256)}</span>{/if}
          {#if a.preview_url}
            <button
              type="button"
              class="preview"
              onclick={() => window.open(a.preview_url ?? '', '_blank', 'noopener,noreferrer')}
            >
              preview ↗
            </button>
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .demo-pane {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .demo-pane header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .demo-pane h3 {
    margin: 0;
    font-size: 1rem;
  }
  .note {
    margin: 0;
    font-size: 0.72rem;
    color: #8b949e;
  }
  button {
    background: transparent;
    border: 1px solid #30363d;
    color: #8b949e;
    border-radius: 4px;
    font-size: 0.74rem;
    padding: 0.15rem 0.6rem;
    cursor: pointer;
  }
  .cards {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.5rem;
  }
  .card {
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 5px;
    padding: 0.5rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.74rem;
  }
  .row {
    display: flex;
    align-items: baseline;
    gap: 0.5rem;
  }
  .kind {
    color: #79c0ff;
    font-weight: 600;
  }
  .run {
    color: #6e7681;
    font-family: 'SF Mono', Consolas, monospace;
  }
  .verified {
    margin-left: auto;
    font-size: 0.66rem;
  }
  .verified[data-ok='yes'] {
    color: #56d364;
  }
  .verified[data-ok='no'] {
    color: #d29922;
  }
  .path {
    color: #c9d1d9;
    word-break: break-all;
  }
  .sha {
    color: #6e7681;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.66rem;
  }
  .preview {
    align-self: flex-start;
    background: transparent;
    border: 0;
    padding: 0;
    color: #58a6ff;
    text-decoration: none;
    cursor: pointer;
    font: inherit;
  }
  .preview:hover {
    text-decoration: underline;
  }
  .empty {
    color: #8b949e;
    font-size: 0.82rem;
    margin: 0;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

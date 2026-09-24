<!-- RFC 28 §H.4 — "Model Ready" notification card.
     Fired by the Scheduler Driver when the Toast queue row
     `kind='model_ready'` reaches its `fire_at_ms` (i.e. the
     model's `resets_at` window has elapsed). The HUD maps the
     Toast payload into this card so the user can resume the
     paused mission with one click. The "Resume" button emits a
     deep-link `opencode://mission/{id}/resume` that the Rust
     core interprets to re-enqueue the paused turn in the
     Orchestrator. -->

<script lang="ts">
  import type { ModelReadyCardPayload } from '$stores/hud';

  interface Props {
    payload: ModelReadyCardPayload;
    hudUrl: string | null;
  }

  const { payload, hudUrl }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  async function onResume(): Promise<void> {
    if (!payload.mission_id) {
      error = 'No mission to resume — the paused turn was cancelled.';
      return;
    }
    busy = true;
    error = null;
    try {
      const { postMissionResume } = await import('$stores/hud');
      await postMissionResume(hudUrl, payload.mission_id);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function fmtReady(resetsAt: string): string {
    try {
      const d = new Date(resetsAt);
      return d
        .toISOString()
        .replace('T', ' ')
        .replace(/\.\d+Z$/, ' UTC');
    } catch {
      return resetsAt;
    }
  }
</script>

<section class="model-ready-card" data-kind="model_ready">
  <header>
    <h3>✓ Model Ready</h3>
  </header>

  <p class="body">
    <code>{payload.model}</code> is available again.
    {#if payload.mission_id}
      <br />
      Resume mission <code>{payload.mission_id.slice(0, 8)}</code>?
    {/if}
  </p>

  <p class="meta">
    Resets at {fmtReady(payload.resets_at)}.
  </p>

  <div class="actions">
    <button type="button" onclick={onResume} disabled={busy || !payload.mission_id}>
      {busy ? 'Resuming…' : 'Resume'}
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</section>

<style>
  .model-ready-card {
    border: 1px solid #56d364;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .model-ready-card header {
    display: flex;
    align-items: center;
  }
  .model-ready-card h3 {
    margin: 0;
    font-size: 1rem;
    color: #56d364;
  }
  .body {
    margin: 0;
    font-size: 0.9rem;
    color: #c9d1d9;
  }
  .body code {
    font-family: 'SF Mono', Consolas, monospace;
    color: #58a6ff;
  }
  .meta {
    margin: 0;
    font-size: 0.75rem;
    color: #6e7681;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
  }
  .actions button {
    padding: 0.4rem 0.9rem;
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.85rem;
  }
  .actions button:hover:not(:disabled) {
    background: #30363d;
  }
  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

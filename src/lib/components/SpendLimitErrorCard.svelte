<!-- RFC 28 §H.3 — Spend-limit / rate-limit error card.
     Mirrors Cline PR #10207: when a model API returns a SpendLimitError
     (status 402/403 spend exhausted or 429 rate-limited), the HUD surfaces
     this card instead of silently retrying. The card is **exempt** from the
     auto-retry loop — no turn requests are spawned while it is visible.
     Two actions:
       - "Request Increase" → deep-link to the provider's usage dashboard.
         5-min localStorage cooldown per provider avoids re-clicking in a
         panic while the dashboard opens in a browser tab.
       - "Switch Provider" → invokes `opencode profile switch {backup}`
         via the Rust core IPC. Disabled when `backup_profile_id` is `None`.
     The HUD receives the card payload from the Rust host through the
     kernel-bus event `spend_limit_observed` (BusEventKind::SpendLimitObserved)
     — see `orchestrator::handle_spend_limit_error`. -->

<script lang="ts">
  import type { SpendLimitErrorCardPayload } from '$stores/hud';

  interface Props {
    payload: SpendLimitErrorCardPayload;
    backupProfileId: string | null;
    hudUrl: string | null;
  }

  let { payload, backupProfileId, hudUrl }: Props = $props();

  let busy = $state(false);
  let error = $state<string | null>(null);

  const cooldownMs = 5 * 60 * 1000;

  function cooldownKey(provider: string): string {
    return `opencode.llm.cooldown.request_increase.${provider}`;
  }

  function cooldownActive(provider: string): boolean {
    try {
      const raw = localStorage.getItem(cooldownKey(provider));
      if (!raw) return false;
      const ts = Number(raw);
      return Number.isFinite(ts) && Date.now() - ts < cooldownMs;
    } catch {
      return false;
    }
  }

  function providerDashboardUrl(provider: string): string {
    const p = provider.toLowerCase();
    if (p === 'anthropic' || p.includes('anthropic')) return 'https://console.anthropic.com/usage';
    if (p === 'openai' || p.includes('openai')) return 'https://platform.openai.com/usage';
    if (p === 'google' || p.includes('google')) return 'https://aistudio.google.com/usage';
    if (p === 'mistral' || p.includes('mistral')) return 'https://console.mistral.ai/usage';
    return 'https://opencode.ai/docs/providers';
  }

  async function onRequestIncrease(): Promise<void> {
    if (cooldownActive(payload.provider)) {
      error = 'Cooldown active — please wait 5 minutes before re-clicking.';
      return;
    }
    try {
      localStorage.setItem(cooldownKey(payload.provider), String(Date.now()));
    } catch {
      // localStorage may be unavailable (private mode, sandbox). Proceed
      // with opening the dashboard; cooldown simply won't persist.
    }
    const url = providerDashboardUrl(payload.provider);
    try {
      window.open(url, '_blank', 'noopener,noreferrer');
    } catch {
      error = 'Could not open browser tab — visit manually: ' + url;
    }
  }

  async function onSwitchProvider(): Promise<void> {
    if (!backupProfileId) {
      error = 'No backup profile configured. Set backup_profile_id in profile.toml.';
      return;
    }
    busy = true;
    error = null;
    try {
      const { postProfileSwitch } = await import('$stores/hud');
      await postProfileSwitch(hudUrl, backupProfileId);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  function fmtResetsAt(resetsAt: string): string {
    try {
      const d = new Date(resetsAt);
      const iso = d.toISOString().replace('T', ' ').replace(/\.\d+Z$/, ' UTC');
      const mins = Math.max(0, Math.round((d.getTime() - Date.now()) / 60000));
      const suffix = mins <= 0 ? ' (now)' : ` (in ${mins} min)`;
      return iso + suffix;
    } catch {
      return resetsAt;
    }
  }

  const titleText = $derived(
    payload.error_type === 'spend_limit' ? 'Spend Limit Reached' : 'Rate Limited',
  );
  const requestCooldown = $derived(cooldownActive(payload.provider));
</script>

<section class="spend-limit-card" data-error-type={payload.error_type}>
  <header>
    <h3>⚠ {titleText}</h3>
  </header>

  <dl class="meta">
    <div>
      <dt>Provider</dt>
      <dd>{payload.provider}</dd>
    </div>
    <div>
      <dt>Model</dt>
      <dd>{payload.model}</dd>
    </div>
    <div>
      <dt>Resets at</dt>
      <dd>{fmtResetsAt(payload.resets_at)}</dd>
    </div>
  </dl>

  <div class="actions">
    <button
      type="button"
      onclick={onRequestIncrease}
      disabled={requestCooldown}
    >
      {requestCooldown ? 'Cooldown…' : 'Request Increase'}
    </button>
    <button
      type="button"
      onclick={onSwitchProvider}
      disabled={busy || !backupProfileId}
    >
      {busy ? 'Switching…' : 'Switch Provider'}
    </button>
  </div>

  {#if error}
    <p class="error">{error}</p>
  {/if}
</section>

<style>
  .spend-limit-card {
    border: 1px solid #f85149;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .spend-limit-card[data-error-type='spend_limit'] {
    border-color: #f0883e;
  }
  .spend-limit-card header {
    display: flex;
    align-items: center;
  }
  .spend-limit-card h3 {
    margin: 0;
    font-size: 1rem;
    color: #f85149;
  }
  .spend-limit-card[data-error-type='spend_limit'] h3 {
    color: #f0883e;
  }
  .meta {
    display: grid;
    grid-template-columns: repeat(3, 1fr);
    gap: 0.4rem;
    margin: 0;
  }
  .meta dt {
    font-size: 0.7rem;
    color: #6e7681;
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  .meta dd {
    margin: 0.1rem 0 0 0;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.85rem;
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

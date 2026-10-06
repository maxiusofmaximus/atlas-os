<!-- RFC 25 §3.10 — SettingsView: the in-app input for provider API keys.
     Writes go straight to the OS keychain (`POST /hud/secrets`); the read side
     returns only a `present` flag, never the value. Mirrors the provider-key
     screens of opencode/Grok/Cursor. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchSecretSlots, postSecret, deleteSecret, type SecretsResponse } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<SecretsResponse | null>(null);
  let account = $state('OPENCODE_GO_KEY');
  let value = $state('');
  let error = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let busy = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    try {
      data = await fetchSecretSlots(hudUrl);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  async function save(): Promise<void> {
    if (!hudUrl) return;
    notice = null;
    error = null;
    const name = account.trim();
    if (!name || !value.trim()) {
      error = 'Name and key are required.';
      return;
    }
    busy = true;
    try {
      await postSecret(hudUrl, name, value);
      notice = `Saved “${name}” to the OS keychain.`;
      value = ''; // never keep the key in the field
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = false;
    }
  }

  async function remove(name: string): Promise<void> {
    if (!hudUrl) return;
    notice = null;
    error = null;
    try {
      await deleteSecret(hudUrl, name);
      notice = `Removed “${name}” from the OS keychain.`;
      await refresh();
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
  });
</script>

<section class="settings">
  <header>
    <h3>Provider API keys</h3>
    <button type="button" onclick={() => void refresh()} disabled={!hudUrl}>Reload</button>
  </header>
  <p class="hint">
    Stored in the OS keychain{data ? ` (service ${data.service})` : ''} — never in files or logs. The
    key is write-only; the list shows only whether a slot is set.
  </p>

  <form
    class="key-form"
    onsubmit={(e) => {
      e.preventDefault();
      void save();
    }}
  >
    <label>
      <span>Name (key slot)</span>
      <input bind:value={account} placeholder="OPENCODE_GO_KEY" />
    </label>
    <label>
      <span>API key</span>
      <input type="password" bind:value placeholder="sk-…" autocomplete="off" />
    </label>
    <button type="submit" disabled={busy || !hudUrl}>{busy ? 'Saving…' : 'Save key'}</button>
  </form>

  {#if error}
    <p class="error">{error}</p>
  {/if}
  {#if notice}
    <p class="notice">{notice}</p>
  {/if}

  {#if !data}
    <p class="empty">Loading…</p>
  {:else}
    <ul class="slots">
      {#each data.slots as slot (slot.account)}
        <li>
          <code>{slot.account}</code>
          <span class:present={slot.present} class="state">
            {slot.present ? 'stored' : 'not set'}
          </span>
          {#if slot.present}
            <button type="button" class="del" onclick={() => void remove(slot.account)}
              >Remove</button
            >
          {/if}
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .settings {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .settings header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .settings h3 {
    margin: 0;
    font-size: 1rem;
  }
  .hint {
    margin: 0;
    color: #8b949e;
    font-size: 0.76rem;
  }
  .key-form {
    display: flex;
    gap: 0.6rem;
    align-items: flex-end;
    flex-wrap: wrap;
  }
  .key-form label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: #8b949e;
  }
  input {
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.76rem;
    min-width: 12rem;
  }
  button {
    background: transparent;
    border: 1px solid #30363d;
    color: #8b949e;
    border-radius: 4px;
    font-size: 0.74rem;
    padding: 0.28rem 0.7rem;
    cursor: pointer;
  }
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  button.del {
    color: #f85149;
    border-color: #f8514955;
    margin-left: auto;
  }
  .slots {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
  }
  .slots li {
    display: flex;
    align-items: center;
    gap: 0.5rem;
    font-size: 0.76rem;
  }
  .slots code {
    color: #79c0ff;
  }
  .state {
    color: #8b949e;
  }
  .state.present {
    color: #3fb950;
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
  .notice {
    color: #3fb950;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

<!-- RFC 65 §10 (P1) — McpView: the MCP catalog with the RFC 07 §2/§3/§4 policy
     the runtime enforces (sandbox, supply chain, tool allowlist), a probe
     (`tools/list`) and an in-place allowlist editor. Read-only except the
     allowlist: a missing/malformed registry is surfaced as a reason, never a
     fabricated list. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchMcp,
    probeMcp,
    saveMcpAllowlist,
    type McpCatalog,
    type McpProbeTool,
    type McpServer,
  } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  let data = $state<McpCatalog | null>(null);
  let repoInput = $state('');
  let drafts = $state<Record<string, string>>({});
  let probed = $state<Record<string, McpProbeTool[]>>({});
  let probing = $state<string | null>(null);
  let saving = $state<string | null>(null);
  let notice = $state<string | null>(null);
  let error = $state<string | null>(null);
  let loading = $state(false);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    loading = true;
    error = null;
    try {
      data = await fetchMcp(hudUrl, repoInput.trim() || undefined);
      if (data.ok) {
        drafts = Object.fromEntries(
          data.servers.map((s) => [s.name, (s.allowed_tools ?? []).join(', ')]),
        );
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void refresh();
  });

  async function probeServer(s: McpServer): Promise<void> {
    if (!hudUrl) return;
    probing = s.name;
    notice = null;
    error = null;
    try {
      const r = await probeMcp(hudUrl, s.name, repoInput.trim() || undefined);
      if (!r.ok) {
        error = r.reason ?? 'probe failed';
      } else {
        probed = { ...probed, [s.name]: r.tools };
        notice = `${s.name}: ${r.tools.length} tool(s) discovered`;
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      probing = null;
    }
  }

  function addTool(server: string, tool: string): void {
    const current = (drafts[server] ?? '')
      .split(',')
      .map((t) => t.trim())
      .filter(Boolean);
    if (!current.includes(tool)) current.push(tool);
    drafts = { ...drafts, [server]: current.join(', ') };
  }

  async function save(s: McpServer): Promise<void> {
    if (!hudUrl) return;
    saving = s.name;
    notice = null;
    error = null;
    try {
      const tools = (drafts[s.name] ?? '')
        .split(',')
        .map((t) => t.trim())
        .filter(Boolean);
      const ack = await saveMcpAllowlist(hudUrl, s.name, tools, repoInput.trim() || undefined);
      if (!ack.ok) {
        error = ack.reason ?? 'save failed';
      } else {
        notice = `${s.name}: saved (${tools.length} tool${tools.length === 1 ? '' : 's'})`;
        await refresh();
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      saving = null;
    }
  }

  function argv(cmd: unknown): string {
    if (Array.isArray(cmd)) return cmd.join(' ');
    return typeof cmd === 'string' ? cmd : '—';
  }

  function supplyClass(s: McpServer): string {
    const v = s.supply?.verdict;
    return v === 'block' ? 'bad' : v === 'warn' ? 'warn' : 'ok';
  }
</script>

<section class="mcp">
  <header>
    <h3>MCP servers</h3>
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
      No MCP registry: <span class="reason">{data.reason}</span>
      <br />Repo: <code>{data.repo || '(unknown)'}</code>
    </p>
  {:else if data.servers.length === 0}
    <p class="empty">No MCP servers declared in <code>{data.path}</code>.</p>
  {:else}
    {#if notice}
      <p class="notice">{notice}</p>
    {/if}
    <ul class="servers">
      {#each data.servers as s (s.name)}
        <li class="server">
          <div class="row">
            <span class="name">{s.name}</span>
            <span class="tag" class:off={!s.enabled}>{s.enabled ? 'enabled' : 'disabled'}</span>
            <span class="tag">{s.transport ?? s.type ?? 'stdio'}</span>
            {#if s.supply}
              <span class="tag {supplyClass(s)}">supply: {s.supply.verdict}</span>
            {/if}
            {#if s.sandbox}
              <span class="tag" class:off={!s.sandbox.enforced}>
                sandbox: {s.sandbox.effective}{s.sandbox.enforced ? '' : ' (unenforced)'}
              </span>
            {/if}
          </div>
          <div class="cmd"><code>{argv(s.command)}</code></div>
          {#if s.sandbox?.finding}
            <div class="finding">! {s.sandbox.finding}</div>
          {/if}
          {#if s.supply && s.supply.reasons.length > 0}
            <div class="finding">supply: {s.supply.reasons.join('; ')}</div>
          {/if}
          <div class="edit">
            <span class="edit-label">allowed tools (RFC 07 §4 — empty = none exposed)</span>
            <div class="edit-row">
              <input
                class="tools-input"
                placeholder="resolve-library-id, query-docs"
                bind:value={drafts[s.name]}
              />
              <button
                type="button"
                onclick={() => void probeServer(s)}
                disabled={probing === s.name || !hudUrl}
              >
                {probing === s.name ? 'Probing…' : 'Probe'}
              </button>
              <button
                type="button"
                onclick={() => void save(s)}
                disabled={saving === s.name || !hudUrl}
              >
                {saving === s.name ? 'Saving…' : 'Save'}
              </button>
            </div>
          </div>
          {#if probed[s.name]?.length}
            <div class="tools">
              {#each probed[s.name] as t (t.name)}
                <button
                  type="button"
                  class="tool add"
                  class:on={t.allowed}
                  onclick={() => addTool(s.name, t.name)}
                  title={t.description ?? ''}
                >
                  + {t.name}
                </button>
              {/each}
            </div>
          {/if}
          <div class="tools">
            {#if s.allowed_tools && s.allowed_tools.length > 0}
              {#each s.allowed_tools as t (t)}<span class="tool">{t}</span>{/each}
            {:else}
              <span class="empty-tools">no tools exposed (empty allowlist — RFC 07 §4)</span>
            {/if}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .mcp {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .mcp header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .mcp h3 {
    margin: 0;
    font-size: 1rem;
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
  button:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }
  .repo {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
    color: #8b949e;
  }
  .repo input,
  .tools-input {
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.3rem 0.5rem;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.76rem;
  }
  .servers {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .server {
    border: 1px solid #21262d;
    border-radius: 4px;
    padding: 0.5rem 0.6rem;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    flex-wrap: wrap;
  }
  .name {
    font-weight: 600;
    color: #c9d1d9;
  }
  .tag {
    font-size: 0.66rem;
    border: 1px solid #30363d;
    border-radius: 999px;
    padding: 0.05rem 0.45rem;
    color: #8b949e;
  }
  .tag.off {
    color: #d29922;
    border-color: #6b4f14;
  }
  .tag.ok {
    color: #3fb950;
    border-color: #1f5a2a;
  }
  .tag.warn {
    color: #d29922;
    border-color: #6b4f14;
  }
  .tag.bad {
    color: #f85149;
    border-color: #6b1f1f;
  }
  .cmd code {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.72rem;
    color: #79c0ff;
    word-break: break-all;
  }
  .finding {
    font-size: 0.72rem;
    color: #d29922;
  }
  .edit {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
  }
  .edit-label {
    font-size: 0.66rem;
    color: #8b949e;
  }
  .edit-row {
    display: flex;
    gap: 0.4rem;
  }
  .edit-row .tools-input {
    flex: 1;
  }
  .tools {
    display: flex;
    flex-wrap: wrap;
    gap: 0.3rem;
  }
  .tool {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.7rem;
    background: #0d1117;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.1rem 0.4rem;
    color: #c9d1d9;
  }
  .tool.add {
    cursor: pointer;
  }
  .tool.add:hover {
    border-color: #58a6ff;
    color: #79c0ff;
  }
  .tool.add.on {
    border-color: #1f5a2a;
    color: #3fb950;
  }
  .empty-tools {
    font-size: 0.72rem;
    color: #8b949e;
  }
  .notice {
    color: #3fb950;
    font-size: 0.78rem;
    margin: 0;
  }
  .reason {
    color: #d29922;
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

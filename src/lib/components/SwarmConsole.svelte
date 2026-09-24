<!-- RFC 31 SECTOR B 4.5 — Swarm Console HUD (munder-difflin office floor).
     Read-only 2D floor: one desk per agent projected from the Kernel Bus
     tail (`swarm_agent_spawned` / `swarm_state_changed`), a mailbox drawer
     per desk fed by `swarm_message` events, and a checks button per worktree
     (CN-004) hitting `GET /swarm/:mission/:agent/checks`. The Swarm pool
     owns the write side; this panel is pure telemetry that operators can
     open from any mission card (RFC 24 §3). -->

<script lang="ts">
  import type { SwarmAgent, SwarmMessage, SwarmCheck } from '$stores/hud';
  import { swarmStateColor, countUnread, fetchSwarmChecks, postSwarmSend } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
    missionId: string | null;
    agents: SwarmAgent[];
    messages: SwarmMessage[];
  }

  const { hudUrl, missionId, agents, messages }: Props = $props();

  let mailboxAgent = $state<string | null>(null);
  let checks = $state<
    Record<string, { loading: boolean; error: string | null; rows: SwarmCheck[] }>
  >({});
  let draftFrom = $state('');
  let draftBody = $state('');
  let sending = $state(false);
  let sendError = $state<string | null>(null);
  let sendOk = $state<string | null>(null);

  function inboxFor(agentId: string): SwarmMessage[] {
    return messages.filter((m) => m.to_agent === agentId);
  }

  function unreadFor(agentId: string): number {
    return countUnread(inboxFor(agentId));
  }

  function checksFor(
    agentId: string,
  ): { loading: boolean; error: string | null; rows: SwarmCheck[] } | undefined {
    return checks[agentId];
  }

  function mailboxInbox(): SwarmMessage[] {
    return mailboxAgent ? inboxFor(mailboxAgent) : [];
  }

  function mailboxTitle(): string {
    return mailboxAgent ? short(mailboxAgent) : '';
  }

  function short(id: string): string {
    return id.slice(0, 8);
  }

  function worktreeShort(path: string | null): string {
    if (!path) return 'no worktree';
    const parts = path.replace(/\\/g, '/').split('/');
    return parts.slice(-2).join('/');
  }

  function openMailbox(agentId: string): void {
    mailboxAgent = agentId;
    draftFrom = '';
    draftBody = '';
    sendError = null;
    sendOk = null;
  }

  function closeMailbox(): void {
    mailboxAgent = null;
    draftFrom = '';
    draftBody = '';
    sending = false;
    sendError = null;
    sendOk = null;
  }

  async function loadChecks(agent: SwarmAgent): Promise<void> {
    if (!hudUrl || !missionId) {
      checks = {
        ...checks,
        [agent.agent_id]: { loading: false, error: 'no HUD URL or mission yet', rows: [] },
      };
      return;
    }
    checks = { ...checks, [agent.agent_id]: { loading: true, error: null, rows: [] } };
    try {
      const rows = await fetchSwarmChecks(hudUrl, missionId, agent.agent_id);
      checks = { ...checks, [agent.agent_id]: { loading: false, error: null, rows } };
    } catch (err) {
      checks = {
        ...checks,
        [agent.agent_id]: {
          loading: false,
          error: err instanceof Error ? err.message : String(err),
          rows: [],
        },
      };
    }
  }

  async function sendMessage(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const to = mailboxAgent;
    if (!to) return;
    const from = draftFrom.trim();
    const body = draftBody.trim();
    if (!from || !body) {
      sendError = 'from and body are required';
      return;
    }
    if (!hudUrl) {
      sendError = 'no HUD URL yet';
      return;
    }
    sending = true;
    sendError = null;
    sendOk = null;
    try {
      const res = await postSwarmSend(hudUrl, { from_agent: from, to_agent: to, body });
      sendOk = `sent ${res.id.slice(0, 8)}`;
      draftBody = '';
    } catch (err) {
      sendError = err instanceof Error ? err.message : String(err);
    } finally {
      sending = false;
    }
  }
</script>

<section class="swarm-console" aria-label="Swarm Console">
  <header>
    <h3>Swarm Console</h3>
    {#if missionId}
      <code class="mission-id">{missionId}</code>
    {/if}
    <span class="count" aria-label="{agents.length} agents">{agents.length} desks</span>
  </header>

  {#if agents.length === 0}
    <p class="empty">
      No agents spawned yet. Try <code
        >atlas swarm start --preset atlas-team --mission &lt;id&gt;</code
      >.
    </p>
  {:else}
    <div class="floor">
      {#each agents as agent (agent.agent_id)}
        <article
          class="desk"
          data-state={swarmStateColor(agent.state)}
          data-role={agent.role}
          aria-label="desk {agent.role} {short(agent.agent_id)}"
        >
          <div class="desk-head">
            <span class="role">{agent.role}</span>
            {#if unreadFor(agent.agent_id) > 0}
              <span class="badge" aria-label="{unreadFor(agent.agent_id)} unread messages"
                >{unreadFor(agent.agent_id)}</span
              >
            {/if}
          </div>
          <code class="agent-id" title={agent.agent_id}>{short(agent.agent_id)}</code>
          <span class="pill" data-phase={swarmStateColor(agent.state)}>{agent.state}</span>
          <p class="meta" title={agent.model_id}>{agent.model_id}</p>
          <p class="meta" title={agent.worktree_path ?? ''}>
            ⌁ {worktreeShort(agent.worktree_path)}
          </p>
          <div class="actions">
            <button type="button" onclick={() => openMailbox(agent.agent_id)}>Mailbox</button>
            <button
              type="button"
              onclick={() => void loadChecks(agent)}
              disabled={checksFor(agent.agent_id)?.loading === true || !hudUrl || !missionId}
            >
              {checksFor(agent.agent_id)?.loading === true ? 'Checking…' : 'Checks'}
            </button>
          </div>
          {#if checksFor(agent.agent_id)?.error}
            <p class="error">{checksFor(agent.agent_id)?.error}</p>
          {:else if (checksFor(agent.agent_id)?.rows.length ?? 0) > 0}
            <ul class="checks">
              {#each checksFor(agent.agent_id)?.rows ?? [] as check (check.name)}
                <li data-status={check.status}>
                  <span class="dot" aria-hidden="true"></span>
                  <span class="check-name">{check.name}</span>
                  {#if check.detail}
                    <span class="check-detail">{check.detail}</span>
                  {/if}
                </li>
              {/each}
            </ul>
          {:else if checksFor(agent.agent_id)?.loading !== true && checksFor(agent.agent_id)}
            <p class="hint">No checks reported.</p>
          {/if}
        </article>
      {/each}
    </div>
  {/if}
</section>

{#if mailboxAgent}
  <aside class="drawer" aria-label="Agent mailbox">
    <header>
      <h3>Mailbox <code>{mailboxTitle()}</code></h3>
      <span class="count"
        >{mailboxInbox().length} msg{mailboxInbox().length === 1 ? '' : 's'}{countUnread(
          mailboxInbox(),
        ) > 0
          ? ` · ${countUnread(mailboxInbox())} unread`
          : ''}</span
      >
      <button type="button" class="drawer-close" onclick={closeMailbox} aria-label="Close">×</button
      >
    </header>

    {#if mailboxInbox().length === 0}
      <p class="empty">No messages for this desk yet.</p>
    {:else}
      <ul class="msgs">
        {#each mailboxInbox() as msg (msg.id)}
          <li data-read={msg.read_at != null ? 'read' : 'unread'}>
            <div class="msg-head">
              <span class="msg-from">{msg.from_agent}</span>
              <time>{msg.created_at}</time>
            </div>
            <p class="msg-body">{msg.body}</p>
          </li>
        {/each}
      </ul>
    {/if}

    <form class="drawer-form" onsubmit={(e) => void sendMessage(e as SubmitEvent)}>
      <h4>Send message</h4>
      <label>
        <span>From (agent id)</span>
        <input bind:value={draftFrom} placeholder="planner agent id…" required />
      </label>
      <label>
        <span>Body</span>
        <textarea bind:value={draftBody} placeholder="Work order for this desk…" rows="3" required
        ></textarea>
      </label>
      {#if sendError}
        <p class="error">{sendError}</p>
      {/if}
      {#if sendOk}
        <p class="success">{sendOk}</p>
      {/if}
      <div class="form-actions">
        <button type="submit" disabled={sending || !hudUrl}>
          {sending ? 'Sending…' : 'Send'}
        </button>
      </div>
    </form>
  </aside>
{/if}

<style>
  .swarm-console {
    border: 1px solid var(--swarm-border, #30363d);
    border-radius: 6px;
    background: var(--swarm-bg, #161b22);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .swarm-console header {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .swarm-console h3 {
    margin: 0;
    font-size: 1rem;
  }
  .mission-id {
    font-size: 0.75rem;
    color: var(--swarm-muted, #8b949e);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    flex: 1 1 auto;
  }
  .count {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .empty {
    color: var(--swarm-muted, #8b949e);
    font-size: 0.85rem;
    margin: 0;
  }
  .empty code {
    background: #0d1117;
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }
  .floor {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 0.6rem;
  }
  .desk {
    border: 1px solid var(--swarm-border, #30363d);
    border-left: 3px solid var(--swarm-muted, #6e7681);
    border-radius: 6px;
    background: #0d1117;
    padding: 0.55rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.8rem;
  }
  .desk[data-state='blue'] {
    border-left-color: #58a6ff;
  }
  .desk[data-state='amber'] {
    border-left-color: #d29922;
  }
  .desk[data-state='green'] {
    border-left-color: #3fb950;
  }
  .desk[data-state='red'] {
    border-left-color: #f85149;
  }
  .desk-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .role {
    text-transform: uppercase;
    letter-spacing: 0.05em;
    font-size: 0.72rem;
    font-weight: 700;
    color: #79c0ff;
  }
  .badge {
    min-width: 1.2rem;
    text-align: center;
    font-size: 0.68rem;
    font-weight: 700;
    color: #0d1117;
    background: #d29922;
    border-radius: 999px;
    padding: 0 0.3rem;
  }
  .agent-id {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    color: var(--swarm-muted, #8b949e);
  }
  .pill {
    align-self: flex-start;
    font-size: 0.68rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: #21262d;
    color: #8b949e;
  }
  .pill[data-phase='blue'] {
    color: #58a6ff;
    background: rgba(88, 166, 255, 0.12);
  }
  .pill[data-phase='amber'] {
    color: #d29922;
    background: rgba(210, 153, 34, 0.12);
  }
  .pill[data-phase='green'] {
    color: #56d364;
    background: rgba(86, 211, 100, 0.12);
  }
  .pill[data-phase='red'] {
    color: #f85149;
    background: rgba(248, 81, 73, 0.12);
  }
  .meta {
    margin: 0;
    font-size: 0.7rem;
    color: var(--swarm-muted, #8b949e);
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .actions {
    display: flex;
    gap: 0.4rem;
    margin-top: 0.15rem;
  }
  .actions button {
    flex: 1;
    padding: 0.3rem 0.5rem;
    background: #21262d;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
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
    font-size: 0.75rem;
    margin: 0;
  }
  .hint {
    color: var(--swarm-muted, #8b949e);
    font-size: 0.72rem;
    margin: 0;
    font-style: italic;
  }
  .success {
    color: #56d364;
    font-size: 0.75rem;
    margin: 0;
  }
  .checks {
    list-style: none;
    padding: 0;
    margin: 0.15rem 0 0 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.72rem;
  }
  .checks li {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
  }
  .checks .dot {
    width: 0.5rem;
    height: 0.5rem;
    border-radius: 999px;
    background: #6e7681;
    flex: none;
    align-self: center;
  }
  .checks li[data-status='pass'] .dot {
    background: #3fb950;
  }
  .checks li[data-status='fail'] .dot {
    background: #f85149;
  }
  .checks li[data-status='pending'] .dot {
    background: #d29922;
  }
  .check-name {
    font-family: 'Fira Code', monospace;
  }
  .check-detail {
    color: var(--swarm-muted, #8b949e);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .drawer {
    position: fixed;
    top: 0;
    right: 0;
    width: 380px;
    max-width: 90vw;
    height: 100vh;
    background: #0d1117;
    border-left: 1px solid #30363d;
    box-shadow: -8px 0 24px rgba(0, 0, 0, 0.5);
    padding: 1rem 1.25rem;
    overflow-y: auto;
    z-index: 50;
  }
  .drawer header {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    margin-bottom: 1rem;
  }
  .drawer h3 {
    font-size: 0.95rem;
    margin: 0;
    color: #58a6ff;
    flex: 1 1 auto;
  }
  .drawer-close {
    background: transparent;
    color: #c9d1d9;
    border: 0;
    font-size: 1.3rem;
    cursor: pointer;
    line-height: 1;
  }
  .msgs {
    list-style: none;
    padding: 0;
    margin: 0 0 1.25rem 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .msgs li {
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    background: #161b22;
  }
  .msgs li[data-read='unread'] {
    border-left: 3px solid #d29922;
  }
  .msg-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.25rem;
    font-size: 0.7rem;
    font-family: 'Fira Code', monospace;
  }
  .msg-from {
    color: #58a6ff;
  }
  .msg-head time {
    opacity: 0.5;
  }
  .msg-body {
    margin: 0;
    font-size: 0.8rem;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .drawer-form {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .drawer-form h4 {
    font-size: 0.8rem;
    color: #79c0ff;
    margin: 0;
  }
  .drawer-form label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
  }
  .drawer-form label span {
    color: #8b949e;
  }
  .drawer-form input,
  .drawer-form textarea {
    background: #161b22;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0.35rem 0.5rem;
    font-family: 'Fira Code', 'JetBrains Mono', monospace;
    font-size: 0.8rem;
    resize: vertical;
  }
  .drawer-form .form-actions {
    display: flex;
    justify-content: flex-end;
  }
  .drawer-form button {
    background: #238636;
    color: #fff;
    border: 1px solid #2ea043;
    border-radius: 4px;
    padding: 0.4rem 0.9rem;
    cursor: pointer;
    font-size: 0.8rem;
  }
  .drawer-form button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
</style>

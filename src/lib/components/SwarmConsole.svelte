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
    border: 1px solid var(--swarm-border, var(--a-border));
    border-radius: 6px;
    background: var(--swarm-bg, var(--a-surface));
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
    color: var(--swarm-muted, var(--a-text-muted));
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
    color: var(--swarm-muted, var(--a-text-muted));
    font-size: 0.85rem;
    margin: 0;
  }
  .empty code {
    background: var(--a-bg);
    padding: 0.1rem 0.3rem;
    border-radius: 3px;
  }
  .floor {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(190px, 1fr));
    gap: 0.6rem;
  }
  .desk {
    border: 1px solid var(--swarm-border, var(--a-border));
    border-left: 3px solid var(--swarm-muted, var(--a-text-faint));
    border-radius: 6px;
    background: var(--a-bg);
    padding: 0.55rem 0.65rem;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
    font-size: 0.8rem;
  }
  .desk[data-state='blue'] {
    border-left-color: var(--a-info);
  }
  .desk[data-state='amber'] {
    border-left-color: var(--a-warn);
  }
  .desk[data-state='green'] {
    border-left-color: var(--a-ok);
  }
  .desk[data-state='red'] {
    border-left-color: var(--a-err);
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
    color: var(--a-info);
  }
  .badge {
    min-width: 1.2rem;
    text-align: center;
    font-size: 0.68rem;
    font-weight: 700;
    color: var(--a-bg);
    background: var(--a-warn);
    border-radius: 999px;
    padding: 0 0.3rem;
  }
  .agent-id {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    color: var(--swarm-muted, var(--a-text-muted));
  }
  .pill {
    align-self: flex-start;
    font-size: 0.68rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    background: var(--a-surface-2);
    color: var(--a-text-muted);
  }
  .pill[data-phase='blue'] {
    color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 12%, transparent);
  }
  .pill[data-phase='amber'] {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
  }
  .pill[data-phase='green'] {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
  }
  .pill[data-phase='red'] {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
  }
  .meta {
    margin: 0;
    font-size: 0.7rem;
    color: var(--swarm-muted, var(--a-text-muted));
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
    background: var(--a-surface-2);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    cursor: pointer;
    font-size: 0.75rem;
  }
  .actions button:hover:not(:disabled) {
    background: var(--a-border);
  }
  .actions button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }
  .error {
    color: var(--a-err);
    font-size: 0.75rem;
    margin: 0;
  }
  .hint {
    color: var(--swarm-muted, var(--a-text-muted));
    font-size: 0.72rem;
    margin: 0;
    font-style: italic;
  }
  .success {
    color: var(--a-ok);
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
    background: var(--a-text-faint);
    flex: none;
    align-self: center;
  }
  .checks li[data-status='pass'] .dot {
    background: var(--a-ok);
  }
  .checks li[data-status='fail'] .dot {
    background: var(--a-err);
  }
  .checks li[data-status='pending'] .dot {
    background: var(--a-warn);
  }
  .check-name {
    font-family: 'Fira Code', monospace;
  }
  .check-detail {
    color: var(--swarm-muted, var(--a-text-muted));
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
    background: var(--a-bg);
    border-left: 1px solid var(--a-border);
    box-shadow: -8px 0 24px var(--a-shadow);
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
    color: var(--a-info);
    flex: 1 1 auto;
  }
  .drawer-close {
    background: transparent;
    color: var(--a-text);
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
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    background: var(--a-surface);
  }
  .msgs li[data-read='unread'] {
    border-left: 3px solid var(--a-warn);
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
    color: var(--a-info);
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
    color: var(--a-info);
    margin: 0;
  }
  .drawer-form label {
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.7rem;
  }
  .drawer-form label span {
    color: var(--a-text-muted);
  }
  .drawer-form input,
  .drawer-form textarea {
    background: var(--a-surface);
    color: var(--a-text);
    border: 1px solid var(--a-border);
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
    background: var(--a-ok);
    color: var(--a-bg);
    border: 1px solid var(--a-ok);
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

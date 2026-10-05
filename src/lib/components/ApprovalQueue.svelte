<!-- RFC 65 §3/§4 (P0) — ApprovalQueue: the operator's answer surface for
     `Confirm`-class actions (RFC 18 §2). Pending set is folded from the WS tail;
     Approve/Deny POST to `/hud/approvals/:id/{approve,deny}` and the decision
     fans out on the bus so every device converges. -->

<script lang="ts">
  import { hud, projectPendingApprovals, approveApproval, denyApproval } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  const pending = $derived(projectPendingApprovals($hud.events));
  let busy = $state<string | null>(null);
  let error = $state<string | null>(null);

  async function answer(id: string, decision: 'approve' | 'deny', ev: MouseEvent): Promise<void> {
    ev.stopPropagation();
    busy = id;
    error = null;
    try {
      if (decision === 'approve') {
        await approveApproval(hudUrl, id);
      } else {
        await denyApproval(hudUrl, id);
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      busy = null;
    }
  }

  function shortId(id: string): string {
    return id.length > 8 ? id.slice(0, 8) : id;
  }
</script>

<section class="approval-queue">
  <header>
    <h3>Approvals</h3>
    <span class="badge" class:pending={pending.length > 0}>{pending.length}</span>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {/if}

  {#if pending.length === 0}
    <p class="empty">No pending approvals.</p>
  {:else}
    <ul class="items">
      {#each pending as p (p.approval_id)}
        <li class="item">
          <div class="meta">
            <code class="action">{p.action}</code>
            <span class="ids">{shortId(p.agent_id)} · {shortId(p.approval_id)}</span>
          </div>
          <div class="actions">
            <button
              class="approve"
              disabled={busy === p.approval_id}
              onclick={(e) => answer(p.approval_id, 'approve', e)}
            >
              Approve
            </button>
            <button
              class="deny"
              disabled={busy === p.approval_id}
              onclick={(e) => answer(p.approval_id, 'deny', e)}
            >
              Deny
            </button>
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</section>

<style>
  .approval-queue {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  h3 {
    margin: 0;
    font-size: 1rem;
  }
  .badge {
    font-size: 0.75rem;
    padding: 0.05rem 0.5rem;
    border-radius: 999px;
    background: #21262d;
    color: #6e7681;
  }
  .badge.pending {
    color: #d29922;
    background: rgba(210, 153, 34, 0.15);
  }
  .empty {
    color: #8b949e;
    font-size: 0.85rem;
    margin: 0;
  }
  .items {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
    border: 1px solid #21262d;
    border-radius: 5px;
    padding: 0.4rem 0.6rem;
  }
  .meta {
    display: flex;
    flex-direction: column;
    gap: 0.1rem;
  }
  .action {
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.82rem;
    color: #f0883e;
  }
  .ids {
    font-size: 0.68rem;
    color: #6e7681;
  }
  .actions {
    display: flex;
    gap: 0.35rem;
  }
  button {
    border: 1px solid transparent;
    border-radius: 5px;
    font-size: 0.78rem;
    padding: 0.25rem 0.6rem;
    cursor: pointer;
  }
  .approve {
    color: #56d364;
    background: rgba(86, 211, 100, 0.12);
    border-color: rgba(86, 211, 100, 0.35);
  }
  .deny {
    color: #f85149;
    background: rgba(248, 81, 73, 0.12);
    border-color: rgba(248, 81, 73, 0.35);
  }
  button:disabled {
    opacity: 0.5;
    cursor: wait;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

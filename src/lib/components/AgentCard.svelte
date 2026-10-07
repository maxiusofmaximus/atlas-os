<!-- RFC 63 §9 + RFC 65 §3 — AgentCard: the live capability-layer view.
     Consumes `BusEventKind::AgentStep` (tag `agent_step`) from the Kernel Bus
     tail and renders the most recent run's step timeline: action, evidence
     (observation), verdict, and running token totals. Pure live view; the
     data is the same forensic trail persisted to `agent_steps`. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import {
    hud,
    agentRunId,
    projectAgentSteps,
    agentRunTotals,
    agentStepColor,
    fetchTail,
    type AgentStepPayload,
  } from '$stores/hud';

  interface Props {
    hudUrl?: string | null;
  }

  const { hudUrl }: Props = $props();

  // Prefer the live WS stream; fall back to polling the REST tail so the card
  // still shows the last run when the agent ran in a separate process (CLI).
  let tailSteps = $state<AgentStepPayload[]>([]);

  const liveRunId = $derived(agentRunId($hud.events));
  const runId = $derived(liveRunId ?? tailRunId(tailSteps));
  const steps = $derived(
    liveRunId ? projectAgentSteps($hud.events, liveRunId) : tailStepsForRun(tailSteps, runId),
  );
  const totals = $derived(agentRunTotals(steps));
  const done = $derived(steps.some((s) => s.action === 'done' && s.verdict !== 'fail'));

  function tailRunId(rows: AgentStepPayload[]): string | null {
    return rows.length > 0 ? (rows[0]?.run_id ?? null) : null;
  }

  function tailStepsForRun(rows: AgentStepPayload[], id: string | null): AgentStepPayload[] {
    if (!id) return [];
    return rows
      .filter((r) => r.run_id === id)
      .slice()
      .reverse();
  }

  async function refreshTail(): Promise<void> {
    if (!hudUrl) return;
    try {
      tailSteps = await fetchTail<AgentStepPayload>(hudUrl, 'agent_steps', 100);
    } catch {
      /* live WS may still cover it; silence */
    }
  }

  onMount(() => {
    void refreshTail();
    const timer = setInterval(() => void refreshTail(), 5000);
    return () => clearInterval(timer);
  });

  function shortId(id: string): string {
    return id.length > 12 ? `${id.slice(0, 12)}…` : id;
  }
</script>

<section class="agent-card">
  <header>
    <h3>Agent</h3>
    {#if runId}
      <span class="tag" class:done>{done ? 'done' : 'running'}</span>
    {:else}
      <span class="tag idle">idle</span>
    {/if}
  </header>

  {#if !runId}
    <p class="empty">
      No agent run yet. Try <code>atlas agent "&lt;task&gt;" --verify</code>.
    </p>
  {:else}
    <div class="meta">
      <code class="run">{shortId(runId)}</code>
      <span class="totals">{totals.tokens_in}↑ {totals.tokens_out}↓ tok</span>
      {#if totals.cost_usd > 0}
        <span class="totals">${totals.cost_usd.toFixed(4)}</span>
      {/if}
    </div>

    <ol class="steps">
      {#each steps as s (s.step)}
        <li class:last={s.step === steps.length - 1}>
          <span class="dot {agentStepColor(s.action, s.verdict)}"></span>
          <div class="body">
            <div class="line">
              <span class="action">{s.action}</span>
              {#if s.verdict}
                <span class="verdict {s.verdict}">{s.verdict}</span>
              {/if}
              <span class="tokens">{s.tokens_in}↑ {s.tokens_out}↓</span>
            </div>
            {#if s.observation}
              <pre class="obs">{s.observation.slice(0, 240)}</pre>
            {/if}
          </div>
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .agent-card {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .agent-card header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .agent-card h3 {
    margin: 0;
    font-size: 1rem;
  }
  .tag {
    font-size: 0.78rem;
    padding: 0.1rem 0.5rem;
    border-radius: 999px;
    color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 12%, transparent);
  }
  .tag.done {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
  }
  .tag.idle {
    color: var(--a-text-faint);
    background: var(--a-surface-2);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.85rem;
    margin: 0;
  }
  .meta {
    display: flex;
    gap: 0.6rem;
    align-items: center;
    font-size: 0.78rem;
  }
  .meta .run {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-text-muted);
  }
  .meta .totals {
    color: var(--a-text-faint);
  }
  .steps {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.35rem;
    max-height: 22rem;
    overflow-y: auto;
  }
  .steps li {
    display: flex;
    gap: 0.5rem;
    align-items: flex-start;
  }
  .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    margin-top: 0.35rem;
    flex: 0 0 auto;
    background: var(--a-text-faint);
  }
  .dot.green {
    background: var(--a-ok);
  }
  .dot.blue {
    background: var(--a-info);
  }
  .dot.amber {
    background: var(--a-warn);
  }
  .dot.red {
    background: var(--a-err);
  }
  .body {
    flex: 1 1 auto;
    min-width: 0;
  }
  .line {
    display: flex;
    gap: 0.5rem;
    align-items: baseline;
    font-size: 0.82rem;
  }
  .action {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-text);
  }
  .verdict {
    font-size: 0.7rem;
    padding: 0.02rem 0.4rem;
    border-radius: 4px;
    text-transform: uppercase;
  }
  .verdict.pass {
    color: var(--a-ok);
    background: color-mix(in srgb, var(--a-ok) 12%, transparent);
  }
  .verdict.fail {
    color: var(--a-err);
    background: color-mix(in srgb, var(--a-err) 12%, transparent);
  }
  .verdict.unknown {
    color: var(--a-warn);
    background: color-mix(in srgb, var(--a-warn) 12%, transparent);
  }
  .tokens {
    margin-left: auto;
    font-size: 0.72rem;
    color: var(--a-text-faint);
  }
  .obs {
    margin: 0.15rem 0 0 0;
    padding: 0.3rem 0.5rem;
    background: var(--a-bg);
    border: 1px solid var(--a-surface-2);
    border-radius: 4px;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.72rem;
    color: var(--a-text-muted);
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 6rem;
    overflow-y: auto;
  }
</style>

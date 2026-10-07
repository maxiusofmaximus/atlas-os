<!-- RFC 65 §3 (P2) — OutlineView: a plan's roadmap milestones, in order, with
     their dependencies. Reads the plan list from `/tail/plans` and the plan body
     from `/payload/plan/:id`. Read-only. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import { fetchTail, fetchPayload, type PlanPayload, type PlanMilestone } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  interface PlanRow {
    plan_id: string;
    mission_id: string;
    strategy?: string;
    risk?: number;
  }

  let plans = $state<PlanRow[]>([]);
  let selected = $state<string>('');
  let milestones = $state<PlanMilestone[]>([]);
  let loading = $state(false);
  let error = $state<string | null>(null);

  async function loadPlans(): Promise<void> {
    if (!hudUrl) {
      loading = false;
      return;
    }
    error = null;
    loading = true;
    try {
      plans = await fetchTail<PlanRow>(hudUrl, 'plans', 50);
      if (!selected && plans[0]) {
        selected = plans[0].plan_id;
        await loadPlan();
      }
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  async function loadPlan(): Promise<void> {
    if (!hudUrl || !selected) return;
    loading = true;
    error = null;
    milestones = [];
    try {
      const raw = await fetchPayload(hudUrl, 'plan', selected);
      const plan = (typeof raw === 'string' ? JSON.parse(raw) : raw) as PlanPayload;
      milestones = plan.roadmap ?? [];
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    } finally {
      loading = false;
    }
  }

  onMount(() => {
    void loadPlans();
  });
</script>

<section class="outline">
  <header>
    <h3>Outline — plan roadmap</h3>
    <select
      bind:value={selected}
      onchange={() => void loadPlan()}
      disabled={plans.length === 0}
      aria-label="Plan"
    >
      {#if plans.length === 0}
        <option value="">no plans</option>
      {/if}
      {#each plans as p (p.plan_id)}
        <option value={p.plan_id}>{p.plan_id.slice(0, 8)} · {p.strategy ?? 'plan'}</option>
      {/each}
    </select>
  </header>

  {#if error}
    <div class="error" role="alert">
      <span>Error: {error}</span>
      <button type="button" onclick={() => void loadPlans()}>Retry</button>
    </div>
  {:else if loading}
    <div class="skeleton" aria-busy="true" aria-label="Loading plan">
      <span class="skel"></span>
      <span class="skel short"></span>
    </div>
  {:else if !selected}
    <p class="empty">No plan selected. Plans appear once a mission is planned.</p>
  {:else if milestones.length === 0}
    <p class="empty">No milestones in this plan.</p>
  {:else}
    <ol class="milestones">
      {#each milestones as m, i (m.id)}
        <li>
          <span class="idx">{i + 1}</span>
          <span class="ms-label">{m.label}</span>
          {#if m.depends_on && m.depends_on.length > 0}
            <span class="deps">after {m.depends_on.join(', ')}</span>
          {/if}
        </li>
      {/each}
    </ol>
  {/if}
</section>

<style>
  .outline {
    border: 1px solid var(--a-surface-2);
    border-radius: 6px;
    background: var(--a-surface);
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .outline header {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.6rem;
  }
  .outline h3 {
    margin: 0;
    font-size: 1rem;
  }
  select {
    background: var(--a-bg);
    color: var(--a-text);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.25rem 0.4rem;
    font-size: 0.78rem;
    max-width: 55%;
  }
  .milestones {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.3rem;
  }
  .milestones li {
    display: flex;
    align-items: baseline;
    gap: 0.6rem;
    background: var(--a-bg);
    border: 1px solid var(--a-surface-2);
    border-radius: 5px;
    padding: 0.35rem 0.5rem;
    font-size: 0.82rem;
  }
  .idx {
    font-family: 'SF Mono', Consolas, monospace;
    color: var(--a-text-faint);
    min-width: 1.2rem;
  }
  .ms-label {
    color: var(--a-text);
  }
  .deps {
    margin-left: auto;
    font-size: 0.68rem;
    color: var(--a-warn);
  }
  .empty {
    color: var(--a-text-muted);
    font-size: 0.82rem;
    margin: 0;
  }
  .skeleton {
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .skel {
    display: block;
    height: 0.8rem;
    border-radius: 4px;
    background: color-mix(in srgb, var(--a-text-faint) 22%, transparent);
  }
  .skel.short {
    width: 60%;
  }
  .error {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    color: var(--a-err);
    font-size: 0.8rem;
    margin: 0;
  }
  .error button {
    font-size: 0.75rem;
    padding: 0.15rem 0.55rem;
    border-radius: 4px;
    border: 1px solid var(--a-border-ui);
    background: var(--a-surface-2);
    color: var(--a-text);
    cursor: pointer;
  }
</style>

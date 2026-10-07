<!-- RFC 65 §5 — ViewSwitcher: the ModeBar's view tab strip. Reads the single
     source of truth (`VIEWS`) and sets `activeView`. Unimplemented (future-fase)
     views are rendered disabled with a fase hint so the operator sees the roadmap
     without landing on an empty panel. -->

<script lang="ts">
  import { VIEWS, activeView, type ViewId } from '$stores/views';

  interface Props {
    /** Views that actually render today; others render disabled. */
    available: ViewId[];
  }

  const { available }: Props = $props();
</script>

<nav class="view-switcher" aria-label="HUD views">
  {#each VIEWS as v (v.id)}
    {@const enabled = available.includes(v.id)}
    <button
      class="tab"
      class:active={$activeView === v.id}
      disabled={!enabled}
      title={enabled ? v.label : `${v.label} — ${v.fase} (not yet)`}
      onclick={() => enabled && activeView.set(v.id)}
    >
      {v.label}
      {#if !enabled}<span class="fase">{v.fase}</span>{/if}
    </button>
  {/each}
</nav>

<style>
  .view-switcher {
    display: flex;
    gap: 0.25rem;
    align-items: center;
    border-bottom: 1px solid var(--a-surface-2);
    padding: 0 0 0.4rem 0;
    flex-wrap: wrap;
  }
  .tab {
    background: transparent;
    border: 1px solid transparent;
    color: var(--a-text-muted);
    font-size: 0.85rem;
    padding: 0.25rem 0.75rem;
    border-radius: 6px;
    cursor: pointer;
  }
  .tab:hover:not(:disabled) {
    color: var(--a-text);
    background: var(--a-surface-2);
  }
  .tab.active {
    color: var(--a-info);
    background: color-mix(in srgb, var(--a-info) 12%, transparent);
    border-color: color-mix(in srgb, var(--a-info) 35%, transparent);
  }
  .tab:disabled {
    color: var(--a-text-faint);
    cursor: not-allowed;
  }
  .fase {
    font-size: 0.62rem;
    margin-left: 0.3rem;
    padding: 0 0.25rem;
    border-radius: 3px;
    background: var(--a-surface-2);
  }
</style>

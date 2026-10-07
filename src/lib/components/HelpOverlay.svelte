<script lang="ts">
  interface Props {
    open: boolean;
    onclose: () => void;
  }

  const { open, onclose }: Props = $props();

  const HOTKEY_ROWS: ReadonlyArray<{ keys: string; action: string }> = [
    { keys: ':m', action: 'Enfocar Mission Rail' },
    { keys: ':v', action: 'Cambiar view (Kanban/Canvas/Outline/Timeline/…)' },
    { keys: ':a', action: 'Foco al Approvals Dock' },
    { keys: ':n', action: 'Nueva misión' },
    { keys: ':f', action: 'Fork selected' },
    { keys: ':s', action: 'Steer selected' },
    { keys: ':d', action: 'Show Demo' },
    { keys: ':r', action: 'Run selected' },
    { keys: ':p', action: 'Pause selected' },
    { keys: ':x', action: 'Stop selected' },
    { keys: ':c', action: 'Comment on selected' },
    { keys: ':e', action: 'Expand selected to canvas' },
    { keys: ':o', action: 'Mode selector (ask/architect/code/context)' },
    { keys: ':t', action: 'Toggle Health KPIs dock' },
    { keys: ':?', action: 'Este panel de ayuda' },
  ];

  let panelEl = $state<HTMLDivElement | undefined>(undefined);
  let restoreFocus: HTMLElement | null = null;

  $effect(() => {
    if (open) {
      restoreFocus = (document.activeElement as HTMLElement | null) ?? null;
      queueMicrotask(() => panelEl?.querySelector<HTMLButtonElement>('.close')?.focus());
    } else if (restoreFocus) {
      restoreFocus.focus();
      restoreFocus = null;
    }
  });

  function focusables(): HTMLElement[] {
    if (!panelEl) return [];
    return Array.from(
      panelEl.querySelectorAll<HTMLElement>(
        'button, [href], input, [tabindex]:not([tabindex="-1"])',
      ),
    );
  }

  function onKeydown(e: KeyboardEvent): void {
    if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
      return;
    }
    if (e.key !== 'Tab') return;
    const list = focusables();
    if (list.length === 0) return;
    const first = list[0];
    const last = list[list.length - 1];
    if (!first || !last) return;
    if (e.shiftKey && document.activeElement === first) {
      e.preventDefault();
      last.focus();
    } else if (!e.shiftKey && document.activeElement === last) {
      e.preventDefault();
      first.focus();
    }
  }
</script>

{#if open}
  <div
    class="help-scrim"
    role="button"
    tabindex="-1"
    aria-label="Cerrar ayuda"
    onclick={onclose}
    onkeydown={(e) => e.key === 'Escape' && onclose()}
  ></div>
  <div
    class="help-panel"
    role="dialog"
    aria-modal="true"
    aria-labelledby="help-title"
    tabindex="-1"
    bind:this={panelEl}
    onkeydown={onKeydown}
  >
    <header class="help-head">
      <h2 id="help-title">Atajos</h2>
      <button class="close" type="button" aria-label="Cerrar (Esc)" onclick={onclose}>✕</button>
    </header>
    <table class="help-table">
      <thead>
        <tr><th>Tecla</th><th>Acción</th></tr>
      </thead>
      <tbody>
        {#each HOTKEY_ROWS as row (row.keys)}
          <tr><td><kbd>{row.keys}</kbd></td><td>{row.action}</td></tr>
        {/each}
      </tbody>
    </table>
  </div>
{/if}

<style>
  .help-scrim {
    position: fixed;
    inset: 0;
    background: var(--a-overlay);
    z-index: 90;
  }
  .help-panel {
    position: fixed;
    top: 12vh;
    left: 50%;
    transform: translateX(-50%);
    width: min(560px, 92vw);
    max-height: 76vh;
    overflow-y: auto;
    background: var(--a-surface);
    border: 1px solid var(--a-border);
    border-radius: 8px;
    box-shadow: 0 12px 40px var(--a-shadow);
    z-index: 91;
  }
  .help-panel:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .help-head {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0.6rem 1rem;
    border-bottom: 1px solid var(--a-border);
  }
  .help-head h2 {
    margin: 0;
    font-size: 1rem;
    color: var(--a-text);
  }
  .close {
    background: transparent;
    border: 1px solid var(--a-border-ui);
    border-radius: 4px;
    color: var(--a-text);
    cursor: pointer;
    font-size: 0.85rem;
    padding: 0.1rem 0.5rem;
  }
  .close:focus-visible {
    outline: 2px solid var(--a-focus);
    outline-offset: 2px;
  }
  .help-table {
    width: 100%;
    border-collapse: collapse;
    font-size: 0.82rem;
  }
  .help-table th,
  .help-table td {
    text-align: left;
    padding: 0.35rem 1rem;
    border-bottom: 1px solid var(--a-border);
  }
  .help-table th {
    color: var(--a-text-muted);
    font-weight: 500;
    font-size: 0.7rem;
    text-transform: uppercase;
    letter-spacing: 0.04em;
  }
  .help-table td {
    color: var(--a-text);
  }
  .help-table kbd {
    font-family: var(--a-mono);
    color: var(--a-text-faint);
    border: 1px solid var(--a-border);
    border-radius: 4px;
    padding: 0.02rem 0.35rem;
  }
  @media (prefers-reduced-motion: reduce) {
    .help-panel {
      transition: none;
    }
  }
</style>

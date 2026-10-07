<!-- RFC 65 §5 — CommandPalette: fuzzy command launcher bound to `:`. Lists the
     view switches plus a few operator actions, and dispatches on Enter. Kept
     dependency-free (Svelte only) per §6. -->

<script lang="ts">
  import { VIEWS, activeView, type ViewId } from '$stores/views';

  interface Props {
    open: boolean;
    available: ViewId[];
    onclose: () => void;
    onaction?: (action: string) => void;
  }

  const { open, available, onclose, onaction }: Props = $props();

  let query = $state('');
  let index = $state(0);

  interface Cmd {
    label: string;
    hint: string;
    run: () => void;
  }

  const commands = $derived<Cmd[]>(
    [
      ...VIEWS.map((v) => ({
        label: `Go to ${v.label}`,
        hint: available.includes(v.id) ? `:${v.key}` : `${v.fase} (not yet)`,
        enabled: available.includes(v.id),
        run: () => activeView.set(v.id),
      })),
      {
        label: 'Refresh HUD data',
        hint: ':r',
        enabled: true,
        run: () => onaction?.('refresh'),
      },
      {
        label: 'Export audit posting',
        hint: ':e',
        enabled: true,
        run: () => onaction?.('export'),
      },
    ]
      .filter((c) => c.enabled !== false)
      .filter((c) => c.label.toLowerCase().includes(query.trim().toLowerCase())),
  );

  $effect(() => {
    if (open) {
      query = '';
      index = 0;
    }
  });

  function dispatch(): void {
    const cmd = commands[index];
    if (cmd) {
      cmd.run();
      onclose();
    }
  }

  function onkey(e: KeyboardEvent): void {
    if (e.key === 'ArrowDown') {
      e.preventDefault();
      index = Math.min(index + 1, commands.length - 1);
    } else if (e.key === 'ArrowUp') {
      e.preventDefault();
      index = Math.max(index - 1, 0);
    } else if (e.key === 'Enter') {
      e.preventDefault();
      dispatch();
    } else if (e.key === 'Escape') {
      e.preventDefault();
      onclose();
    }
  }
</script>

{#if open}
  <div
    class="overlay"
    role="button"
    tabindex="-1"
    onclick={onclose}
    onkeydown={(e) => e.key === 'Escape' && onclose()}
  >
    <div
      class="palette"
      role="dialog"
      aria-label="Command palette"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={onkey}
    >
      <input class="search" placeholder="Type a command…" bind:value={query} onkeydown={onkey} />
      <ul class="commands">
        {#each commands as cmd, i (cmd.label)}
          <li>
            <button
              class="cmd"
              class:selected={i === index}
              onclick={() => {
                cmd.run();
                onclose();
              }}
            >
              <span class="label">{cmd.label}</span>
              <span class="hint">{cmd.hint}</span>
            </button>
          </li>
        {/each}
        {#if commands.length === 0}
          <li class="empty">No matching command.</li>
        {/if}
      </ul>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed;
    inset: 0;
    background: var(--a-overlay);
    display: flex;
    justify-content: center;
    align-items: flex-start;
    padding-top: 12vh;
    z-index: 100;
  }
  .palette {
    width: min(560px, 92vw);
    background: var(--a-surface);
    border: 1px solid var(--a-border);
    border-radius: 8px;
    box-shadow: 0 12px 40px var(--a-shadow);
    overflow: hidden;
  }
  .search {
    width: 100%;
    box-sizing: border-box;
    background: var(--a-bg);
    border: none;
    border-bottom: 1px solid var(--a-border);
    color: var(--a-text);
    font-size: 1rem;
    padding: 0.75rem 1rem;
    outline: none;
  }
  .commands {
    list-style: none;
    margin: 0;
    padding: 0.25rem;
    max-height: 46vh;
    overflow-y: auto;
  }
  .cmd {
    width: 100%;
    display: flex;
    justify-content: space-between;
    align-items: center;
    background: transparent;
    border: none;
    color: var(--a-text);
    font-size: 0.9rem;
    padding: 0.5rem 0.75rem;
    border-radius: 5px;
    cursor: pointer;
    text-align: left;
  }
  .cmd.selected {
    background: color-mix(in srgb, var(--a-info) 15%, transparent);
  }
  .hint {
    color: var(--a-text-faint);
    font-size: 0.78rem;
    font-family: 'SF Mono', Consolas, monospace;
  }
  .empty {
    color: var(--a-text-faint);
    font-size: 0.85rem;
    padding: 0.75rem 1rem;
  }
</style>

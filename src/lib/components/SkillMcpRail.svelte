<!-- RFC 65 §10 (P3) — SkillMcpRail: the skills catalog + MCP server registry,
     with a drag-to-stage affordance. Skills come from `/tail/skills`, MCP from
     `GET /hud/mcp` (`.opencode/mcp.json`). The hot-swap activation transaction
     (RFC 24 §8 validate → snapshot → activate) is NOT wired: there is no
     `skill.activated` writer yet, so a drop only *stages* a selection here and
     says so — it does not mutate any agent. -->

<script lang="ts">
  import { onMount } from 'svelte';
  import {
    fetchTail,
    fetchMcp,
    postActivateSkill,
    type SkillCatalogRow,
    type McpCatalog,
    type McpServer,
  } from '$stores/hud';

  interface Props {
    hudUrl: string | null;
  }

  const { hudUrl }: Props = $props();

  type Staged = { kind: 'skill' | 'mcp'; name: string } | null;

  let skills = $state<SkillCatalogRow[]>([]);
  let mcp = $state<McpCatalog | null>(null);
  let error = $state<string | null>(null);
  let staged = $state<Staged>(null);
  let dragOver = $state(false);
  let activation = $state<string | null>(null);

  async function refresh(): Promise<void> {
    if (!hudUrl) return;
    error = null;
    try {
      skills = await fetchTail<SkillCatalogRow>(hudUrl, 'skills', 50);
      mcp = await fetchMcp(hudUrl);
    } catch (err) {
      error = err instanceof Error ? err.message : String(err);
    }
  }

  onMount(() => {
    void refresh();
  });

  function startDrag(e: DragEvent, kind: 'skill' | 'mcp', name: string): void {
    e.dataTransfer?.setData('text/plain', `${kind}:${name}`);
    if (e.dataTransfer) e.dataTransfer.effectAllowed = 'copy';
  }

  async function onDrop(e: DragEvent): Promise<void> {
    e.preventDefault();
    dragOver = false;
    const raw = e.dataTransfer?.getData('text/plain') ?? '';
    const idx = raw.indexOf(':');
    if (idx <= 0) return;
    const kind = raw.slice(0, idx);
    const name = raw.slice(idx + 1);
    if ((kind !== 'skill' && kind !== 'mcp') || !name) return;
    staged = { kind, name };
    activation = null;
    if (kind === 'skill' && hudUrl) {
      try {
        const ack = await postActivateSkill(hudUrl, name);
        activation = `Activated ${ack.skill_id} (scope ${ack.agent_id}).`;
      } catch (err) {
        activation = `Activation failed: ${err instanceof Error ? err.message : String(err)}`;
      }
    }
  }

  function mcpServers(): McpServer[] {
    return mcp?.servers ?? [];
  }
</script>

<section class="rail">
  <header>
    <h3>Skill &amp; MCP rail</h3>
    <button type="button" onclick={() => void refresh()} disabled={!hudUrl}>Refresh</button>
  </header>

  {#if error}
    <p class="error">Error: {error}</p>
  {:else}
    <div class="columns">
      <div class="col">
        <h4>Skills catalog <span class="count">{skills.length}</span></h4>
        {#if skills.length === 0}
          <p class="empty">No skill manifests.</p>
        {:else}
          <ul>
            {#each skills as s (s.skill_id + '@' + s.version)}
              <li
                draggable="true"
                ondragstart={(e) => startDrag(e, 'skill', s.skill_id)}
                title="drag to stage"
              >
                <span class="name">{s.skill_id}</span>
                <span class="ver">v{s.version}</span>
                {#if s.verified}<span class="badge ok">✓</span>{/if}
                {#if s.requires_sandbox}<span class="badge sandbox">sandbox</span>{/if}
              </li>
            {/each}
          </ul>
        {/if}
      </div>

      <div class="col">
        <h4>MCP servers <span class="count">{mcpServers().length}</span></h4>
        {#if !mcp || !mcp.ok}
          <p class="empty">No MCP catalog: {mcp?.reason ?? 'unavailable'}</p>
        {:else if mcpServers().length === 0}
          <p class="empty">No servers declared in <code>.opencode/mcp.json</code>.</p>
        {:else}
          <ul>
            {#each mcpServers() as s (s.name)}
              <li
                draggable="true"
                ondragstart={(e) => startDrag(e, 'mcp', s.name)}
                title="drag to stage"
              >
                <span class="name">{s.name}</span>
                <span class="ver">{s.type ?? 'mcp'}</span>
                <span class="badge" data-on={s.enabled ? 'yes' : 'no'}>
                  {s.enabled ? 'on' : 'off'}
                </span>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    </div>

    <div
      class="dropzone"
      class:over={dragOver}
      class:staged={!!staged}
      role="region"
      aria-label="Stage a skill or MCP server"
      ondragover={(e) => {
        e.preventDefault();
        dragOver = true;
      }}
      ondragleave={() => (dragOver = false)}
      ondrop={(e) => void onDrop(e)}
    >
      {#if staged}
        Staged <strong>{staged.kind}</strong> · <code>{staged.name}</code>
        <button type="button" class="clear" onclick={() => (staged = null)}>clear</button>
      {:else}
        ＋ drag a skill or MCP server here to stage it
      {/if}
      {#if activation}
        <p class="activate-msg" role="status">{activation}</p>
      {/if}
    </div>

    <p class="note">
      Skill drops activate via <code>POST /hud/skills/:id/activate</code> (publishes
      <code>SkillActivated</code> on the Kernel Bus). MCP drops stage a selection only.
    </p>
  {/if}
</section>

<style>
  .rail {
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
    padding: 0.75rem 1rem;
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .rail header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }
  .rail h3 {
    margin: 0;
    font-size: 1rem;
  }
  .rail h4 {
    margin: 0 0 0.3rem 0;
    font-size: 0.78rem;
    color: #79c0ff;
    text-transform: uppercase;
    letter-spacing: 0.04em;
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
  .columns {
    display: grid;
    grid-template-columns: 1fr 1fr;
    gap: 0.6rem;
  }
  .col {
    background: #0d1117;
    border: 1px solid #21262d;
    border-radius: 5px;
    padding: 0.5rem 0.6rem;
  }
  .count {
    color: #6e7681;
    font-size: 0.7rem;
  }
  ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 0.2rem;
    font-size: 0.74rem;
  }
  li {
    display: flex;
    align-items: baseline;
    gap: 0.4rem;
    cursor: grab;
    padding: 0.1rem 0;
  }
  li:active {
    cursor: grabbing;
  }
  .name {
    color: #c9d1d9;
  }
  .ver {
    color: #6e7681;
    font-family: 'SF Mono', Consolas, monospace;
    font-size: 0.68rem;
  }
  .badge {
    margin-left: auto;
    font-size: 0.64rem;
    padding: 0 0.3rem;
    border-radius: 3px;
    background: #21262d;
    color: #8b949e;
  }
  .badge.ok {
    color: #56d364;
  }
  .badge.sandbox {
    color: #d29922;
  }
  .badge[data-on='yes'] {
    color: #56d364;
  }
  .dropzone {
    border: 1px dashed #30363d;
    border-radius: 5px;
    padding: 0.5rem;
    text-align: center;
    font-size: 0.76rem;
    color: #8b949e;
    transition:
      border-color 0.15s,
      background 0.15s;
  }
  .dropzone.over {
    border-color: #58a6ff;
    background: rgba(88, 166, 255, 0.08);
  }
  .dropzone.staged {
    border-style: solid;
    border-color: #56d364;
    color: #c9d1d9;
  }
  .clear {
    margin-left: 0.4rem;
    font-size: 0.66rem;
  }
  .activate-msg {
    margin: 0.4rem 0 0 0;
    font-size: 0.7rem;
    color: #56d364;
  }
  .note {
    margin: 0;
    font-size: 0.68rem;
    color: #6e7681;
  }
  .empty {
    color: #8b949e;
    font-size: 0.74rem;
    margin: 0;
  }
  .error {
    color: #f85149;
    font-size: 0.8rem;
    margin: 0;
  }
</style>

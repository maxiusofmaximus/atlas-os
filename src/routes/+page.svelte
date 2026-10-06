<script lang="ts">
  // Atlas OS — HUD Mission Control landing page (Phase 1).
  // See RFC 24 for the full design. Phase 1 renders:
  //  - HUD URL + WS status (where the axum WS server is bound)
  //  - Live journal stream (kernel bus events)
  //  - Nine tail boxes polling the axum tail routes (RFC 24 §2)
  //    every 5s. Each box shows the latest 20 rows; clicking a row
  //    opens the JSON payload in a side drawer (Phase 2).
  import { onMount } from 'svelte';
  import {
    hud,
    fetchTail,
    fetchAnnotations,
    postAnnotation,
    postExportPosting,
    phaseColor,
    projectSwarmAgents,
    type TailKind,
    type StepPhaseTag,
    type DiffAnnotation,
    type ExportPostingResponse,
    type SwarmMessagePayload,
    fetchRemoteStatus,
    type RemoteAccessStatus,
  } from '$stores/hud';
  import AutoresearchCard from '$lib/components/AutoresearchCard.svelte';
  import AgentCard from '$lib/components/AgentCard.svelte';
  import ApprovalQueue from '$lib/components/ApprovalQueue.svelte';
  import KanbanBoard from '$lib/components/KanbanBoard.svelte';
  import CommandPalette from '$lib/components/CommandPalette.svelte';
  import CostDashboard from '$lib/components/CostDashboard.svelte';
  import HealthKPIs from '$lib/components/HealthKPIs.svelte';
  import AuditTimeline from '$lib/components/AuditTimeline.svelte';
  import CanvasView from '$lib/components/CanvasView.svelte';
  import OutlineView from '$lib/components/OutlineView.svelte';
  import TimelineView from '$lib/components/TimelineView.svelte';
  import WorktreesView from '$lib/components/WorktreesView.svelte';
  import SettingsView from '$lib/components/SettingsView.svelte';
  import McpView from '$lib/components/McpView.svelte';
  import DemoPane from '$lib/components/DemoPane.svelte';
  import SkillMcpRail from '$lib/components/SkillMcpRail.svelte';
  import ViewSwitcher from '$lib/components/ViewSwitcher.svelte';
  import { activeView, type ViewId } from '$stores/views';
  import AvailabilityCard from '$lib/components/AvailabilityCard.svelte';
  import EvalCard from '$lib/components/EvalCard.svelte';
  import SwarmConsole from '$lib/components/SwarmConsole.svelte';
  import JournalObserver from '$lib/components/JournalObserver.svelte';
  import type { PageData } from './$types';

  const { data } = $props<{ data: PageData }>();

  type TailBox = {
    kind: TailKind;
    title: string;
    rows: Array<Record<string, unknown>>;
    loading: boolean;
    error: string | null;
  };

  // Order mirrors the RFC 24 §3 left-to-right reading order of the
  // Mission Control deck: prompt → planning → coding → validation →
  // repair → learning → supervisor → skills. RFC 27 §B/§G add the
  // two orchestration boxes (model swaps + step pills) at the tail.
  const tailKinds: Array<{ kind: TailKind; title: string }> = [
    { kind: 'verdicts', title: 'Prompt verdicts' },
    { kind: 'consolidated', title: 'Consolidated missions' },
    { kind: 'plans', title: 'Plans' },
    { kind: 'diffs', title: 'Code diffs' },
    { kind: 'validation_reports', title: 'Validation reports' },
    { kind: 'repairs', title: 'Repair runs' },
    { kind: 'patterns', title: 'Learned patterns' },
    { kind: 'checkpoints', title: 'Mission checkpoints' },
    { kind: 'skills', title: 'Skill manifests' },
    { kind: 'model_swaps', title: 'Model swaps' },
    { kind: 'step_states', title: 'Step pills' },
  ];

  let tails = $state<Record<string, TailBox>>(
    Object.fromEntries(
      tailKinds.map((t) => [
        t.kind,
        { kind: t.kind, title: t.title, rows: [], loading: false, error: null } satisfies TailBox,
      ]),
    ),
  );

  let pollTimer: ReturnType<typeof setInterval> | null = null;

  // ────────────── RFC 31 §B 4.5 — Swarm Console derived state ──────────────
  // Desks + mailbox feed project straight off the WS tail; the mission id
  // follows the first spawned agent (single-mission floor, Phase 4 scope).
  const swarmAgents = $derived(projectSwarmAgents($hud.events));
  const swarmMissionId = $derived(swarmAgents[0]?.mission_id ?? null);
  const swarmMessages = $derived(
    $hud.events.flatMap((evt) => {
      if (evt.kind !== 'swarm_message') return [];
      const msg = (evt.payload as SwarmMessagePayload | null)?.message;
      return msg ? [msg] : [];
    }),
  );

  // ────────────── RFC 27 §E — annotation drawer state ──────────────
  type DrawerState = {
    open: boolean;
    diffId: string | null;
    diffLabel: string;
    annotations: DiffAnnotation[];
    loading: boolean;
    error: string | null;
    submitting: boolean;
    submitError: string | null;
    draftBody: string;
    draftAuthor: string;
    draftFilePath: string;
    draftLineNo: string;
  };

  let drawer = $state<DrawerState>({
    open: false,
    diffId: null,
    diffLabel: '',
    annotations: [],
    loading: false,
    error: null,
    submitting: false,
    submitError: null,
    draftBody: '',
    draftAuthor: '',
    draftFilePath: '',
    draftLineNo: '',
  });

  // ────────────── RFC 28 §D — audit export state ──────────────
  type ExportState = {
    busy: boolean;
    last: number;
    outputDir: string;
    result: ExportPostingResponse | null;
    error: string | null;
  };

  let exportState = $state<ExportState>({
    busy: false,
    last: 50,
    outputDir: '',
    result: null,
    error: null,
  });

  async function runExportPosting(): Promise<void> {
    const hudUrl = data.hudUrl;
    if (!hudUrl) {
      exportState = { ...exportState, error: 'no HUD URL yet' };
      return;
    }
    exportState = { ...exportState, busy: true, error: null, result: null };
    try {
      const req: { last?: number; output_dir?: string } = { last: exportState.last };
      if (exportState.outputDir.trim() !== '') {
        req.output_dir = exportState.outputDir.trim();
      }
      const result = await postExportPosting(hudUrl, req);
      exportState = { ...exportState, busy: false, result, error: null };
    } catch (err) {
      exportState = {
        ...exportState,
        busy: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  async function openAnnotationDrawer(row: Record<string, unknown>): Promise<void> {
    const diffId = row.id != null ? String(row.id) : null;
    const hudUrl = data.hudUrl;
    if (!diffId || !hudUrl) return;
    drawer = {
      ...drawer,
      open: true,
      diffId,
      diffLabel: rowId(row).slice(0, 8),
      loading: true,
      error: null,
      annotations: [],
      submitting: false,
      submitError: null,
    };
    try {
      const ann = await fetchAnnotations(hudUrl, diffId);
      drawer = { ...drawer, annotations: ann, loading: false };
    } catch (err) {
      drawer = {
        ...drawer,
        loading: false,
        error: err instanceof Error ? err.message : String(err),
      };
    }
  }

  function closeDrawer(): void {
    drawer = {
      ...drawer,
      open: false,
      diffId: null,
      diffLabel: '',
      annotations: [],
      loading: false,
      error: null,
      submitting: false,
      submitError: null,
      draftBody: '',
      draftAuthor: '',
      draftFilePath: '',
      draftLineNo: '',
    };
  }

  async function submitAnnotation(event: SubmitEvent): Promise<void> {
    event.preventDefault();
    const diffId = drawer.diffId;
    const hudUrl = data.hudUrl;
    if (!diffId || !hudUrl) return;
    const body = drawer.draftBody.trim();
    const author = drawer.draftAuthor.trim();
    if (!body || !author) {
      drawer = { ...drawer, submitError: 'body and author are required' };
      return;
    }
    const input: Parameters<typeof postAnnotation>[2] = { body, author };
    const fp = drawer.draftFilePath.trim();
    if (fp) input.file_path = fp;
    const ln = Number(drawer.draftLineNo);
    if (drawer.draftLineNo.trim() !== '' && Number.isFinite(ln) && ln > 0) {
      input.line_no = ln;
    }
    drawer = { ...drawer, submitting: true, submitError: null };
    try {
      await postAnnotation(hudUrl, diffId, input);
      const ann = await fetchAnnotations(hudUrl, diffId);
      drawer = {
        ...drawer,
        annotations: ann,
        submitting: false,
        draftBody: '',
        draftFilePath: '',
        draftLineNo: '',
      };
    } catch (err) {
      drawer = {
        ...drawer,
        submitting: false,
        submitError: err instanceof Error ? err.message : String(err),
      };
    }
  }

  async function refreshOne(box: TailBox): Promise<TailBox> {
    if (!data.hudUrl) return { ...box, error: 'no HUD URL yet' };
    try {
      const rows = await fetchTail<Record<string, unknown>>(data.hudUrl, box.kind, 20);
      return { ...box, rows, loading: false, error: null };
    } catch (err) {
      return { ...box, loading: false, error: err instanceof Error ? err.message : String(err) };
    }
  }

  async function refreshAll() {
    const entries = await Promise.all(
      Object.values(tails).map(async (box) => {
        const next = await refreshOne({ ...box, loading: true });
        return [box.kind, next] as const;
      }),
    );
    tails = Object.fromEntries(entries) as Record<string, TailBox>;
  }

  // ─── RFC 65 §11 / RFC 24 §16 — remote access status (OIDC/bearer) ───
  let remote: RemoteAccessStatus | null = $state(null);

  onMount(() => {
    if (data.hudUrl) {
      hud.connect(data.hudUrl);
      void fetchRemoteStatus(data.hudUrl)
        .then((r) => (remote = r))
        .catch(() => (remote = null));
    }
    void refreshAll();
    pollTimer = setInterval(() => void refreshAll(), 5000);
    return () => {
      hud.disconnect();
      if (pollTimer) clearInterval(pollTimer);
    };
  });

  function rowId(row: Record<string, unknown>): string {
    return String(
      row.id ??
        row.mission_id ??
        row.learn_id ??
        row.skill_id ??
        row.event_id ??
        Object.keys(row).join('|'),
    );
  }

  function rowSummary(row: Record<string, unknown>): string {
    const pick = row.label ?? row.prompt ?? row.summary ?? row.phase ?? row.kind ?? row.title ?? '';
    return String(pick).slice(0, 80);
  }

  /// RFC 27 §G — color lookup for a step pill. `row.phase` is typed as
  /// `unknown` (it came through the generic tail), so funnel through a
  /// narrowing guard instead of a `as StepPhaseTag` cast that Svelte's
  /// template parser rejects.
  function pillColor(row: Record<string, unknown>): string {
    return phaseColor(typeof row.phase === 'string' ? (row.phase as StepPhaseTag) : 'pending');
  }

  // ─── RFC 65 §5 — views, command palette and hotkeys ───
  // Views available today (P0). Later fases append to this list.
  const availableViews: ViewId[] = [
    'overview',
    'agent',
    'kanban',
    'approvals',
    'cost',
    'health',
    'audit',
    'canvas',
    'outline',
    'timeline',
    'worktrees',
    'settings',
    'mcp',
  ];
  let paletteOpen = $state(false);

  function onGlobalKey(e: KeyboardEvent): void {
    const target = e.target as HTMLElement | null;
    const typing =
      target &&
      (target.tagName === 'INPUT' || target.tagName === 'TEXTAREA' || target.isContentEditable);
    if (typing) return;
    if (e.key === ':') {
      e.preventDefault();
      paletteOpen = true;
    } else if (e.key === 'Escape') {
      paletteOpen = false;
    }
  }

  $effect(() => {
    window.addEventListener('keydown', onGlobalKey);
    return () => window.removeEventListener('keydown', onGlobalKey);
  });
</script>

<main>
  <header>
    <h1>Atlas OS</h1>
    <span class="version">v{import.meta.env.VITE_OC_VERSION ?? '0.1.0'}</span>
    {#if remote}
      <span
        class="remote"
        data-state={remote.local_only ? 'local' : 'remote'}
        title={remote.oidc_issuer ?? 'no OIDC issuer configured'}
      >
        {remote.local_only ? 'local-only' : 'remote'}
        {#if remote.oidc_configured}· OIDC{/if}
        {#if remote.token_configured}· bearer{/if}
      </span>
    {/if}
  </header>

  <ViewSwitcher available={availableViews} />

  <CommandPalette
    open={paletteOpen}
    available={availableViews}
    onclose={() => (paletteOpen = false)}
    onaction={(a) => {
      if (a === 'refresh') window.location.reload();
    }}
  />

  {#if $activeView === 'agent'}
    <section class="agent">
      <h2>Agent (live)</h2>
      <p class="hint">
        RFC 63 §9 / RFC 65 §3. Step timeline of the capability layer, streamed via the Kernel Bus
        <code>agent_step</code> event (<code>atlas agent "&lt;task&gt;" --verify</code>).
      </p>
      <AgentCard hudUrl={data.hudUrl ?? null} />
      <DemoPane hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'kanban'}
    <section class="kanban">
      <h2>Missions</h2>
      <p class="hint">
        RFC 65 §3. Missions as cards across Pending / Running / Done / Failed (<code
          >GET /tail/missions</code
        >).
      </p>
      <KanbanBoard hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'approvals'}
    <section class="approvals">
      <h2>Approvals</h2>
      <p class="hint">
        RFC 65 §4 / RFC 24 §6. Pending <code>Confirm</code>-class actions (RFC 18); Approve/Deny fan
        out a decision on the Kernel Bus to every connected device.
      </p>
      <ApprovalQueue hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'cost'}
    <section class="cost-view">
      <h2>Cost &amp; Res</h2>
      <p class="hint">
        RFC 65 §3. Window totals and per-model spend from <code>model_invocations</code>, cumulative
        pressure, and the pending provider reset windows from <code>model_resets</code> (RFC 28 §H)
        via <code>GET /hud/cost</code>.
      </p>
      <CostDashboard hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'health'}
    <section class="health-view">
      <h2>Health KPIs</h2>
      <p class="hint">
        RFC 65 §3. Agent-session telemetry (<code>agent_session_events</code>), capability run
        states (<code>agent_runs</code>) and swarm-registry states (<code>swarm_agents</code>) via
        <code>GET /hud/health</code>; supervisor heartbeat liveness from the live
        <code>agent_heartbeat</code> bus event.
      </p>
      <HealthKPIs hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'audit'}
    <section class="audit-view">
      <h2>Audit</h2>
      <p class="hint">
        RFC 65 §3 / RFC 24 §10. The append-only <code>audit_log</code> chain, newest first, via
        <code>GET /hud/audit</code>. Hash-chain verification is not implemented yet and is not
        claimed here.
      </p>
      <AuditTimeline hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'canvas'}
    <section class="canvas-view">
      <h2>Canvas</h2>
      <p class="hint">
        RFC 65 §3 / RFC 28 §C. Persisted mission graph (<code>GET /graph/:id</code>, M15) rendered
        by <code>GraphView</code>; pick a mission to inspect its nodes and DFA edges.
      </p>
      <CanvasView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'outline'}
    <section class="outline-view">
      <h2>Outline</h2>
      <p class="hint">
        RFC 65 §3. A plan's roadmap milestones in order with their dependencies (<code
          >/tail/plans</code
        >
        + <code>/payload/plan/:id</code>).
      </p>
      <OutlineView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'timeline'}
    <section class="timeline-view-page">
      <h2>Timeline</h2>
      <p class="hint">
        RFC 65 §3. Chronological strip of the <code>journal_events</code> stream (<code
          >GET /hud/journal</code
        >).
      </p>
      <TimelineView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'worktrees'}
    <section class="worktrees-view-page">
      <h2>Worktrees</h2>
      <p class="hint">
        RFC 65 §3 / RFC 05 §4. Git worktrees of a repository (<code>GET /hud/worktrees</code>);
        fail-safe when git is missing or the path is not a repo.
      </p>
      <WorktreesView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'settings'}
    <section class="settings-view-page">
      <h2>Settings</h2>
      <p class="hint">
        RFC 25 §3.10. Provider API keys live in the <strong>OS keychain</strong> (Windows Credential Manager
        / macOS Keychain / Linux Secret Service) — never in files, prompts or logs.
      </p>
      <SettingsView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else if $activeView === 'mcp'}
    <section class="mcp-view-page">
      <h2>MCP servers</h2>
      <p class="hint">
        RFC 65 §10 / RFC 07. The local MCP catalog (<code>GET /hud/mcp</code>) with the policy the
        runtime enforces: sandbox (§2), supply chain (§3) and the tool allowlist (§4).
      </p>
      <McpView hudUrl={data.hudUrl ?? null} />
    </section>
  {:else}
    <section class="hud-health">
      <h2>HUD Mission Control</h2>
      <p>
        WS status:
        <span class="status" data-state={$hud.connected ? 'online' : 'offline'}>
          {$hud.connected ? 'connected' : 'disconnected'}
        </span>
      </p>
      <p class="hud-url">
        URL:
        <code>{$hud.url || data.hudUrl || 'waiting…'}</code>
      </p>
    </section>

    <section class="skill-mcp">
      <h2>Skill &amp; MCP rail</h2>
      <p class="hint">
        RFC 65 §10 / RFC 24 §8. Skills from <code>/tail/skills</code> and MCP servers from
        <code>GET /hud/mcp</code> (<code>.opencode/mcp.json</code>). Hot-swap activation is not
        wired in this build — drag only stages a selection.
      </p>
      <SkillMcpRail hudUrl={data.hudUrl ?? null} />
    </section>

    <section class="audit-export">
      <h2>Audit export — posting format</h2>
      <p class="hint">
        Export the most recent audit-log entries to <code>.posting.yaml</code> snapshots under
        <code>&lt;profile&gt;/snapshots/YYYY-MM-DD/</code>. Closes <em>RFC 27 §3.H Brecha H</em>.
        Format compatible with <code>darrenburns/posting</code> (Apache-2.0), no runtime dep. See
        <em>RFC 28 §D</em>.
      </p>
      <form
        onsubmit={(e) => {
          e.preventDefault();
          runExportPosting();
        }}
      >
        <label>
          Last N
          <input
            type="number"
            min="1"
            max="10000"
            bind:value={exportState.last}
            disabled={exportState.busy}
          />
        </label>
        <label>
          Output dir (optional, defaults to <code>&lt;profile&gt;/snapshots/</code>)
          <input
            type="text"
            placeholder="e.g. C:/snapshots or ./snap"
            bind:value={exportState.outputDir}
            disabled={exportState.busy}
          />
        </label>
        <button type="submit" disabled={exportState.busy || !data.hudUrl}>
          {exportState.busy ? 'exporting…' : 'Export as posting'}
        </button>
      </form>
      {#if exportState.error}
        <p class="error">Error: {exportState.error}</p>
      {/if}
      {#if exportState.result}
        <p class="success">
          Packed {exportState.result.entries_packed} entries into
          {exportState.result.files_written.length} file(s)
          {#if exportState.result.entries_purged > 0}
            (purged {exportState.result.entries_purged})
          {/if}
        </p>
        <p class="snap-root">Root: <code>{exportState.result.snapshot_root}</code></p>
        {#if exportState.result.files_written.length > 0}
          <ul class="snap-files">
            {#each exportState.result.files_written as f (f)}
              <li><code>{f}</code></li>
            {/each}
          </ul>
        {/if}
      {/if}
    </section>

    <section class="autoresearch">
      <h2>Autoresearch loop</h2>
      <p class="hint">
        RFC 28 §A. Greedy hill-climbing on git commits; supervisor-owned keep verdict. Card waits
        for telemetry from <code>opencode mission new --autoresearch</code>.
      </p>
      <AutoresearchCard snapshot={null} candidates={[]} hudUrl={data.hudUrl} />
    </section>

    <section class="swarm">
      <h2>Swarm Console</h2>
      <p class="hint">
        RFC 31 §B 4.5. Office floor (munder-difflin): desks follow the Kernel Bus
        <code>swarm_agent_spawned</code> / <code>swarm_state_changed</code> stream, the mailbox
        drawer follows <code>swarm_message</code>, and Checks polls the worktree (CN-004).
      </p>
      <SwarmConsole
        hudUrl={data.hudUrl ?? null}
        missionId={swarmMissionId}
        agents={swarmAgents}
        messages={swarmMessages}
      />
    </section>

    <section class="proactive">
      <h2>Proactive turn</h2>
      <p class="hint">
        RFC 20 Fase 23. Operator availability from the calendar busy windows + the mission backlog (<code
          >atlas calendar proactive</code
        >, <code>GET /hud/availability</code>).
      </p>
      <AvailabilityCard hudUrl={data.hudUrl ?? null} />
    </section>

    <section class="eval">
      <h2>Evaluation</h2>
      <p class="hint">
        RFC 20 Fase 22. pass rate, tokens/solved, $/solved and the failure-kind vector over recent
        runs (<code>atlas eval run golden</code>, <code>atlas eval import &lt;job-dir&gt;</code>).
      </p>
      <EvalCard hudUrl={data.hudUrl ?? null} />
    </section>

    <section class="journal">
      <h2>Journal tail (live)</h2>
      {#if $hud.events.length === 0}
        <p class="empty">No events yet. Try: <code>opencode mission new "hello world"</code>.</p>
      {:else}
        <ul>
          {#each $hud.events as evt (evt.id)}
            <li>
              <time>{evt.ts}</time>
              <span class="kind">{evt.kind}</span>
              <span class="payload">{JSON.stringify(evt.payload).slice(0, 120)}</span>
            </li>
          {/each}
        </ul>
      {/if}
    </section>

    <section class="observer">
      <h2>Journal Observer</h2>
      <p class="hint">
        RFC 19 §10 / research 33 §B 6.1. Full inspection over
        <code>GET /hud/journal</code>: newest-first pages with expandable payload, kind filter and
        pagination.
      </p>
      <JournalObserver hudUrl={data.hudUrl ?? null} />
    </section>

    <section class="tails">
      <h2>Pipeline tails</h2>
      <div class="tails-grid">
        {#each Object.values(tails) as box (box.kind)}
          <article class="tail-box" data-kind={box.kind}>
            <header>
              <h3>{box.title}</h3>
              <span class="count">{box.rows.length}</span>
            </header>
            {#if box.error}
              <p class="error">{box.error}</p>
            {:else if box.rows.length === 0}
              <p class="empty">No rows yet.</p>
            {:else if box.kind === 'step_states'}
              {#each box.rows as row (rowId(row))}
                <p class="step-row">
                  <code>{String(row.step_id ?? '').slice(0, 8)}</code>
                  <span class="pill" data-phase={pillColor(row)}>{row.phase}</span>
                </p>
              {/each}
            {:else if box.kind === 'model_swaps'}
              {#each box.rows as row (rowId(row))}
                <p class="swap-row">
                  <code>{row.prev_model_id}</code>
                  <span aria-hidden="true">→</span>
                  <code class="swap-new">{row.new_model_id}</code>
                  <span class="swap-init" data-by={row.initiator}>{row.initiator}</span>
                </p>
              {/each}
            {:else}
              <ul>
                {#each box.rows as row (rowId(row))}
                  <li>
                    <code>{rowId(row).slice(0, 8)}</code>
                    <span>{rowSummary(row)}</span>
                    {#if box.kind === 'diffs'}
                      <button
                        type="button"
                        class="annotate-btn"
                        onclick={(e) => {
                          e.stopPropagation();
                          void openAnnotationDrawer(row);
                        }}
                        aria-label="Annotate diff">✎</button
                      >
                    {/if}
                  </li>
                {/each}
              </ul>
            {/if}
          </article>
        {/each}
      </div>
    </section>

    {#if drawer.open}
      <aside class="drawer" data-open>
        <header>
          <h3>Annotate diff <code>{drawer.diffLabel}</code></h3>
          <button type="button" class="drawer-close" onclick={closeDrawer} aria-label="Close"
            >×</button
          >
        </header>

        <section class="drawer-ann">
          <h4>Annotations</h4>
          {#if drawer.loading}
            <p class="empty">Loading…</p>
          {:else if drawer.error}
            <p class="error">{drawer.error}</p>
          {:else if drawer.annotations.length === 0}
            <p class="empty">No annotations yet.</p>
          {:else}
            <ul>
              {#each drawer.annotations as ann (ann.id)}
                <li>
                  <div class="ann-head">
                    <span class="ann-author">{ann.author}</span>
                    <time>{ann.created_at}</time>
                  </div>
                  <div class="ann-loc">
                    {#if ann.file_path}<code>{ann.file_path}</code>{:else}<span class="dim"
                        >diff-level</span
                      >{/if}
                    {#if ann.line_no != null}<span class="pill pill-loc">L{ann.line_no}</span>{/if}
                  </div>
                  <p class="ann-body">{ann.body}</p>
                </li>
              {/each}
            </ul>
          {/if}
        </section>

        <form class="drawer-form" onsubmit={(e) => void submitAnnotation(e as SubmitEvent)}>
          <h4>Add annotation</h4>
          <label>
            <span>Author</span>
            <input bind:value={drawer.draftAuthor} placeholder="max" required />
          </label>
          <label>
            <span>Body</span>
            <textarea
              bind:value={drawer.draftBody}
              placeholder="Suggestion, question, fix hint…"
              rows="3"
              required
            ></textarea>
          </label>
          <div class="form-row">
            <label>
              <span>File path (opt.)</span>
              <input bind:value={drawer.draftFilePath} placeholder="src/lib.rs" />
            </label>
            <label>
              <span>Line no (opt.)</span>
              <input type="number" min="1" bind:value={drawer.draftLineNo} placeholder="42" />
            </label>
          </div>
          {#if drawer.submitError}
            <p class="error">{drawer.submitError}</p>
          {/if}
          <div class="form-actions">
            <button type="submit" disabled={drawer.submitting || !data.hudUrl}>
              {drawer.submitting ? 'Posting…' : 'Post'}
            </button>
          </div>
        </form>
      </aside>
    {/if}
  {/if}
</main>

<style>
  :global(html) {
    font-family:
      'Inter',
      system-ui,
      -apple-system,
      sans-serif;
    background: #0d1117;
    color: #c9d1d9;
  }
  main {
    max-width: 1100px;
    margin: 0 auto;
    padding: 1.5rem;
  }
  header {
    display: flex;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 1.5rem;
  }
  h1 {
    font-size: 1.6rem;
    margin: 0;
  }
  .version {
    opacity: 0.6;
    font-family: 'Fira Code', monospace;
  }
  section {
    margin-bottom: 2rem;
    border: 1px solid #30363d;
    background: #161b22;
    border-radius: 8px;
    padding: 1rem 1.25rem;
  }
  h2 {
    font-size: 1.1rem;
    margin-top: 0;
    color: #58a6ff;
  }
  .status[data-state='online'] {
    color: #3fb950;
  }
  .status[data-state='offline'] {
    color: #f85149;
  }
  code {
    background: #21262d;
    padding: 0.1rem 0.35rem;
    border-radius: 4px;
    font-family: 'Fira Code', 'JetBrains Mono', monospace;
  }
  .journal ul {
    list-style: none;
    padding-left: 0;
    margin: 0;
  }
  .journal li {
    display: grid;
    grid-template-columns: 14rem 14rem 1fr;
    gap: 1rem;
    font-family: 'Fira Code', monospace;
    font-size: 0.85rem;
    padding: 0.35rem 0;
    border-bottom: 1px solid #21262d;
  }
  .journal time {
    opacity: 0.55;
  }
  .kind {
    color: #79c0ff;
  }
  .payload {
    opacity: 0.9;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .empty {
    opacity: 0.6;
  }
  .audit-export {
    margin-top: 1rem;
    padding: 0.75rem 1rem;
    border: 1px solid #21262d;
    border-radius: 6px;
    background: #161b22;
  }
  .audit-export h2 {
    margin: 0 0 0.5rem 0;
    font-size: 1rem;
  }
  .audit-export .hint {
    margin: 0 0 0.75rem 0;
    font-size: 0.85rem;
    color: #8b949e;
  }
  .audit-export form {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: flex-end;
  }
  .audit-export label {
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    font-size: 0.78rem;
    color: #8b949e;
  }
  .audit-export input[type='text'],
  .audit-export input[type='number'] {
    padding: 0.4rem 0.5rem;
    background: #0d1117;
    color: #c9d1d9;
    border: 1px solid #30363d;
    border-radius: 4px;
    min-width: 10rem;
  }
  .audit-export button[type='submit'] {
    padding: 0.45rem 1rem;
    background: #238636;
    color: #fff;
    border: none;
    border-radius: 4px;
    cursor: pointer;
  }
  .audit-export button[type='submit']:disabled {
    background: #21262d;
    color: #8b949e;
    cursor: not-allowed;
  }
  .audit-export .error {
    margin-top: 0.5rem;
    color: #f85149;
    font-size: 0.85rem;
  }
  .audit-export .success {
    margin-top: 0.5rem;
    color: #56d364;
    font-size: 0.85rem;
  }
  .audit-export .snap-root,
  .audit-export .snap-files {
    margin-top: 0.25rem;
    font-size: 0.8rem;
    color: #8b949e;
  }
  .audit-export .snap-files {
    list-style: square;
    padding-left: 1.25rem;
  }
  .autoresearch {
    margin-top: 1rem;
  }
  .autoresearch h2 {
    font-size: 1rem;
    margin: 0 0 0.25rem 0;
  }
  .autoresearch .hint {
    margin: 0 0 0.5rem 0;
    font-size: 0.85rem;
    color: #8b949e;
  }
  .swarm {
    margin-top: 1rem;
  }
  .swarm h2 {
    font-size: 1rem;
    margin: 0 0 0.25rem 0;
  }
  .swarm .hint {
    margin: 0 0 0.5rem 0;
    font-size: 0.85rem;
    color: #8b949e;
  }
  .observer {
    margin-top: 1rem;
  }
  .observer h2 {
    font-size: 1rem;
    margin: 0 0 0.25rem 0;
  }
  .observer .hint {
    margin: 0 0 0.5rem 0;
    font-size: 0.85rem;
    color: #8b949e;
  }
  .tails-grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 0.75rem;
  }
  .tail-box {
    border: 1px solid #30363d;
    background: #0d1117;
    border-radius: 6px;
    padding: 0.6rem 0.75rem;
    min-height: 140px;
  }
  .tail-box header {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin: 0 0 0.4rem 0;
  }
  .tail-box h3 {
    font-size: 0.85rem;
    color: #79c0ff;
    margin: 0;
  }
  .tail-box .count {
    font-family: 'Fira Code', monospace;
    font-size: 0.7rem;
    opacity: 0.6;
  }
  .tail-box ul {
    list-style: none;
    padding: 0;
    margin: 0;
    font-family: 'Fira Code', monospace;
    font-size: 0.72rem;
  }
  .tail-box li {
    display: flex;
    gap: 0.4rem;
    padding: 0.18rem 0;
    border-bottom: 1px solid #161b22;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .tail-box .error {
    color: #f85149;
    font-size: 0.75rem;
  }
  .annotate-btn {
    background: transparent;
    color: #79c0ff;
    border: 1px solid #30363d;
    border-radius: 4px;
    padding: 0 0.35rem;
    cursor: pointer;
    font-size: 0.75rem;
    line-height: 1.2;
  }
  .annotate-btn:hover {
    background: #161b22;
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
    justify-content: space-between;
    align-items: baseline;
    gap: 1rem;
    margin-bottom: 1rem;
    border: 0;
    background: transparent;
    padding: 0;
  }
  .drawer h3 {
    font-size: 0.95rem;
    margin: 0;
    color: #58a6ff;
  }
  .drawer-close {
    background: transparent;
    color: #c9d1d9;
    border: 0;
    font-size: 1.3rem;
    cursor: pointer;
    line-height: 1;
  }
  .drawer-ann h4,
  .drawer-form h4 {
    font-size: 0.8rem;
    color: #79c0ff;
    margin: 0 0 0.5rem 0;
  }
  .drawer-ann {
    margin-bottom: 1.25rem;
    padding: 0;
    border: 0;
    background: transparent;
  }
  .drawer-ann ul {
    list-style: none;
    padding: 0;
    margin: 0;
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
  }
  .drawer-ann li {
    border: 1px solid #21262d;
    border-radius: 6px;
    padding: 0.5rem 0.6rem;
    background: #161b22;
  }
  .ann-head {
    display: flex;
    justify-content: space-between;
    align-items: baseline;
    margin-bottom: 0.25rem;
    font-size: 0.7rem;
    font-family: 'Fira Code', monospace;
  }
  .ann-author {
    color: #58a6ff;
  }
  .ann-head time {
    opacity: 0.5;
  }
  .ann-loc {
    display: flex;
    gap: 0.4rem;
    align-items: baseline;
    margin-bottom: 0.35rem;
    font-size: 0.7rem;
  }
  .ann-loc .dim {
    opacity: 0.55;
  }
  .pill-loc {
    color: #f0883e;
    border: 1px solid #f0883e;
  }
  .ann-body {
    margin: 0;
    font-size: 0.8rem;
    white-space: pre-wrap;
    word-break: break-word;
  }
  .drawer-form {
    display: flex;
    flex-direction: column;
    gap: 0.6rem;
    padding: 0;
    border: 0;
    background: transparent;
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
  .drawer-form .form-row {
    display: flex;
    gap: 0.6rem;
  }
  .drawer-form .form-row label {
    flex: 1;
  }
  .drawer-form .form-actions {
    display: flex;
    justify-content: flex-end;
    margin-top: 0.4rem;
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
  .remote {
    font-size: 0.72rem;
    border-radius: 999px;
    padding: 0.05rem 0.5rem;
    border: 1px solid #30363d;
    color: #8b949e;
    font-family: 'Fira Code', monospace;
  }
  .remote[data-state='remote'] {
    color: #58a6ff;
    border-color: #58a6ff;
  }
  .skill-mcp {
    margin-top: 1rem;
  }
  .skill-mcp h2 {
    font-size: 1rem;
    margin: 0 0 0.25rem 0;
  }
  .skill-mcp .hint {
    margin: 0 0 0.5rem 0;
    font-size: 0.85rem;
    color: #8b949e;
  }

  /* RFC 65 §11 / RFC 24 §16 — responsive (mobile review). */
  @media (max-width: 720px) {
    main {
      padding: 0.9rem;
    }
    header {
      flex-wrap: wrap;
      gap: 0.5rem;
    }
    section {
      padding: 0.75rem 0.85rem;
    }
    .tails-grid {
      grid-template-columns: 1fr;
    }
    .journal li {
      grid-template-columns: 1fr;
    }
    .drawer {
      width: 100vw;
      max-width: 100vw;
    }
  }
</style>

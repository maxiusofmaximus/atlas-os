// Atlas OS — HUD Svelte store (RFC 24 §4).
// Connects to the local axum WebSocket server; keeps a rolling buffer of
// the last 200 kernel-bus events so any subscribed component can render
// Mission Control from anywhere.
//
// The HUD survives webview crashes (RFC 25 §2). If the WS dies, we wait
// 1s and reconnect with exponential backoff (cap 10s).
// Phase 0: tail only; Phase 8 will add Kanban/cards/Canvas views.
//
// The connecting URL is provided by the consumer (the +page.svelte passes
// the Hud URL obtained from the Rust core via the `hud_url` IPC command).
// This is required because the axum HUD server binds to an ephemeral port
// known only at runtime — `window.location.port` (Vite's 5173) is NOT the
// HUD port and would always fail to connect.

import { writable, type Readable } from 'svelte/store';

export interface HudEvent {
  id: string;
  ts: string;
  kind: string;
  payload: unknown;
}

export interface HudState {
  connected: boolean;
  url: string | null;
  events: HudEvent[];
}

// ────────────── RFC 27 §B / §G — typed tail rows ──────────────
//
// Mirror the Rust `ModelSwapRow` and `StepStateRow` so the Mission
// Control UI renders colour-coded step pills and a swap timeline
// without an `unknown` cast.

export interface ModelSwapRow {
  swap_id: string;
  mission_id: string;
  prev_model_id: string;
  new_model_id: string;
  initiator: 'user' | 'auto';
  occurred_at: string;
}

export interface StepStateRow {
  mission_id: string;
  plan_id: string;
  step_id: string;
  phase: StepPhaseTag;
  updated_at: string;
}

export type StepPhaseTag = 'pending' | 'executing' | 'verifying' | 'done' | 'blocked';

/** RFC 27 §G — colour token for a step pill. Mirrors the planner's
 * `StepPhase` enum so the UI never has to guess. */
export function phaseColor(phase: StepPhaseTag): string {
  switch (phase) {
    case 'pending':
      return 'grey';
    case 'executing':
      return 'blue';
    case 'verifying':
      return 'amber';
    case 'done':
      return 'green';
    case 'blocked':
      return 'red';
  }
}

// ────────────── RFC 27 §E — diff annotation drawer helpers ──────────────
//
// Mirror the Rust `DiffAnnotationRow` returned by `GET /diff/{id}/annotation`
// and the POST echo (`AnnotationPosted`) so the Svelte drawer avoids
// `unknown` casts. `exactOptionalPropertyTypes: true` is honoured:
// optionals are typed `T | null` and only sent when non-null.

export interface DiffAnnotation {
  id: string;
  diff_id: string;
  file_path: string | null;
  line_no: number | null;
  body: string;
  author: string;
  created_at: string;
}

export interface AnnotationPosted {
  id: string;
  diff_id: string;
  created_at: string;
}

export interface AnnotationPostInput {
  body: string;
  author: string;
  file_path?: string | null;
  line_no?: number | null;
}

export async function fetchAnnotations(hudUrl: string, diffId: string): Promise<DiffAnnotation[]> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/diff/${encodeURIComponent(diffId)}/annotation`);
  if (!res.ok) {
    throw new Error(`HUD annotations GET failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as DiffAnnotation[];
}

export async function postAnnotation(
  hudUrl: string,
  diffId: string,
  input: AnnotationPostInput,
): Promise<AnnotationPosted> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const payload: Record<string, unknown> = { body: input.body, author: input.author };
  if (input.file_path != null && input.file_path.trim() !== '') {
    payload.file_path = input.file_path;
  }
  if (input.line_no != null && Number.isFinite(input.line_no) && input.line_no > 0) {
    payload.line_no = input.line_no;
  }
  const res = await fetch(`${trimmed}/diff/${encodeURIComponent(diffId)}/annotation`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(payload),
  });
  if (!res.ok) {
    throw new Error(`HUD annotations POST failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as AnnotationPosted;
}

const MAX_EVENTS = 200;
const initialState: HudState = {
  connected: false,
  url: null,
  events: [],
};

const { subscribe, update } = writable<HudState>(initialState);

let socket: WebSocket | null = null;
let reconnectTimer: ReturnType<typeof setTimeout> | null = null;
let reconnBackoff = 1000;

/**
 * Convert an HTTP HUD URL (e.g. `http://localhost:57457/`) into the
 * corresponding WebSocket URL the axum server expects (`ws://localhost:57457/ws`).
 *
 * - Strips the trailing slash so appending `/ws` always land.
 * - Preserves an existing `/ws` suffix so calling `toWsUrl(toWsUrl(x))`
 *   is idempotent.
 * - Replaces the `http`/`https` scheme with `ws`/`wss`.
 *
 * Exported so tests can pin the behaviour and so future embedders
 * (CLI bridge, mobile remote access — RFC 24 §16) reuse the same
 * canonicaliser instead of inlining their own.
 */
export function toWsUrl(url: string): string {
  return url.replace(/^http/, 'ws').replace(/\/$/, '') + (url.endsWith('/ws') ? '' : '/ws');
}

// ────────────── RFC 24 §2 — tail artefact kinds ──────────────
//
// Phase 1 exposes nine tail routes on the axum HUD server:
//
//   /tail/journal            /tail/missions          /tail/verdicts
//   /tail/consolidated       /tail/plans             /tail/diffs
//   /tail/validation_reports /tail/repairs            /tail/patterns
//   /tail/checkpoints        /tail/skills            /tail/model_swaps
//   /tail/step_states
//
// Each returns the latest N rows (default 20, max 200) as a JSON array.
// The store exposes a generic `fetchTail(kind, last?)` helper plus a
// type map so Mission Control components can weed by `kind` without
// re-defining the row shapes.

export type TailKind =
  | 'journal'
  | 'missions'
  | 'verdicts'
  | 'consolidated'
  | 'plans'
  | 'diffs'
  | 'validation_reports'
  | 'repairs'
  | 'patterns'
  | 'checkpoints'
  | 'skills'
  | 'model_swaps'
  | 'step_states'
  | 'agent_steps';

/**
 * Fetch a tail. `hudUrl` is the HTTP root URL (no `/tail/` segment).
 * `last` defaults to the server default (20) when omitted.
 *
 * Throws on HTTP failure or non-200 status so the caller can surface
 * the error in the Mission Control UI (RFC 24 §3).
 */
export async function fetchTail<T = unknown>(
  hudUrl: string,
  kind: TailKind,
  last?: number,
): Promise<T[]> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = last ? `?last=${encodeURIComponent(last)}` : '';
  const res = await fetch(`${trimmed}/tail/${kind}${query}`);
  if (!res.ok) {
    throw new Error(`HUD tail "${kind}" failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as T[];
}

// ────────────── RFC 20 Fase 22 (EVAL.2) — evaluation metrics ──────────────
//
// Mirrors `crate::eval::metrics::EvalSummary` so the `<EvalCard>` renders the
// normalized metrics the field converges on (pass rate, tokens/solved,
// $/solved, failure-kind histogram) plus the per harness × model breakdown.

export interface EvalSummary {
  runs: number;
  total: number;
  passed: number;
  failed: number;
  errored: number;
  pass_rate: number;
  tokens_total: number;
  tokens_per_solved: number;
  no_action_turns: number;
  cost_usd: number;
  cost_per_solved: number;
  failure_kinds: Record<string, number>;
}

export interface EvalGroupSummary {
  key: string;
  summary: EvalSummary;
}

export interface EvalSummaryResponse {
  summary: EvalSummary;
  groups: EvalGroupSummary[];
}

/**
 * Fetch the aggregated evaluation metrics. `hudUrl` is the HTTP root URL.
 * Throws on HTTP failure or non-200 status so the card can surface the error.
 */
export async function fetchEvalSummary(
  hudUrl: string,
  last?: number,
): Promise<EvalSummaryResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = last ? `?last=${encodeURIComponent(last)}` : '';
  const res = await fetch(`${trimmed}/hud/eval/summary${query}`);
  if (!res.ok) {
    throw new Error(`HUD eval summary failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as EvalSummaryResponse;
}

// ────────────── RFC 20 Fase 23 (v3.1.2.3) — proactive availability ──────────
//
// Mirrors `crate::planning::availability::Availability` + the persisted
// `ProactivePolicyRow` served by `GET /hud/availability`.

export type Availability = 'RunNow' | 'Blocked' | { WaitUntil: number };

export interface AvailabilityResponse {
  enabled: boolean;
  policy: {
    eta_ms: number;
    weight_threshold: number;
    horizon_ms: number;
    enabled: boolean;
  };
  availability: Availability | null;
  pending_mission: string | null;
}

export async function fetchAvailability(hudUrl: string): Promise<AvailabilityResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/hud/availability`);
  if (!res.ok) {
    throw new Error(`HUD availability failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as AvailabilityResponse;
}

/** Normalise a raw WS frame into a `HudEvent`. The axum bridge forwards the
 *  whole `BusEvent` (`{ id, kind: { type, ...fields }, ts }`); tests/older
 *  producers may send a flat `{ kind: "x", payload }`. Both are accepted, and
 *  the internally-tagged object becomes the event's `payload` so projections
 *  can read its fields (`payload.agent`, `payload.run_id`, …). */
export function normalizeWsEvent(raw: unknown): HudEvent {
  const r = (raw ?? {}) as { id?: string; ts?: string; kind?: unknown; payload?: unknown };
  let kind: string;
  let payload: unknown;
  if (r.kind && typeof r.kind === 'object') {
    const k = r.kind as { type?: string };
    kind = k.type ?? 'unknown';
    payload = r.kind;
  } else {
    kind = typeof r.kind === 'string' ? r.kind : 'unknown';
    payload = r.payload;
  }
  return {
    id: r.id ?? crypto.randomUUID(),
    ts: r.ts ?? new Date().toISOString(),
    kind,
    payload,
  };
}

function connectWs(target: string) {
  let ws: WebSocket;
  try {
    ws = new WebSocket(target);
  } catch (err) {
    console.warn('HUD WS construct failed', err);
    scheduleReconnect(target);
    return;
  }
  socket = ws;
  ws.addEventListener('open', () => {
    reconnBackoff = 1000;
    update((s) => ({ ...s, connected: true, url: target }));
  });
  ws.addEventListener('message', (msg) => {
    try {
      const item = normalizeWsEvent(JSON.parse(msg.data as string));
      update((s) => ({ ...s, events: [...s.events, item].slice(-MAX_EVENTS) }));
    } catch (err) {
      console.warn('HUD WS message parse failed', err);
    }
  });
  ws.addEventListener('close', () => {
    socket = null;
    update((s) => ({ ...s, connected: false }));
    scheduleReconnect(target);
  });
  ws.addEventListener('error', () => {
    try {
      ws?.close();
    } catch {
      /* ignore */
    }
  });
}

function scheduleReconnect(target: string) {
  if (reconnectTimer) clearTimeout(reconnectTimer);
  reconnectTimer = setTimeout(() => {
    reconnBackoff = Math.min(reconnBackoff * 2, 10_000);
    connectWs(target);
  }, reconnBackoff);
}

export const hud: Readable<HudState> & {
  connect: (target?: string) => void;
  disconnect: () => void;
} = {
  subscribe,
  connect(target?: string) {
    if (!target || socket) return;
    const resolved = target.startsWith('ws') ? target : toWsUrl(target);
    connectWs(resolved);
  },
  disconnect() {
    if (reconnectTimer) clearTimeout(reconnectTimer);
    socket?.close();
    socket = null;
    update((s) => ({ ...s, connected: false }));
  },
};

// ═══════════════════ RFC 28 §D — POST /audit/export-posting ═══════════════════
//
// HUD frontend helper for the "Export as posting" button on the Audit card.
// Calls the axum route with optional `last` and `output_dir` and returns the
// resolved list of written files. The caller prompts for `output_dir` via a
// Tauri save dialog if desired (handled in +page.svelte).

export interface ExportPostingRequest {
  last?: number;
  output_dir?: string;
}

export interface ExportPostingResponse {
  files_written: string[];
  entries_packed: number;
  entries_purged: number;
  snapshot_root: string;
}

export async function postExportPosting(
  hudUrl: string,
  req: ExportPostingRequest,
): Promise<ExportPostingResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/audit/export-posting`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(`HUD audit export failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as ExportPostingResponse;
}

// ────────────── RFC 28 §A — Autoresearch telemetry ──────────────
// The supervisor publishes RunSnapshot on every transition; HUD renders
// the card via the AutoresearchCard. We map the Rust types here:
//   RunSnapshot (Rust) -> AutoresearchSnapshot (TS)
//   Outcome (Rust tag) -> AutoresearchOutcome (TS union)

export type AutoresearchOutcome = 'running' | 'improved' | 'plateau' | 'timeout' | 'aborted';

export interface AutoresearchSnapshot {
  id: string;
  mission_id: string;
  baseline_metric: number;
  best_metric: number | null;
  git_sha_start: string;
  git_sha_end: string | null;
  metric_command: string;
  max_steps: number;
  timebox_seconds: number;
  step_count: number;
  outcome: AutoresearchOutcome;
  ts_started: number;
  ts_ended: number | null;
}

export interface AutoresearchCandidate {
  step: number;
  git_sha: string;
  diff_hunk: string;
  metric_baseline_at_step: number;
  metric_after: number;
  kept: boolean;
  rationale: string;
}

// HUD action for `AutoresearchCard`:
//   Pause   -> emit `session/cancel` with `outcome: 'aborted'`
//   Stop    -> same, but host marks it intentional manual stop
// The action is carried over the existing WS connection; the Rust core
// already understands `session/cancel` from RFC 27 §F.
export interface AutoresearchCancel {
  run_id: string;
  outcome: 'aborted';
}

export async function postAutoresearchCancel(
  hudUrl: string,
  req: AutoresearchCancel,
): Promise<void> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/autoresearch/cancel`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(`HUD autoresearch cancel failed: ${res.status} ${res.statusText}`);
  }
}

// �∂∂∂∂∂∂∂∂∂∂∂∂ RFC 28 §C item 7 — graph view typed client ∂∂∂∂∂∂∂∂∂∂∂∂
//
// Mirrors the Rust `graph::MissionGraph` payload emitted by
// `GET /graph/:mission_id`. Keep these types in sync with
// `src-tauri/src/graph/mod.rs` (the route serialises that struct
// verbatim). The HUD `<GraphView>` component consumes these.

export type Provenance = 'EXTRACTED' | 'INFERRED' | 'AMBIGUOUS';

export type NodeKind = 'engine_state' | 'mission' | 'skill' | 'external';

export type EdgeKind = 'calls' | 'imports' | 'transitions_to' | 'depends_on' | 'references';

export interface GraphNode {
  id: string;
  mission_id: string;
  kind: NodeKind;
  label: string;
  provenance: Provenance;
  attrs_json: string;
}

export interface GraphEdge {
  id: string;
  mission_id: string;
  src: string;
  dst: string;
  kind: EdgeKind;
  precondition: string | null;
  guard: string | null;
  visit_count: number;
}

export interface MissionGraph {
  mission_id: string;
  nodes: GraphNode[];
  edges: GraphEdge[];
}

export async function fetchGraph(hudUrl: string, missionId: string): Promise<MissionGraph> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/graph/${encodeURIComponent(missionId)}`, {
    method: 'GET',
    headers: { accept: 'application/json' },
  });
  if (res.status === 404) {
    throw new Error(`No graph persisted for mission ${missionId}`);
  }
  if (!res.ok) {
    throw new Error(`HUD graph fetch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as MissionGraph;
}

// ────────────── RFC 28 §H.3 / §H.4 — Spend-limit & Model-ready cards ──────────────
//
// Card payloads mirrored from the Rust `BusEventKind::SpendLimitObserved`
// event and the Toast `kind='model_ready'` body. The HUD `+page.svelte`
// projects the WS stream onto these typed objects so the Svelte components
// avoid `unknown` casts.

export type SpendLimitErrorType = 'rate_limit' | 'spend_limit';

export interface SpendLimitErrorCardPayload {
  provider: string;
  model: string;
  status_code: number;
  error_type: SpendLimitErrorType;
  resets_at: string;
  request_id: string | null;
  toast_enqueued_id: number | null;
}

export interface ModelReadyCardPayload {
  provider: string;
  model: string;
  resets_at: string;
  mission_id: string | null;
  toast_queue_id: number;
}

export interface ProfileSwitchRequest {
  backup_profile_id: string;
}

export interface ProfileSwitchResponse {
  ok: boolean;
  new_profile_id: string;
}

export async function postProfileSwitch(
  hudUrl: string | null,
  backupProfileId: string,
): Promise<ProfileSwitchResponse> {
  const trimmed = (hudUrl ?? '').replace(/\/$/, '');
  if (!trimmed) {
    throw new Error('HUD URL unavailable — could not switch profile');
  }
  const req: ProfileSwitchRequest = { backup_profile_id: backupProfileId };
  const res = await fetch(`${trimmed}/profile/switch`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(`HUD profile switch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as ProfileSwitchResponse;
}

// ────────────── Phase 4.5 — Swarm Console (RFC 31 SECTOR B 4.5 + A.2) ──────────────
//
// Mirrors the Rust M29 rows (`swarm_agents` / `agent_mailbox`) and the Kernel
// Bus broadcast kinds `swarm_agent_spawned` / `swarm_message` /
// `swarm_state_changed`. The `<SwarmConsole>` office floor derives desks and
// the mailbox drawer purely from the WS tail via `projectSwarmAgents` /
// `projectSwarmInbox`, so no extra subscription is needed; the `fetch*`
// helpers below cover the REST fallback (`atlas swarm inbox` equivalent) and
// the per-worktree checks button (CN-004).

export type SwarmRole =
  | 'planner'
  | 'researcher'
  | 'architect'
  | 'backend'
  | 'frontend'
  | 'database'
  | 'security'
  | 'testing'
  | 'reviewer'
  | 'merger';

export type SwarmAgentState =
  'spawned' | 'idle' | 'working' | 'waiting_review' | 'blocked' | 'done' | 'failed';

export interface SwarmAgent {
  agent_id: string;
  mission_id: string;
  role: SwarmRole;
  model_id: string;
  state: SwarmAgentState;
  worktree_path: string | null;
  updated_at: string;
}

export interface SwarmMessage {
  id: string;
  from_agent: string;
  to_agent: string;
  body: string;
  read_at: string | null;
  created_at: string;
}

export interface SwarmAgentSpawnedPayload {
  agent: SwarmAgent;
}

export interface SwarmMessagePayload {
  message: SwarmMessage;
}

export interface SwarmStateChangedPayload {
  agent_id: string;
  mission_id: string;
  state: SwarmAgentState;
  updated_at: string;
}

export const SWARM_EVENT_KINDS = [
  'swarm_agent_spawned',
  'swarm_message',
  'swarm_state_changed',
] as const;

export type SwarmEventKind = (typeof SWARM_EVENT_KINDS)[number];

/** Colour token for an agent desk / state pill. Mirrors `phaseColor` above. */
export function swarmStateColor(state: SwarmAgentState): string {
  switch (state) {
    case 'spawned':
      return 'grey';
    case 'idle':
      return 'grey';
    case 'working':
      return 'blue';
    case 'waiting_review':
      return 'amber';
    case 'blocked':
      return 'red';
    case 'done':
      return 'green';
    case 'failed':
      return 'red';
  }
}

/**
 * Fold the WS tail into the current desk roster: last `swarm_agent_spawned`
 * wins per `agent_id`, later `swarm_state_changed` events patch the state.
 * Unknown payloads are skipped so a malformed broadcast never breaks the floor.
 */
export function projectSwarmAgents(events: HudEvent[]): SwarmAgent[] {
  const byId = new Map<string, SwarmAgent>();
  for (const evt of events) {
    if (evt.kind === 'swarm_agent_spawned') {
      const agent = (evt.payload as SwarmAgentSpawnedPayload | null)?.agent;
      if (agent?.agent_id) {
        byId.set(agent.agent_id, agent);
      }
    } else if (evt.kind === 'swarm_state_changed') {
      const change = evt.payload as SwarmStateChangedPayload | null;
      if (!change?.agent_id) continue;
      const current = byId.get(change.agent_id);
      if (current) {
        byId.set(change.agent_id, {
          ...current,
          state: change.state,
          updated_at: change.updated_at,
        });
      }
    }
  }
  return [...byId.values()];
}

/**
 * Fold the WS tail into one agent's inbox: every `swarm_message`
 * addressed to `agentId`, in broadcast order.
 */
export function projectSwarmInbox(events: HudEvent[], agentId: string): SwarmMessage[] {
  const out: SwarmMessage[] = [];
  for (const evt of events) {
    if (evt.kind !== 'swarm_message') continue;
    const message = (evt.payload as SwarmMessagePayload | null)?.message;
    if (message?.to_agent === agentId) {
      out.push(message);
    }
  }
  return out;
}

/** Unread count for the mailbox drawer badge (`read_at == null`). */
export function countUnread(messages: SwarmMessage[]): number {
  return messages.filter((m) => m.read_at == null).length;
}

export async function fetchSwarmAgents(hudUrl: string, missionId: string): Promise<SwarmAgent[]> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/swarm/${encodeURIComponent(missionId)}/agents`);
  if (!res.ok) {
    throw new Error(`HUD swarm agents fetch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as SwarmAgent[];
}

export async function fetchSwarmInbox(hudUrl: string, agentId: string): Promise<SwarmMessage[]> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/swarm/inbox/${encodeURIComponent(agentId)}`);
  if (!res.ok) {
    throw new Error(`HUD swarm inbox fetch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as SwarmMessage[];
}

export interface SwarmSendRequest {
  from_agent: string;
  to_agent: string;
  body: string;
}

export interface SwarmSendResponse {
  id: string;
}

export async function postSwarmSend(
  hudUrl: string,
  req: SwarmSendRequest,
): Promise<SwarmSendResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/swarm/send`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(`HUD swarm send failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as SwarmSendResponse;
}

export type SwarmCheckStatus = 'pass' | 'fail' | 'pending';

export interface SwarmCheck {
  name: string;
  status: SwarmCheckStatus;
  detail: string | null;
}

export async function fetchSwarmChecks(
  hudUrl: string,
  missionId: string,
  agentId: string,
): Promise<SwarmCheck[]> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(
    `${trimmed}/swarm/${encodeURIComponent(missionId)}/${encodeURIComponent(agentId)}/checks`,
  );
  if (!res.ok) {
    throw new Error(`HUD swarm checks fetch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as SwarmCheck[];
}

export interface MissionResumeRequest {
  mission_id: string;
}

export interface MissionResumeResponse {
  ok: boolean;
  mission_id: string;
}

export async function postMissionResume(
  hudUrl: string | null,
  missionId: string,
): Promise<MissionResumeResponse> {
  const trimmed = (hudUrl ?? '').replace(/\/$/, '');
  if (!trimmed) {
    throw new Error('HUD URL unavailable — could not resume mission');
  }
  const req: MissionResumeRequest = { mission_id: missionId };
  const res = await fetch(`${trimmed}/mission/resume`, {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify(req),
  });
  if (!res.ok) {
    throw new Error(`HUD mission resume failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as MissionResumeResponse;
}

// ────────────── RFC 19 §10 — Journal Observer (research 33 SECTOR B 6.1) ──────────────
//
// Mirrors the Rust `JournalEntry` row plus the `JournalPageResponse`
// envelope emitted by `GET /hud/journal`. Unlike `fetchTail` (newest-N
// projections, several without payloads), the observer returns complete
// payloads with `limit`/`offset` pagination and an optional `kind`
// filter so the operator can isolate one event stream.

export interface JournalObserverEntry {
  id: number;
  ts: string;
  kind: string;
  payload: unknown;
}

export interface JournalPageParams {
  limit?: number;
  offset?: number;
  kind?: string | null;
}

export interface JournalPage {
  entries: JournalObserverEntry[];
  total: number;
  limit: number;
  offset: number;
}

export async function fetchJournalPage(
  hudUrl: string,
  params?: JournalPageParams,
): Promise<JournalPage> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = new URLSearchParams();
  if (params?.limit != null) {
    query.set('limit', String(params.limit));
  }
  if (params?.offset != null) {
    query.set('offset', String(params.offset));
  }
  const kind = params?.kind?.trim();
  if (kind) {
    query.set('kind', kind);
  }
  const suffix = query.size > 0 ? `?${query.toString()}` : '';
  const res = await fetch(`${trimmed}/hud/journal${suffix}`);
  if (!res.ok) {
    throw new Error(`HUD journal page fetch failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as JournalPage;
}

// ────────────── RFC 20 Phase 8.2 — VRAM/RAM/cost monitor card ──────────────
//
// Mirrors the Rust `BusEventKind::HardwareSnapshot` payload published by
// `atlas monitor` (module `src-tauri/src/monitor/`). The card derives from
// the WS tail via `projectHardwareSnapshot` (last snapshot wins) so no new
// tail route is needed; thresholds mirror the Rust consts
// (`MONITOR_RAM_WARN_PRESSURE` 0.85 / crit 0.95, cost warn $5 / crit $20).

export type MonitorPressure = 'ok' | 'warn' | 'critical';

export interface HardwareSnapshotPayload {
  ram_total_mb: number;
  ram_used_mb: number;
  vram_total_mb: number | null;
  vram_used_mb: number | null;
  cost_usd: number;
}

export const MONITOR_RAM_WARN = 0.85;
export const MONITOR_RAM_CRIT = 0.95;
export const MONITOR_COST_WARN_USD = 5.0;
export const MONITOR_COST_CRIT_USD = 20.0;

export function pressureRatio(usedMb: number, totalMb: number): number {
  if (!Number.isFinite(usedMb) || !Number.isFinite(totalMb) || totalMb <= 0) return 0;
  return Math.min(1, Math.max(0, usedMb / totalMb));
}

export function classifyMonitorPressure(
  ratio: number,
  warn: number,
  crit: number,
): MonitorPressure {
  if (ratio >= crit) return 'critical';
  if (ratio >= warn) return 'warn';
  return 'ok';
}

/** Last `hardware_snapshot` in the WS tail, or `null` when none arrived yet. */
export function projectHardwareSnapshot(events: HudEvent[]): HardwareSnapshotPayload | null {
  let last: HardwareSnapshotPayload | null = null;
  for (const evt of events) {
    if (evt.kind !== 'hardware_snapshot') continue;
    const p = evt.payload as HardwareSnapshotPayload | null;
    if (p == null || typeof p.ram_total_mb !== 'number') continue;
    last = p;
  }
  return last;
}

export function monitorPressureOf(snap: HardwareSnapshotPayload): {
  ram: MonitorPressure;
  vram: MonitorPressure;
  cost: MonitorPressure;
} {
  const ram = classifyMonitorPressure(
    pressureRatio(snap.ram_used_mb, snap.ram_total_mb),
    MONITOR_RAM_WARN,
    MONITOR_RAM_CRIT,
  );
  const vram =
    snap.vram_total_mb != null && snap.vram_used_mb != null
      ? classifyMonitorPressure(
          pressureRatio(snap.vram_used_mb, snap.vram_total_mb),
          MONITOR_RAM_WARN,
          MONITOR_RAM_CRIT,
        )
      : 'ok';
  const cost = classifyMonitorPressure(snap.cost_usd, MONITOR_COST_WARN_USD, MONITOR_COST_CRIT_USD);
  return { ram, vram, cost };
}

// ═══════════ RFC 63 §7/§9 + RFC 65 §3 — AgentCard (capability layer) ═══════════
//
// The Rust agent loop emits `BusEventKind::AgentStep` per turn (tag
// `agent_step`); the store folds the WS tail into one run's step timeline so
// `<AgentCard>` renders steps / tool calls / evidence / tokens live. Mirrors
// `crate::core::bus::BusEventKind::AgentStep` (serde renames run_id, etc.).

export type AgentStepAction = 'run_command' | 'done' | 'invalid';
export type AgentStepVerdict = 'pass' | 'fail' | 'unknown';

export interface AgentStepPayload {
  run_id: string;
  step: number;
  action: string;
  observation: string | null;
  verdict: string | null;
  tokens_in: number;
  tokens_out: number;
  cost_usd: number;
}

export const AGENT_STEP_KIND = 'agent_step';

/** Last `agent_step` run in the WS tail, or `null` when none arrived yet. */
export function agentRunId(events: HudEvent[]): string | null {
  for (let i = events.length - 1; i >= 0; i--) {
    const evt = events[i];
    if (!evt || evt.kind !== AGENT_STEP_KIND) continue;
    const p = evt.payload as AgentStepPayload | null;
    if (p?.run_id) return p.run_id;
  }
  return null;
}

/** All steps for `runId`, in broadcast order (the live timeline). */
export function projectAgentSteps(events: HudEvent[], runId: string): AgentStepPayload[] {
  const out: AgentStepPayload[] = [];
  for (const evt of events) {
    if (evt.kind !== AGENT_STEP_KIND) continue;
    const p = evt.payload as AgentStepPayload | null;
    if (p?.run_id === runId && typeof p.step === 'number') {
      out.push(p);
    }
  }
  return out;
}

/** Colour token for a step's action/verdict pill. */
export function agentStepColor(action: string, verdict: string | null): string {
  if (verdict === 'fail') return 'red';
  if (verdict === 'pass') return 'green';
  if (verdict === 'unknown') return 'amber';
  switch (action) {
    case 'done':
      return 'green';
    case 'run_command':
      return 'blue';
    case 'invalid':
      return 'red';
    default:
      return 'grey';
  }
}

/** Aggregate tokens for a run's step list. */
export function agentRunTotals(steps: AgentStepPayload[]): {
  tokens_in: number;
  tokens_out: number;
  cost_usd: number;
} {
  return steps.reduce(
    (acc, s) => ({
      tokens_in: acc.tokens_in + (s.tokens_in ?? 0),
      tokens_out: acc.tokens_out + (s.tokens_out ?? 0),
      cost_usd: acc.cost_usd + (s.cost_usd ?? 0),
    }),
    { tokens_in: 0, tokens_out: 0, cost_usd: 0 },
  );
}

// ═══════════════ RFC 65 §3 — Kanban (missions as cards) ═══════════════
//
// Mirrors the Rust `Mission` row served by `GET /tail/missions`
// (id: Uuid → string, label, status). The board groups missions into the four
// operator columns; unknown statuses fall into `running` so nothing is hidden.

export type KanbanColumn = 'pending' | 'running' | 'done' | 'failed';

export interface MissionRow {
  id: string;
  label: string;
  status: string;
}

export const KANBAN_COLUMNS: readonly KanbanColumn[] = ['pending', 'running', 'done', 'failed'];

/** Map a free-form mission status to a board column (default: running). */
export function kanbanColumnOf(status: string): KanbanColumn {
  const s = status.toLowerCase();
  if (s.includes('done') || s.includes('complet') || s.includes('consolidat')) return 'done';
  if (s.includes('fail') || s.includes('error') || s.includes('abort')) return 'failed';
  if (s.includes('pend') || s.includes('new') || s.includes('queue')) return 'pending';
  return 'running';
}

export async function fetchMissions(hudUrl: string): Promise<MissionRow[]> {
  return fetchTail<MissionRow>(hudUrl, 'missions');
}

// ═══════════════ RFC 65 §3/§4 — Approvals queue (RFC 24 §6) ═══════════════
//
// Mirrors `BusEventKind::ApprovalRequest` (tag `approval_request`) and
// `ApprovalDecision` (tag `approval_decision`). The drawer folds the WS tail
// into the pending set: a request appears, its decision (or another device's)
// removes it. Answers POST to the HUD routes added in `hud/approvals.rs`.

export interface ApprovalRequestPayload {
  approval_id: string;
  agent_id: string;
  action: string;
}

export interface ApprovalDecisionPayload {
  approval_id: string;
  decision: string;
  user_id: string;
}

export const APPROVAL_REQUEST_KIND = 'approval_request';
export const APPROVAL_DECISION_KIND = 'approval_decision';

/** Pending approvals: requests not yet answered by any device. */
export function projectPendingApprovals(events: HudEvent[]): ApprovalRequestPayload[] {
  const pending = new Map<string, ApprovalRequestPayload>();
  for (const evt of events) {
    if (evt.kind === APPROVAL_REQUEST_KIND) {
      const p = evt.payload as ApprovalRequestPayload | null;
      if (p?.approval_id) pending.set(p.approval_id, p);
    } else if (evt.kind === APPROVAL_DECISION_KIND) {
      const d = evt.payload as ApprovalDecisionPayload | null;
      if (d?.approval_id) pending.delete(d.approval_id);
    }
  }
  return [...pending.values()];
}

export interface ApprovalAck {
  approval_id: string;
  decision: string;
  user_id: string;
}

async function postApprovalDecision(
  hudUrl: string | null,
  approvalId: string,
  decision: 'approve' | 'deny',
  userId: string,
  reason?: string,
): Promise<ApprovalAck> {
  const trimmed = (hudUrl ?? '').replace(/\/$/, '');
  if (!trimmed) throw new Error('HUD URL unavailable — could not answer approval');
  const body: Record<string, unknown> = { user_id: userId };
  if (reason) body.reason = reason;
  const res = await fetch(
    `${trimmed}/hud/approvals/${encodeURIComponent(approvalId)}/${decision}`,
    {
      method: 'POST',
      headers: { 'content-type': 'application/json' },
      body: JSON.stringify(body),
    },
  );
  if (!res.ok) {
    throw new Error(`HUD approval ${decision} failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as ApprovalAck;
}

export function approveApproval(
  hudUrl: string | null,
  approvalId: string,
  userId = 'operator',
): Promise<ApprovalAck> {
  return postApprovalDecision(hudUrl, approvalId, 'approve', userId);
}

export function denyApproval(
  hudUrl: string | null,
  approvalId: string,
  reason?: string,
  userId = 'operator',
): Promise<ApprovalAck> {
  return postApprovalDecision(hudUrl, approvalId, 'deny', userId, reason);
}

// ═══════════════ RFC 65 §3 — Cost & Res (CostDashboard) ═══════════════
//
// Mirrors `hud/cost.rs` (`GET /hud/cost`): window totals + per-model roll-up
// from `model_invocations`, cumulative spend with its pressure level, and the
// pending `model_resets` windows (RFC 28 §H).

export interface CostByModel {
  model_id: string;
  provider: string;
  invocations: number;
  cost_usd: number;
  tokens_in: number;
  tokens_out: number;
  mean_latency_ms: number;
}

export interface CostTotals {
  invocations: number;
  cost_usd: number;
  tokens_in: number;
  tokens_out: number;
  mean_latency_ms: number;
}

export interface PendingReset {
  provider: string;
  model: string;
  status_code: number;
  error_type: string | null;
  resets_at_ms: number;
}

export interface CostResponse {
  window: number;
  totals: CostTotals;
  by_model: CostByModel[];
  cumulative_usd: number;
  pressure: { level: MonitorPressure; warn_usd: number; crit_usd: number };
  pending_resets: PendingReset[];
}

export async function fetchCost(hudUrl: string, window?: number): Promise<CostResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = window ? `?window=${encodeURIComponent(window)}` : '';
  const res = await fetch(`${trimmed}/hud/cost${query}`);
  if (!res.ok) {
    throw new Error(`HUD cost failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as CostResponse;
}

/** Compact USD formatting for the cost table (`$0.0000`). */
export function formatUsd(n: number): string {
  return `$${(Number.isFinite(n) ? n : 0).toFixed(4)}`;
}

// ═══════════════ RFC 65 §3 — Health KPIs (HealthKPIs) ═══════════════
//
// Mirrors `hud/health.rs` (`GET /hud/health`): agent-session telemetry,
// capability-layer run states and live swarm-registry states.

export interface AgentEventTypeCount {
  event_type: string;
  count: number;
}

export interface AgentSessionEvent {
  id: number;
  ts: number;
  pane_id: string | null;
  event_type: string;
  agent: string;
  task_id: string | null;
}

export interface StatusCount {
  status: string;
  count: number;
}

export interface StateCount {
  state: string;
  count: number;
}

export interface HealthResponse {
  agent_events: { total: number; by_type: AgentEventTypeCount[]; recent: AgentSessionEvent[] };
  agent_runs: StatusCount[];
  swarm_agents: StateCount[];
}

export async function fetchHealth(hudUrl: string, last?: number): Promise<HealthResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = last ? `?last=${encodeURIComponent(last)}` : '';
  const res = await fetch(`${trimmed}/hud/health${query}`);
  if (!res.ok) {
    throw new Error(`HUD health failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as HealthResponse;
}

export const AGENT_HEARTBEAT_KIND = 'agent_heartbeat';

/** Latest supervisor heartbeat timestamp from the WS tail (null if none). */
export function projectLatestHeartbeat(events: HudEvent[]): string | null {
  for (let i = events.length - 1; i >= 0; i -= 1) {
    if (events[i]?.kind === AGENT_HEARTBEAT_KIND) return events[i]?.ts ?? null;
  }
  return null;
}

// ═══════════════ RFC 65 §3 — Audit timeline (AuditTimeline) ═══════════════
//
// Mirrors `hud/audit.rs` (`GET /hud/audit`): the append-only `audit_log` chain,
// newest first. Hash-chain verification (RFC 24 §10) is not implemented yet.

export interface AuditEntry {
  seq: number;
  ts: string;
  actor: string;
  action: string;
  inputs: unknown;
  outputs: unknown;
}

export interface AuditResponse {
  rows: AuditEntry[];
  count: number;
}

export async function fetchAudit(hudUrl: string, last?: number): Promise<AuditResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = last ? `?last=${encodeURIComponent(last)}` : '';
  const res = await fetch(`${trimmed}/hud/audit${query}`);
  if (!res.ok) {
    throw new Error(`HUD audit failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as AuditResponse;
}

// ═══════════════ RFC 65 §3 — payload drill-down (Outline) ═══════════════
//
// `GET /payload/:kind/:id` returns the raw artefact payload as a JSON body.
// Used by OutlineView to read a plan's `roadmap` milestones.

export interface PlanMilestone {
  id: string;
  label: string;
  objectives?: string[];
  depends_on?: string[];
}

export interface PlanPayload {
  id?: string;
  mission_id?: string;
  roadmap?: PlanMilestone[];
}

export async function fetchPayload(hudUrl: string, kind: string, id: string): Promise<unknown> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(
    `${trimmed}/payload/${encodeURIComponent(kind)}/${encodeURIComponent(id)}`,
  );
  if (!res.ok) {
    throw new Error(`HUD payload ${kind} failed: ${res.status} ${res.statusText}`);
  }
  const text = await res.text();
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return text;
  }
}

// ═══════════════ RFC 65 §3 — Worktrees (WorktreesView) ═══════════════
//
// Mirrors `hud/worktrees.rs` (`GET /hud/worktrees`): the git worktrees of a
// repo, or `ok:false` + reason when git is missing / the path is not a repo.

export interface WorktreeEntry {
  path: string;
  branch: string | null;
  detached: boolean;
}

export interface WorktreesResponse {
  repo: string;
  ok: boolean;
  reason: string | null;
  entries: WorktreeEntry[];
}

export async function fetchWorktrees(hudUrl: string, repo?: string): Promise<WorktreesResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = repo ? `?repo=${encodeURIComponent(repo)}` : '';
  const res = await fetch(`${trimmed}/hud/worktrees${query}`);
  if (!res.ok) {
    throw new Error(`HUD worktrees failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as WorktreesResponse;
}

// ═══════════════ RFC 65 §9 — Demos over diffs (DemoPane) ═══════════════
//
// Mirrors `hud/demos.rs` (`GET /hud/demos`): the artefacts produced by agent
// runs. The full Loom-style TTS video (RFC 24 §9) is not implemented; this is
// the persisted subset (screenshots / files / preview URLs).

export interface DemoArtifact {
  id: string;
  run_id: string;
  kind: string;
  path: string | null;
  sha256: string | null;
  verified: boolean;
  preview_url: string | null;
}

export interface DemosResponse {
  artifacts: DemoArtifact[];
  count: number;
}

export async function fetchDemos(hudUrl: string, last?: number): Promise<DemosResponse> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = last ? `?last=${encodeURIComponent(last)}` : '';
  const res = await fetch(`${trimmed}/hud/demos${query}`);
  if (!res.ok) {
    throw new Error(`HUD demos failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as DemosResponse;
}

// ═══════════════ RFC 65 §10 — Skill + MCP rail ═══════════════
//
// Skills from `/tail/skills` (SkillRow); MCP servers from `GET /hud/mcp`
// (`.opencode/mcp.json`). Atlas's own hot-swap MCP runtime (RFC 07 / RFC 24 §8)
// is not implemented — this is the read side of the catalog.

export interface SkillCatalogRow {
  skill_id: string;
  version: string;
  engine: string;
  priority: number;
  domain: string | null;
  language: string | null;
  framework: string | null;
  confidence: number;
  auto_generated: boolean;
  verified: boolean;
  requires_sandbox: boolean;
  generated_at: string;
}

export interface McpServer {
  name: string;
  type: string | null;
  enabled: boolean;
  command: unknown;
}

export interface McpCatalog {
  repo: string;
  ok: boolean;
  reason: string | null;
  path?: string;
  servers: McpServer[];
}

export async function fetchMcp(hudUrl: string, repo?: string): Promise<McpCatalog> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const query = repo ? `?repo=${encodeURIComponent(repo)}` : '';
  const res = await fetch(`${trimmed}/hud/mcp${query}`);
  if (!res.ok) {
    throw new Error(`HUD mcp failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as McpCatalog;
}

// ═══════════════ RFC 65 §11 / RFC 24 §16 — remote access status ═══════════════

export interface RemoteAccessStatus {
  local_only: boolean;
  token_configured: boolean;
  oidc_configured: boolean;
  oidc_issuer: string | null;
  hud_port: number | null;
  latency_target_ms: number;
}

export async function fetchRemoteStatus(hudUrl: string): Promise<RemoteAccessStatus> {
  const trimmed = hudUrl.replace(/\/$/, '');
  const res = await fetch(`${trimmed}/remote/status`);
  if (!res.ok) {
    throw new Error(`HUD remote status failed: ${res.status} ${res.statusText}`);
  }
  return (await res.json()) as RemoteAccessStatus;
}

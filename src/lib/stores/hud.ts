// OpenCode OS — HUD Svelte store (RFC 24 §4).
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
  | 'step_states';

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
      const evt = JSON.parse(msg.data as string) as {
        id: string;
        ts: string;
        kind: string;
        payload: unknown;
      } & { type?: string };
      const item: HudEvent = {
        id: evt.id ?? crypto.randomUUID(),
        ts: evt.ts ?? new Date().toISOString(),
        kind: evt.kind ?? evt.type ?? 'unknown',
        payload: evt.payload,
      };
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

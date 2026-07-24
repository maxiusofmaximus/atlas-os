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

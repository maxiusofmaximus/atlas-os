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
 * - Strips the trailing slash so appending `/ws` always lands.
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

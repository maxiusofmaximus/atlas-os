// Atlas OS — HUD view routing (RFC 65 §5). A single active view id plus the
// catalog the ViewSwitcher renders. Kept in a store (not router) because the
// HUD is a CSR app that swaps panels in place — the Kernel Bus stream must not
// remount on navigation.
//
// RFC 67 §1.11 — the catalog is the 10-id model: 8 projections (kanban, canvas,
// outline, timeline, cost, health, audit, worktrees) + 2 panels (settings, mcp).
// `overview`, `agent` and `approvals` are dissolved: their content lives in the
// Mission Rail, the Agent Card (inside kanban/canvas) and the Approvals Dock.
// The per-view single-letter `key` bindings are gone (they collided with the
// RFC 24 §19 hotkey table); hotkeys are the leader-key actions below.

import { writable, type Readable } from 'svelte/store';

export type ViewId =
  | 'kanban'
  | 'cost'
  | 'health'
  | 'audit'
  | 'canvas'
  | 'outline'
  | 'timeline'
  | 'worktrees'
  | 'settings'
  | 'mcp';

export interface ViewDef {
  id: ViewId;
  label: string;
  /** Fase that ships it (RFC 65 §7) — shown as a hint when unimplemented. */
  fase: 'P0' | 'P1' | 'P2';
}

export const VIEWS: readonly ViewDef[] = [
  { id: 'kanban', label: 'Kanban', fase: 'P0' },
  { id: 'cost', label: 'Cost & Res', fase: 'P1' },
  { id: 'health', label: 'Health KPIs', fase: 'P1' },
  { id: 'audit', label: 'Audit', fase: 'P1' },
  { id: 'canvas', label: 'Canvas', fase: 'P2' },
  { id: 'outline', label: 'Outline', fase: 'P2' },
  { id: 'timeline', label: 'Timeline', fase: 'P2' },
  { id: 'worktrees', label: 'Worktrees', fase: 'P2' },
  { id: 'settings', label: 'Settings', fase: 'P1' },
  { id: 'mcp', label: 'MCP', fase: 'P1' },
];

const initial: ViewId = 'kanban';
const { subscribe, set, update } = writable<ViewId>(initial);

export const activeView: Readable<ViewId> & {
  set: (v: ViewId) => void;
  cycle: (dir?: 1 | -1) => void;
} = {
  subscribe,
  set,
  /** Move to the next/previous view in the catalog. */
  cycle(dir: 1 | -1 = 1) {
    update((current) => {
      const ids = VIEWS.map((v) => v.id);
      const i = ids.indexOf(current);
      const next = (i + dir + ids.length) % ids.length;
      return ids[next] ?? current;
    });
  },
};

const mission = writable<string | null>(null);

/** The mission selected in the Mission Rail. Selecting a mission is pure
 *  in-place state and never reconnects the Kernel Bus stream (RFC 67 §2). */
export const activeMissionId: Readable<string | null> & {
  set: (id: string | null) => void;
} = {
  subscribe: mission.subscribe,
  set: mission.set,
};

/** RFC 24 §19 — leader-key actions (pressed after `:`). */
export type HotkeyAction = 'view' | 'approvals' | 'new-mission' | 'mission' | 'demo' | 'help';

export const HOTKEYS: Readonly<Record<string, HotkeyAction>> = {
  v: 'view',
  a: 'approvals',
  n: 'new-mission',
  m: 'mission',
  d: 'demo',
  '?': 'help',
};

/** Resolve a leader key to its action, or null. */
export function hotkeyAction(key: string): HotkeyAction | null {
  return HOTKEYS[key.toLowerCase()] ?? null;
}

export type MissionSort = 'recent' | 'frecency';

const SORT_KEY = 'atlas.missionSort';

function readStoredSort(): MissionSort {
  try {
    return localStorage.getItem(SORT_KEY) === 'frecency' ? 'frecency' : 'recent';
  } catch {
    return 'recent';
  }
}

function writeStoredSort(v: MissionSort): void {
  try {
    localStorage.setItem(SORT_KEY, v);
  } catch {
    /* ignore */
  }
}

const sortStore = writable<MissionSort>(readStoredSort());

/** Mission Rail ordering (RFC 67 §20 H-01). UI-only state, persisted to
 *  localStorage — never a backend setting. */
export const missionSort: Readable<MissionSort> & {
  set: (v: MissionSort) => void;
  toggle: () => void;
} = {
  subscribe: sortStore.subscribe,
  set(v: MissionSort) {
    writeStoredSort(v);
    sortStore.set(v);
  },
  toggle() {
    sortStore.update((current) => {
      const next: MissionSort = current === 'recent' ? 'frecency' : 'recent';
      writeStoredSort(next);
      return next;
    });
  },
};

/** The `+ New` / `:n` inline mission form visibility (RFC 67 §24.5). */
export const newMissionOpen = writable<boolean>(false);

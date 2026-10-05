// Atlas OS — HUD view routing (RFC 65 §5). A single active view id plus the
// catalog the ViewSwitcher renders. Kept in a store (not router) because the
// HUD is a CSR app that swaps panels in place — the Kernel Bus stream must not
// remount on navigation.
//
// Views arrive in fases (RFC 65 §7): P0 ships overview/+agent/+kanban/
// +approvals; later fases append cost/health/audit/canvas. `VIEWS` is the one
// source of truth so the switcher, the palette and hotkeys never drift.

import { writable, type Readable } from 'svelte/store';

export type ViewId =
  | 'overview'
  | 'agent'
  | 'kanban'
  | 'approvals'
  | 'cost'
  | 'health'
  | 'audit'
  | 'canvas'
  | 'outline'
  | 'timeline'
  | 'worktrees';

export interface ViewDef {
  id: ViewId;
  label: string;
  /** Single-letter hotkey after the `:` prefix (RFC 24 hotkey table). */
  key: string;
  /** Fase that ships it (RFC 65 §7) — shown as a hint when unimplemented. */
  fase: 'P0' | 'P1' | 'P2';
}

export const VIEWS: readonly ViewDef[] = [
  { id: 'overview', label: 'Overview', key: 'o', fase: 'P0' },
  { id: 'agent', label: 'Agent', key: 'a', fase: 'P0' },
  { id: 'kanban', label: 'Kanban', key: 'k', fase: 'P0' },
  { id: 'approvals', label: 'Approvals', key: 'p', fase: 'P0' },
  { id: 'cost', label: 'Cost & Res', key: 'c', fase: 'P1' },
  { id: 'health', label: 'Health KPIs', key: 'h', fase: 'P1' },
  { id: 'audit', label: 'Audit', key: 'u', fase: 'P1' },
  { id: 'canvas', label: 'Canvas', key: 'g', fase: 'P2' },
  { id: 'outline', label: 'Outline', key: 'l', fase: 'P2' },
  { id: 'timeline', label: 'Timeline', key: 't', fase: 'P2' },
  { id: 'worktrees', label: 'Worktrees', key: 'w', fase: 'P2' },
];

const initial: ViewId = 'overview';
const { subscribe, set, update } = writable<ViewId>(initial);

export const activeView: Readable<ViewId> & {
  set: (v: ViewId) => void;
  cycle: (dir?: 1 | -1) => void;
} = {
  subscribe,
  set,
  /** Move to the next/previous implemented view (P0+P1+P2). Every catalog
   *  view is implemented, so the cycle covers them all. */
  cycle(dir: 1 | -1 = 1) {
    update((current) => {
      const done = VIEWS.filter(
        (v) => v.fase === 'P0' || v.fase === 'P1' || v.fase === 'P2',
      ).map((v) => v.id);
      const i = done.indexOf(current);
      const next = (i + dir + done.length) % done.length;
      return done[next] ?? current;
    });
  },
};

/** Resolve a hotkey letter to a view id, or null. */
export function viewForKey(key: string): ViewId | null {
  const k = key.toLowerCase();
  return VIEWS.find((v) => v.key === k)?.id ?? null;
}

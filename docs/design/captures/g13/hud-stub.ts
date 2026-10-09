import { writable } from 'svelte/store';

export interface HudEvent {
  id: string;
  ts: string;
  kind: string;
  payload: unknown;
}

export const hud = writable({
  connected: true,
  url: 'http://hud',
  events: [
    {
      id: 'e1',
      ts: '2026-10-06T00:00:00Z',
      kind: 'approval_request',
      payload: {
        approval_id: 'a1',
        agent_id: 'builder-1234567890abcdef',
        action: 'network_binding',
      },
    },
    {
      id: 'e2',
      ts: '2026-10-06T00:00:01Z',
      kind: 'approval_request',
      payload: { approval_id: 'a2', agent_id: 'builder-abcdef1234567890', action: 'read_file' },
    },
  ] as HudEvent[],
});

export function projectPendingApprovals(events: HudEvent[]): Array<Record<string, unknown>> {
  const pending = new Map<string, Record<string, unknown>>();
  for (const e of events) {
    const p = e.payload as Record<string, unknown> | null;
    if (!p) continue;
    if (e.kind === 'approval_request' && typeof p.approval_id === 'string') {
      pending.set(p.approval_id, p);
    } else if (e.kind === 'approval_decision' && typeof p.approval_id === 'string') {
      pending.delete(p.approval_id);
    }
  }
  return [...pending.values()];
}

export async function approveApproval(): Promise<Record<string, unknown>> {
  return { ok: true };
}

export async function denyApproval(): Promise<Record<string, unknown>> {
  return { ok: true };
}

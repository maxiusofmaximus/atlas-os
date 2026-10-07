// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { HudEvent, HudState } from '$stores/hud';

const spies = vi.hoisted(() => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
}));

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  const { writable } = await import('svelte/store');
  const store = writable<HudState>({ connected: true, url: null, events: [] });
  return {
    ...actual,
    hud: Object.assign(store, { connect: spies.connect, disconnect: spies.disconnect }),
  };
});

import { hud } from '$stores/hud';
import ApprovalsDock from './ApprovalsDock.svelte';

type Writable = { set: (value: HudState) => void };

const setHud = (patch: Partial<HudState>): void =>
  (hud as unknown as Writable).set({ connected: true, url: null, events: [], ...patch });

let seq = 0;
const approval = (id: string, agent: string, action: string, files?: string[]): HudEvent => ({
  id: `e${seq++}`,
  ts: '2026-10-06T00:00:00Z',
  kind: 'approval_request',
  payload: files
    ? { approval_id: id, agent_id: agent, action, files }
    : { approval_id: id, agent_id: agent, action },
});

function render(props: ComponentProps<typeof ApprovalsDock> = {}) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(ApprovalsDock, { target, props });
  return { target, component };
}

const okResponse = {
  ok: true,
  status: 200,
  statusText: 'OK',
  json: async () => ({ approval_id: 'x', decision: 'approve', user_id: 'operator' }),
} as unknown as Response;
const conflictResponse = { ok: false, status: 409, statusText: 'Conflict' } as unknown as Response;
const errorResponse = { ok: false, status: 500, statusText: 'Server Error' } as unknown as Response;

const fetchMock = vi.fn();

async function selectAll(target: HTMLElement): Promise<void> {
  target.querySelectorAll<HTMLButtonElement>('.row-btn').forEach((b) => b.click());
  await vi.waitFor(() =>
    expect(target.querySelectorAll('.row-btn[aria-pressed="true"]').length).toBeGreaterThan(0),
  );
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
  setHud({});
  vi.stubGlobal('fetch', fetchMock);
});

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('ApprovalsDock behaviour', () => {
  it('renders the gate and question channels from the WS tail', async () => {
    setHud({
      events: [
        approval('a1', 'agent-1', 'network_binding'),
        approval('q1', 'agent-2', 'read_file'),
      ],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    const gate = target.querySelector('.channel.gate');
    const question = target.querySelector('.channel.question');
    expect(gate).not.toBeNull();
    expect(question).not.toBeNull();
    expect(gate?.getAttribute('aria-live')).toBe('assertive');
    expect(question?.getAttribute('aria-live')).toBe('polite');
    await unmount(component);
  });

  it('renders the empty state when nothing is pending', async () => {
    setHud({ events: [] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    expect(target.querySelector('.empty')?.textContent).toContain('Sin aprobaciones pendientes');
    await unmount(component);
  });

  it('single Apr posts to the per-id approve route and calls onResolved', async () => {
    fetchMock.mockResolvedValue(okResponse);
    setHud({ events: [approval('a1', 'agent-1', 'network_binding')] });
    const onResolved = vi.fn();
    const { target, component } = render({ hudUrl: 'http://hud', onResolved });

    target.querySelector<HTMLButtonElement>('.apr')?.click();
    await vi.waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        'http://hud/hud/approvals/a1/approve',
        expect.objectContaining({ method: 'POST' }),
      ),
    );
    await vi.waitFor(() => expect(onResolved).toHaveBeenCalledWith('a1', 'approve'));
    await unmount(component);
  });

  it('single Deny posts to the per-id deny route', async () => {
    fetchMock.mockResolvedValue(okResponse);
    setHud({ events: [approval('a2', 'agent-2', 'network_binding')] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    target.querySelector<HTMLButtonElement>('.deny')?.click();
    await vi.waitFor(() =>
      expect(fetchMock).toHaveBeenCalledWith(
        'http://hud/hud/approvals/a2/deny',
        expect.objectContaining({ method: 'POST' }),
      ),
    );
    await unmount(component);
  });

  it('shows an error banner when a single decision fails', async () => {
    fetchMock.mockResolvedValue(errorResponse);
    setHud({ events: [approval('a3', 'agent-3', 'network_binding')] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    target.querySelector<HTMLButtonElement>('.apr')?.click();
    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.channel.gate')).not.toBeNull();
    await unmount(component);
  });

  it('batch approve posts the selected ids to /hud/approvals/batch and clears selection', async () => {
    fetchMock.mockResolvedValue(okResponse);
    setHud({
      events: [approval('a1', 'agent-1', 'network_binding'), approval('a2', 'agent-2', 'exec')],
    });
    const onResolved = vi.fn();
    const { target, component } = render({ hudUrl: 'http://hud', onResolved });

    await selectAll(target);
    await vi.waitFor(() =>
      expect(target.querySelector<HTMLButtonElement>('.approve-all')?.disabled).toBe(false),
    );
    target.querySelector<HTMLButtonElement>('.approve-all')?.click();

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(url).toBe('http://hud/hud/approvals/batch');
    const body = JSON.parse((init as RequestInit).body as string);
    expect(body.decision).toBe('approve');
    expect(body.items.map((i: { approval_id: string }) => i.approval_id).sort()).toEqual([
      'a1',
      'a2',
    ]);
    await vi.waitFor(() => expect(onResolved).toHaveBeenCalledTimes(2));
    expect(target.querySelectorAll('.row-btn[aria-pressed="true"]').length).toBe(0);
    await unmount(component);
  });

  it('batch includes the optional reason in the payload', async () => {
    fetchMock.mockResolvedValue(okResponse);
    setHud({
      events: [approval('a1', 'agent-1', 'network_binding'), approval('a2', 'agent-2', 'exec')],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    await selectAll(target);
    await vi.waitFor(() =>
      expect(target.querySelector<HTMLButtonElement>('.approve-all')?.disabled).toBe(false),
    );
    const input = target.querySelector<HTMLInputElement>('.reason input');
    if (!input) throw new Error('reason input not found');
    input.value = 'unsafe change';
    input.dispatchEvent(new Event('input', { bubbles: true }));
    target.querySelector<HTMLButtonElement>('.approve-all')?.click();

    await vi.waitFor(() => expect(fetchMock).toHaveBeenCalled());
    const body = JSON.parse((fetchMock.mock.calls[0]?.[1] as RequestInit).body as string);
    expect(body.reason).toBe('unsafe change');
    await unmount(component);
  });

  it('409 shows the conflict copy and keeps the selection', async () => {
    fetchMock.mockResolvedValue(conflictResponse);
    setHud({
      events: [approval('a1', 'agent-1', 'network_binding'), approval('a2', 'agent-2', 'exec')],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    await selectAll(target);
    await vi.waitFor(() =>
      expect(target.querySelector<HTMLButtonElement>('.approve-all')?.disabled).toBe(false),
    );
    target.querySelector<HTMLButtonElement>('.approve-all')?.click();

    await vi.waitFor(() => expect(target.querySelector('.batch-error')).not.toBeNull());
    expect(target.querySelector('.batch-error')?.textContent).toContain('Conflicto');
    expect(target.querySelectorAll('.row-btn[aria-pressed="true"]').length).toBe(2);
    await unmount(component);
  });

  it('partial: a file conflict disables the batch and shows n de m', async () => {
    setHud({
      events: [
        approval('a1', 'agent-1', 'network_binding', ['src/shared.rs']),
        approval('a2', 'agent-2', 'exec', ['src/shared.rs']),
        approval('a3', 'agent-3', 'read_file', ['src/other.rs']),
      ],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    await selectAll(target);
    await vi.waitFor(() => expect(target.querySelector('.partial')).not.toBeNull());
    const partial = target.querySelector('.partial');
    expect(partial?.textContent).toContain('1 de 3');
    expect(partial?.textContent).toContain('2 en conflicto');
    expect(partial?.textContent).toContain('src/shared.rs');
    expect(target.querySelector<HTMLButtonElement>('.approve-all')?.disabled).toBe(true);
    await unmount(component);
  });
});

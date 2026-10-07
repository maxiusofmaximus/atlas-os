// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { HudEvent, HudState } from '$stores/hud';

const spies = vi.hoisted(() => ({
  approve: vi.fn(),
  deny: vi.fn(),
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
    approveApproval: spies.approve,
    denyApproval: spies.deny,
  };
});

import { hud } from '$stores/hud';
import ApprovalsDock from './ApprovalsDock.svelte';

type Writable = { set: (value: HudState) => void };

const setHud = (patch: Partial<HudState>): void =>
  (hud as unknown as Writable).set({ connected: true, url: null, events: [], ...patch });

let seq = 0;
const approval = (id: string, agent: string, action: string): HudEvent => ({
  id: `e${seq++}`,
  ts: '2026-10-06T00:00:00Z',
  kind: 'approval_request',
  payload: { approval_id: id, agent_id: agent, action },
});

function render(props: ComponentProps<typeof ApprovalsDock> = {}) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(ApprovalsDock, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
  setHud({});
});

describe('ApprovalsDock behaviour', () => {
  it('routes a sensitive action to the gate channel and Apr posts approve + onResolved', async () => {
    spies.approve.mockResolvedValue({
      approval_id: 'a1',
      decision: 'approve',
      user_id: 'operator',
    });
    setHud({ events: [approval('a1', 'agent-1', 'network_binding')] });
    const onResolved = vi.fn();
    const { target, component } = render({ hudUrl: 'http://hud', onResolved });

    const gate = target.querySelector('.channel.gate');
    expect(gate).not.toBeNull();
    expect(gate?.getAttribute('aria-live')).toBe('assertive');
    expect(gate?.textContent).toContain('network_binding');

    gate?.querySelector<HTMLButtonElement>('.apr')?.click();
    await vi.waitFor(() => expect(spies.approve).toHaveBeenCalledWith('http://hud', 'a1'));
    await vi.waitFor(() => expect(onResolved).toHaveBeenCalledWith('a1', 'approve'));
    await unmount(component);
  });

  it('routes a non-sensitive action to the async question channel', async () => {
    setHud({ events: [approval('q1', 'agent-2', 'read_file')] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    const question = target.querySelector('.channel.question');
    expect(question).not.toBeNull();
    expect(question?.getAttribute('aria-live')).toBe('polite');
    await unmount(component);
  });

  it('Deny posts the deny route', async () => {
    spies.deny.mockResolvedValue({ approval_id: 'a3', decision: 'deny', user_id: 'operator' });
    setHud({ events: [approval('a3', 'agent-4', 'network_binding')] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    target.querySelector<HTMLButtonElement>('.deny')?.click();
    await vi.waitFor(() => expect(spies.deny).toHaveBeenCalledWith('http://hud', 'a3'));
    await unmount(component);
  });

  it('renders the empty state when nothing is pending', async () => {
    setHud({ events: [] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    expect(target.querySelector('.empty')?.textContent).toContain('No pending approvals.');
    await unmount(component);
  });

  it('keeps the row and shows the error when the POST fails', async () => {
    spies.approve.mockRejectedValueOnce(new Error('approve failed'));
    setHud({ events: [approval('a2', 'agent-3', 'network_binding')] });
    const { target, component } = render({ hudUrl: 'http://hud' });

    target.querySelector<HTMLButtonElement>('.apr')?.click();
    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.error')?.textContent).toContain('approve failed');
    expect(target.querySelector('.channel.gate')).not.toBeNull();
    await unmount(component);
  });
});

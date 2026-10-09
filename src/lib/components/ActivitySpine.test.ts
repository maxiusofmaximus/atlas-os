// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { HudEvent, HudState } from '$stores/hud';

const spies = vi.hoisted(() => ({
  connect: vi.fn(),
  disconnect: vi.fn(),
}));

vi.mock('$stores/hud', async () => {
  const { writable } = await import('svelte/store');
  const store = writable<HudState>({ connected: false, url: null, events: [] });
  return {
    hud: Object.assign(store, {
      connect: spies.connect,
      disconnect: spies.disconnect,
    }),
  };
});

import { hud } from '$stores/hud';
import ActivitySpine from './ActivitySpine.svelte';

type Writable = { set: (value: HudState) => void };

const setHud = (patch: Partial<HudState>): void =>
  (hud as unknown as Writable).set({ connected: false, url: null, events: [], ...patch });

let seq = 0;
const evt = (kind: string, payload: unknown): HudEvent => ({
  id: `e${seq++}`,
  ts: '2026-10-06T00:00:00Z',
  kind,
  payload,
});

function render(props: ComponentProps<typeof ActivitySpine> = {}) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(ActivitySpine, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
  setHud({});
});

describe('ActivitySpine behaviour', () => {
  it('renders a doom_loop_detected event with the err tone', async () => {
    setHud({
      connected: true,
      events: [evt('doom_loop_detected', { agent_id: 'agent-1234', count: 3 })],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    const row = target.querySelector('.row-btn.tone-err');
    expect(row).not.toBeNull();
    expect(row?.textContent).toContain('doom loop detected');
    await unmount(component);
  });

  it('renders the empty state when there are no events', async () => {
    setHud({ connected: true, events: [] });
    const { target, component } = render();

    expect(target.querySelector('.empty')?.textContent).toContain('Sin actividad todavía');
    await unmount(component);
  });

  it('keeps the last events and Retry reconnects when the stream is down', async () => {
    setHud({
      connected: false,
      events: [evt('agent_step', { run_id: 'r1', step: 1, action: 'run_command' })],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    expect(target.querySelector('.disconnected')).not.toBeNull();
    expect(target.querySelector('.row-btn')).not.toBeNull();

    target.querySelector<HTMLButtonElement>('.retry')?.click();
    expect(spies.connect).toHaveBeenCalledWith('http://hud');
    await unmount(component);
  });

  it('invokes onSelect with the event when a row is clicked', async () => {
    setHud({
      connected: true,
      events: [evt('agent_step', { run_id: 'r1', step: 1, action: 'run_command' })],
    });
    const onSelect = vi.fn();
    const { target, component } = render({ hudUrl: 'http://hud', onSelect });

    target.querySelector<HTMLButtonElement>('.row-btn')?.click();
    expect(onSelect).toHaveBeenCalledTimes(1);
    expect(onSelect.mock.calls[0]?.[0]).toMatchObject({ kind: 'agent_step' });
    await unmount(component);
  });

  it('shows the full agent id (not the old truncation) with the id as a tooltip', async () => {
    const fullId = 'builder-1234567890abcdef';
    setHud({
      connected: true,
      events: [evt('agent_status_changed', { agent_id: fullId, status: 'coding' })],
    });
    const { target, component } = render({ hudUrl: 'http://hud' });

    const actor = target.querySelector<HTMLElement>('.actor');
    expect(actor).not.toBeNull();
    expect(actor?.textContent).toBe(fullId);
    expect(actor?.textContent).not.toBe('builder-');
    expect(actor?.getAttribute('title')).toBe(fullId);
    await unmount(component);
  });
});

// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { HealthResponse, HudState } from '$stores/hud';

const spies = vi.hoisted(() => ({
  fetchHealth: vi.fn(),
  connect: vi.fn(),
  disconnect: vi.fn(),
}));

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  const { writable } = await import('svelte/store');
  const store = writable<HudState>({ connected: false, url: null, events: [] });
  return {
    ...actual,
    hud: Object.assign(store, { connect: spies.connect, disconnect: spies.disconnect }),
    fetchHealth: spies.fetchHealth,
  };
});

import HealthKPIs from './HealthKPIs.svelte';

const healthData: HealthResponse = {
  agent_events: {
    total: 5,
    by_type: [{ event_type: 'agent_step', count: 3 }],
    recent: [
      {
        id: 1,
        ts: 1700000000,
        pane_id: 'p1',
        event_type: 'agent_step',
        agent: 'a1',
        task_id: null,
      },
    ],
  },
  agent_runs: [{ status: 'success', count: 2 }],
  swarm_agents: [{ state: 'working', count: 1 }],
};

function render(props: ComponentProps<typeof HealthKPIs>) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(HealthKPIs, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
});

describe('HealthKPIs behaviour', () => {
  it('renders the KPI totals and breakdowns', async () => {
    spies.fetchHealth.mockResolvedValue(healthData);
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelectorAll('.kpi-value').length).toBe(3));
    expect(target.querySelectorAll('.kpi-value')[0]?.textContent).toBe('5');
    expect(target.querySelectorAll('.kpi-value')[1]?.textContent).toBe('2');
    expect(target.querySelectorAll('.kpi-value')[2]?.textContent).toBe('1');
    expect(target.querySelector('.bars')?.textContent).toContain('agent_step');
    await unmount(component);
  });

  it('renders the empty state when there is no HUD URL', async () => {
    const { target, component } = render({ hudUrl: null });

    expect(target.querySelector('.empty')?.textContent).toContain('No health data yet.');
    await unmount(component);
  });

  it('renders the loading skeleton while the fetch is in flight', async () => {
    spies.fetchHealth.mockReturnValue(new Promise(() => {}));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.skeleton')).not.toBeNull());
    await unmount(component);
  });

  it('renders the error state and Retry re-fetches', async () => {
    spies.fetchHealth.mockRejectedValueOnce(new Error('health down'));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.error')?.textContent).toContain('health down');

    spies.fetchHealth.mockResolvedValue(healthData);
    target.querySelector<HTMLButtonElement>('.error .retry')?.click();
    await vi.waitFor(() => expect(target.querySelector('.kpi-value')).not.toBeNull());
    expect(target.querySelector('.error')).toBeNull();
    expect(spies.fetchHealth).toHaveBeenCalledTimes(2);
    await unmount(component);
  });
});

// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { CostResponse } from '$stores/hud';

const spies = vi.hoisted(() => ({ fetchCost: vi.fn() }));

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchCost: spies.fetchCost };
});

import CostDashboard from './CostDashboard.svelte';

const costData: CostResponse = {
  window: 200,
  totals: {
    invocations: 3,
    cost_usd: 1.2345,
    tokens_in: 100,
    tokens_out: 50,
    mean_latency_ms: 42.4,
  },
  by_model: [
    {
      model_id: 'gpt-x',
      provider: 'openai',
      invocations: 2,
      cost_usd: 1.0,
      tokens_in: 80,
      tokens_out: 40,
      mean_latency_ms: 40,
    },
  ],
  cumulative_usd: 9.5,
  pressure: { level: 'warn', warn_usd: 5, crit_usd: 20 },
  pending_resets: [
    {
      provider: 'openai',
      model: 'gpt-x',
      status_code: 429,
      error_type: 'rate_limit',
      resets_at_ms: 1700000000000,
    },
  ],
};

function render(props: ComponentProps<typeof CostDashboard>) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(CostDashboard, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
});

describe('CostDashboard behaviour', () => {
  it('renders totals, the per-model roll-up and pending resets', async () => {
    spies.fetchCost.mockResolvedValue(costData);
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.kpi-value')).not.toBeNull());
    expect(target.querySelector('[data-kpi="cumulative"] .kpi-value')?.textContent).toContain(
      '9.5000',
    );
    expect(target.querySelector('.models tbody .model-id')?.textContent).toBe('gpt-x');
    expect(target.querySelector('.resets')?.textContent).toContain('openai/gpt-x');
    await unmount(component);
  });

  it('renders the empty state when there is no HUD URL', async () => {
    const { target, component } = render({ hudUrl: null });

    expect(target.querySelector('.empty')?.textContent).toContain('No cost data yet.');
    await unmount(component);
  });

  it('renders the loading skeleton while the fetch is in flight', async () => {
    spies.fetchCost.mockReturnValue(new Promise(() => {}));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.skeleton')).not.toBeNull());
    await unmount(component);
  });

  it('renders the error state and Retry re-fetches', async () => {
    spies.fetchCost.mockRejectedValueOnce(new Error('cost down'));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.error')?.textContent).toContain('cost down');

    spies.fetchCost.mockResolvedValue(costData);
    target.querySelector<HTMLButtonElement>('.error .retry')?.click();
    await vi.waitFor(() => expect(target.querySelector('.kpi-value')).not.toBeNull());
    expect(target.querySelector('.error')).toBeNull();
    expect(spies.fetchCost).toHaveBeenCalledTimes(2);
    await unmount(component);
  });
});

// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for AvailabilityCard (FASE 13 / hallazgo G7).
// Mounts the real component in jsdom and asserts the happy policy + RUN NOW
// state plus that an incomplete `{}` response renders an explanatory empty
// state instead of crashing.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import AvailabilityCard from './AvailabilityCard.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchAvailability: vi.fn() };
});

import { fetchAvailability, type AvailabilityResponse } from '$stores/hud';

const mFetch = vi.mocked(fetchAvailability);

const full: AvailabilityResponse = {
  enabled: true,
  policy: { eta_ms: 120_000, weight_threshold: 0.5, horizon_ms: 0, enabled: true },
  availability: 'RunNow',
  pending_mission: 'm-12345678',
};

let target: HTMLElement;
let app: ReturnType<typeof mount> | null = null;

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
  flushSync();
}

beforeEach(() => {
  target = document.createElement('div');
  document.body.appendChild(target);
  vi.clearAllMocks();
});

afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
});

describe('AvailabilityCard', () => {
  it('renders the policy and the RUN NOW state', async () => {
    mFetch.mockResolvedValue(full);
    app = mount(AvailabilityCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('RUN NOW');
    expect(target.textContent).toContain('2 min');
    expect(target.textContent).toContain('0.50');
  });

  it('does not crash on a {} response and shows the incomplete state', async () => {
    mFetch.mockResolvedValue({} as unknown as AvailabilityResponse);
    app = mount(AvailabilityCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.querySelector('.avail-card')).not.toBeNull();
    expect(target.textContent).toContain('missing fields');
    expect(target.textContent).toContain('no data');
    expect(target.textContent).not.toContain('min');
  });
});

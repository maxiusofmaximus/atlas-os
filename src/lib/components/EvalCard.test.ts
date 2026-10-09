// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for EvalCard (FASE 13 / hallazgo G7). Mounts the
// real component in jsdom and asserts the happy metrics plus that an incomplete
// `{}` response renders an explanatory empty state instead of crashing.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import EvalCard from './EvalCard.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchEvalSummary: vi.fn() };
});

import { fetchEvalSummary, type EvalSummaryResponse } from '$stores/hud';

const mFetch = vi.mocked(fetchEvalSummary);

const summary = {
  runs: 3,
  total: 10,
  passed: 8,
  failed: 1,
  errored: 1,
  pass_rate: 0.8,
  tokens_total: 1000,
  tokens_per_solved: 125.4,
  no_action_turns: 0,
  cost_usd: 0.4,
  cost_per_solved: 0.05,
  failure_kinds: { timeout: 1 },
};

const full: EvalSummaryResponse = { summary, groups: [{ key: 'golden/gpt', summary }] };

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

describe('EvalCard', () => {
  it('renders the metrics for a full summary', async () => {
    mFetch.mockResolvedValue(full);
    app = mount(EvalCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('pass rate');
    expect(target.textContent).toContain('80.0%');
    expect(target.textContent).toContain('8/10');
  });

  it('does not crash on a {} response and shows the incomplete state', async () => {
    mFetch.mockResolvedValue({} as unknown as EvalSummaryResponse);
    app = mount(EvalCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.querySelector('.eval-card')).not.toBeNull();
    expect(target.textContent).toContain('incomplete');
    expect(target.textContent).not.toContain('pass rate');
  });
});

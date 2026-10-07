// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for AgentCard (RFC 67 §4, lote F1). Mounts the real
// component in jsdom, drives the WS store (data/unknown) and the REST tail mock,
// and asserts the DOM for the data, empty, error and Unknown states plus the
// Retry callback.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import AgentCard from './AgentCard.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  const { writable } = await import('svelte/store');
  const store = writable({ connected: true, url: 'http://hud', events: [] as unknown[] });
  return {
    ...actual,
    hud: Object.assign(store, { connect: () => {}, disconnect: () => {} }),
    fetchTail: vi.fn(),
  };
});

import { fetchTail, hud, type AgentStepPayload, type HudState } from '$stores/hud';

const mFetch = vi.mocked(fetchTail);
const setHud = (hud as unknown as { set: (value: HudState) => void }).set.bind(hud);

const step: AgentStepPayload = {
  run_id: 'run-abcdef123456',
  step: 1,
  action: 'run_command',
  observation: 'ok',
  verdict: 'pass',
  tokens_in: 5,
  tokens_out: 7,
  cost_usd: 0.01,
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
  setHud({ connected: true, url: 'http://hud', events: [] });
});

afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
});

describe('AgentCard', () => {
  it('renders the agent run from the REST tail when no live run exists', async () => {
    mFetch.mockResolvedValue([step]);
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('run-abcdef12');
    expect(target.textContent).toContain('5↑ / 7↓');
    expect(target.querySelector('.status')?.textContent).toBe('coding');
  });

  it('renders the empty state when there is no run', async () => {
    mFetch.mockResolvedValue([]);
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No agent run yet');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mFetch.mockRejectedValueOnce(new Error('tail down'));
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('no se pudo cargar el agente');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    mFetch.mockResolvedValueOnce([step]);
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('run-abcdef12');
  });

  it('derives the Unknown status with a ? glyph when the heartbeat is stale', async () => {
    const stale = new Date(Date.now() - 60_000).toISOString();
    setHud({
      connected: true,
      url: 'http://hud',
      events: [
        { id: '1', ts: stale, kind: 'agent_step', payload: { ...step, verdict: null } },
        { id: '2', ts: stale, kind: 'agent_heartbeat', payload: { agent_id: 'a1' } },
      ],
    });
    mFetch.mockResolvedValue([]);
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.querySelector('.status')?.textContent).toBe('unknown');
    expect(target.querySelector('.spine')?.textContent).toBe('?');
  });
});

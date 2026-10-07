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

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: '',
    json: async () => body,
  } as unknown as Response;
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
  vi.unstubAllGlobals();
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

  it('shows the empty state (not success/Worked) when a run id has no steps (D3)', async () => {
    setHud({
      connected: true,
      url: 'http://hud',
      events: [
        {
          id: '1',
          ts: new Date().toISOString(),
          kind: 'agent_step',
          payload: { run_id: 'run-abcdef123456' },
        },
        {
          id: '2',
          ts: new Date().toISOString(),
          kind: 'agent_status_changed',
          payload: { agent_id: 'a1', status: 'success' },
        },
      ],
    });
    mFetch.mockResolvedValue([]);
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No agent run yet');
    expect(target.textContent).not.toContain('Worked for');
  });

  it('reveals the task notes and sandbox frame panels (H-05/H-08)', async () => {
    mFetch.mockResolvedValue([step]);
    vi.stubGlobal(
      'fetch',
      vi.fn(async (url: RequestInfo | URL) => {
        if (String(url).includes('/hud/agent/')) {
          return jsonResponse({ reason: 'unknown agent run run-abcdef12' }, 404);
        }
        return jsonResponse([
          {
            id: 'n1',
            task_id: 'run-abcdef123456',
            file_path: null,
            line_no: null,
            body: 'card note',
            author: 'max',
            created_at: '2026-10-07T10:00:00Z',
          },
        ]);
      }),
    );
    app = mount(AgentCard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    const toggle = Array.from(target.querySelectorAll('button')).find((b) =>
      b.textContent?.includes('Notes & frame'),
    );
    expect(toggle).not.toBeNull();
    toggle?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('Task notes');
    expect(target.textContent).toContain('card note');
    expect(target.textContent).toContain('Sandbox frame');
    expect(target.textContent).toContain('unknown agent run');
  });
});

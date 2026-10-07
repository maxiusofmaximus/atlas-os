// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for KanbanBoard (RFC 67 §7, lote F6-F9).
// Mounts the real component in jsdom and drives its data contract by mocking
// the store fetcher; asserts the DOM for the data, empty and error states and
// that the Retry callback re-fetches.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import KanbanBoard from './KanbanBoard.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchMissions: vi.fn() };
});

import { fetchMissions } from '$stores/hud';

const mFetch = vi.mocked(fetchMissions);

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

describe('KanbanBoard', () => {
  it('renders missions grouped into columns', async () => {
    mFetch.mockResolvedValue([
      { id: 'mission-aaaa1111', label: 'Fix login', status: 'running' },
      { id: 'mission-bbbb2222', label: 'Ship docs', status: 'done' },
    ]);
    app = mount(KanbanBoard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('Fix login');
    expect(target.textContent).toContain('Ship docs');
    expect(target.querySelector('.dot.done')).not.toBeNull();
  });

  it('renders the empty state when there are no missions', async () => {
    mFetch.mockResolvedValue([]);
    app = mount(KanbanBoard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No missions yet');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mFetch.mockRejectedValueOnce(new Error('boom'));
    app = mount(KanbanBoard, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('boom');
    const retry = target.querySelector('button');
    expect(retry).not.toBeNull();
    mFetch.mockResolvedValueOnce([
      { id: 'mission-cccc3333', label: 'Recovered', status: 'pending' },
    ]);
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('Recovered');
  });
});

// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for GraphView (RFC 67 §8, lote F6-F9). Mounts the
// real component in jsdom, mocks the store fetcher, and asserts the DOM for the
// data, empty and error states plus the Retry callback.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import GraphView from './GraphView.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchGraph: vi.fn() };
});

import { fetchGraph, type MissionGraph } from '$stores/hud';

const mFetch = vi.mocked(fetchGraph);

const graph: MissionGraph = {
  mission_id: 'm1',
  nodes: [
    {
      id: 'n1',
      mission_id: 'm1',
      kind: 'mission',
      label: 'LoginBug',
      provenance: 'EXTRACTED',
      attrs_json: '{}',
    },
  ],
  edges: [
    {
      id: 'e1',
      mission_id: 'm1',
      src: 'n1',
      dst: 'n2',
      kind: 'calls',
      precondition: null,
      guard: null,
      visit_count: 2,
    },
  ],
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

describe('GraphView', () => {
  it('renders nodes and edges from the graph', async () => {
    mFetch.mockResolvedValue(graph);
    app = mount(GraphView, { target, props: { hudUrl: 'http://hud', missionId: 'm1' } });
    await settle();
    expect(target.textContent).toContain('LoginBug');
    expect(target.textContent).toContain('1 nodes · 1 edges');
    expect(target.textContent).toContain('calls');
  });

  it('renders the empty state when no mission is selected', async () => {
    app = mount(GraphView, { target, props: { hudUrl: 'http://hud', missionId: '' } });
    await settle();
    expect(target.textContent).toContain('No graph persisted');
    expect(mFetch).not.toHaveBeenCalled();
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mFetch.mockRejectedValueOnce(new Error('graph boom'));
    app = mount(GraphView, { target, props: { hudUrl: 'http://hud', missionId: 'm1' } });
    await settle();
    expect(target.textContent).toContain('graph boom');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    mFetch.mockResolvedValueOnce(graph);
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('LoginBug');
  });
});

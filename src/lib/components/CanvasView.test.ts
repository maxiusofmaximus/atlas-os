// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for CanvasView (RFC 67 §8, lote F6-F9). Mounts the
// real component in jsdom; mocks the mission list and the nested graph fetch and
// asserts the DOM for the data, empty and error states plus the Retry callback.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import CanvasView from './CanvasView.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchMissions: vi.fn(), fetchGraph: vi.fn() };
});

import { fetchMissions, fetchGraph, type MissionGraph } from '$stores/hud';

const mMissions = vi.mocked(fetchMissions);
const mGraph = vi.mocked(fetchGraph);

const graph: MissionGraph = {
  mission_id: 'm-aaaa1111',
  nodes: [
    {
      id: 'n1',
      mission_id: 'm-aaaa1111',
      kind: 'mission',
      label: 'NodeX',
      provenance: 'INFERRED',
      attrs_json: '{}',
    },
  ],
  edges: [],
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

describe('CanvasView', () => {
  it('renders the mission picker and the selected mission graph', async () => {
    mMissions.mockResolvedValue([{ id: 'm-aaaa1111', label: 'Mission A', status: 'running' }]);
    mGraph.mockResolvedValue(graph);
    app = mount(CanvasView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    await settle();
    expect(target.textContent).toContain('Mission A');
    expect(target.textContent).toContain('NodeX');
  });

  it('renders the empty state when there are no missions', async () => {
    mMissions.mockResolvedValue([]);
    app = mount(CanvasView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No mission selected');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mMissions.mockRejectedValueOnce(new Error('missions boom'));
    app = mount(CanvasView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('missions boom');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    mMissions.mockResolvedValueOnce([{ id: 'm-bbbb2222', label: 'Recovered', status: 'pending' }]);
    mGraph.mockResolvedValue(graph);
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    await settle();
    expect(target.textContent).toContain('Recovered');
  });
});

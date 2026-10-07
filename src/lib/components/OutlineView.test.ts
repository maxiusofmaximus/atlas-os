// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for OutlineView (RFC 67 §9, lote F6-F9). Mounts the
// real component in jsdom, mocks the plan tail and plan payload fetchers, and
// asserts the DOM for the data, empty and error states plus the Retry callback.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import OutlineView from './OutlineView.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchTail: vi.fn(), fetchPayload: vi.fn() };
});

import { fetchTail, fetchPayload } from '$stores/hud';

const mTail = vi.mocked(fetchTail);
const mPayload = vi.mocked(fetchPayload);

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

describe('OutlineView', () => {
  it('renders the selected plan roadmap milestones', async () => {
    mTail.mockResolvedValue([{ plan_id: 'plan-12345678', mission_id: 'm1', strategy: 'staged' }]);
    mPayload.mockResolvedValue({ roadmap: [{ id: 'ms1', label: 'Milestone one' }] });
    app = mount(OutlineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('Milestone one');
  });

  it('renders the empty state when there are no plans', async () => {
    mTail.mockResolvedValue([]);
    app = mount(OutlineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No plan selected');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mTail.mockRejectedValueOnce(new Error('plans boom'));
    app = mount(OutlineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('plans boom');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    mTail.mockResolvedValueOnce([{ plan_id: 'plan-87654321', mission_id: 'm2' }]);
    mPayload.mockResolvedValueOnce({ roadmap: [{ id: 'ms2', label: 'Recovered milestone' }] });
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('Recovered milestone');
  });
});

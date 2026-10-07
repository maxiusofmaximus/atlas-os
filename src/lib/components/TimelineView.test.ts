// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for TimelineView (RFC 67 §10, lote F6-F9). Mounts the
// real component in jsdom, mocks the journal page fetch, and asserts the DOM for
// the data, empty and error states plus the Retry callback.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import TimelineView from './TimelineView.svelte';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchJournalPage: vi.fn() };
});

import { fetchJournalPage } from '$stores/hud';

const mFetch = vi.mocked(fetchJournalPage);

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

describe('TimelineView', () => {
  it('renders journal entries as a chronological strip', async () => {
    mFetch.mockResolvedValue({
      entries: [
        {
          id: 1,
          ts: '2026-10-06T10:00:00.000Z',
          kind: 'mission_consolidated',
          payload: { mission_id: 'm1' },
        },
      ],
      total: 1,
      limit: 50,
      offset: 0,
    });
    app = mount(TimelineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('mission_consolidated');
    expect(target.textContent).toContain('1 / 1');
  });

  it('renders the empty state when there are no journal events', async () => {
    mFetch.mockResolvedValue({ entries: [], total: 0, limit: 50, offset: 0 });
    app = mount(TimelineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('No journal events yet');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    mFetch.mockRejectedValueOnce(new Error('journal boom'));
    app = mount(TimelineView, { target, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(target.textContent).toContain('journal boom');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    mFetch.mockResolvedValueOnce({
      entries: [{ id: 2, ts: '2026-10-06T11:00:00.000Z', kind: 'agent_step', payload: null }],
      total: 1,
      limit: 50,
      offset: 0,
    });
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('agent_step');
  });
});

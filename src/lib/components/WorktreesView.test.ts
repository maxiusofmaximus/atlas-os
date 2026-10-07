// @vitest-environment jsdom
// Atlas OS — WorktreesView behaviour (RFC 67 §22.1 F10). Mounts the component
// with a mocked `fetch` and asserts the DOM in the data / empty / error states.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import WorktreesView from './WorktreesView.svelte';

async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await tick();
}

function stubFetch(handler: (url: string) => unknown): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL) => {
      const body = handler(String(input));
      return { ok: true, status: 200, statusText: 'OK', json: async () => body } as Response;
    }),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('WorktreesView', () => {
  it('renders the worktree table when entries exist', async () => {
    stubFetch(() => ({
      repo: '/repo',
      ok: true,
      reason: null,
      entries: [{ path: '/repo/wt-a', branch: 'feature/a', detached: false }],
    }));
    const c = mount(WorktreesView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('/repo/wt-a');
    expect(document.body.textContent).toContain('feature/a');
    expect(document.body.querySelector('table')).toBeTruthy();
    expect(document.body.textContent).toContain('v1');
    unmount(c);
  });

  it('renders the empty state when the repo has no worktrees', async () => {
    stubFetch(() => ({ repo: '/repo', ok: true, reason: null, entries: [] }));
    const c = mount(WorktreesView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('has no worktrees');
    unmount(c);
  });

  it('surfaces the fail-safe reason when the repo is not a git repo', async () => {
    stubFetch(() => ({
      repo: '/repo',
      ok: false,
      reason: 'not a git repository',
      entries: [],
    }));
    const c = mount(WorktreesView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('not a git repository');
    unmount(c);
  });
});

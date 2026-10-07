// @vitest-environment jsdom
// Atlas OS — MissionRail behaviour (RFC 67 §2, §20 H-01/H-09). Mounts the rail
// with a routed fetch mock and asserts data / empty / error, the recent↔frecency
// toggle, the `+ New` form (inline 400), plus D1 refresh and the C4 region.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import MissionRail from './MissionRail.svelte';
import { missionSort, newMissionOpen } from '$stores/views';

async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await tick();
  flushSync();
}

type RouteResult = { status?: number; body: unknown };

function stubRoutes(handler: (url: string, init?: RequestInit) => RouteResult): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL, init?: RequestInit) => {
      const { status = 200, body } = handler(String(input), init);
      return {
        ok: status >= 200 && status < 300,
        status,
        statusText: status === 400 ? 'Bad Request' : 'OK',
        json: async () => body,
      } as Response;
    }),
  );
}

const recent = (missions: unknown[]): RouteResult => ({
  body: { sort: 'recent', count: missions.length, missions },
});

afterEach(() => {
  vi.unstubAllGlobals();
  vi.useRealTimers();
  localStorage.clear();
  missionSort.set('recent');
  newMissionOpen.set(false);
  document.body.innerHTML = '';
});

describe('MissionRail', () => {
  it('renders missions with their rollup and fires onselect on click', async () => {
    stubRoutes(() =>
      recent([
        { id: 'm1', label: 'Fix bug', status: 'running', frecency: 0 },
        { id: 'm2', label: 'Danger', status: 'doom_loop', frecency: 0 },
      ]),
    );
    const onselect = vi.fn();
    const c = mount(MissionRail, {
      target: document.body,
      props: { hudUrl: 'http://hud', activeMissionId: 'm1', onselect },
    });
    await settle();
    expect(document.body.textContent).toContain('Fix bug');
    expect(document.body.textContent).toContain('running');
    expect(document.body.textContent).toContain('Danger');
    expect(document.body.textContent).toContain('blocked');
    const items = document.body.querySelectorAll<HTMLButtonElement>('.item');
    expect(items[0]?.getAttribute('aria-current')).toBe('true');
    items[0]?.click();
    expect(onselect).toHaveBeenCalledWith('m1');
    unmount(c);
  });

  it('renders the empty state with a CTA', async () => {
    stubRoutes(() => recent([]));
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('Sin misiones');
    expect(document.body.textContent).toContain('+ New Mission');
    unmount(c);
  });

  it('renders the error state with a Retry when the request fails', async () => {
    stubRoutes(() => ({ status: 500, body: {} }));
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('no se pudo cargar misiones');
    expect(document.body.textContent).toContain('Retry');
    unmount(c);
  });

  it('toggles recent↔frecency, refetches and persists the choice (H-01)', async () => {
    stubRoutes((url) =>
      url.includes('sort=frecency')
        ? {
            body: {
              sort: 'frecency',
              count: 2,
              missions: [
                { id: 'm2', label: 'Hot', status: 'running', frecency: 9 },
                { id: 'm1', label: 'Cold', status: 'running', frecency: 1 },
              ],
            },
          }
        : recent([
            { id: 'm1', label: 'Cold', status: 'running', frecency: 1 },
            { id: 'm2', label: 'Hot', status: 'running', frecency: 9 },
          ]),
    );
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    const names = (): string[] =>
      [...document.body.querySelectorAll('.item .name')].map((n) => n.textContent ?? '');
    expect(names()[0]).toBe('Cold');
    const freq = [...document.body.querySelectorAll<HTMLButtonElement>('.sort')].find((b) =>
      b.textContent?.includes('Frequency'),
    );
    freq?.click();
    await settle();
    expect(names()[0]).toBe('Hot');
    expect(localStorage.getItem('atlas.missionSort')).toBe('frecency');
    unmount(c);
  });

  it('refreshes on a light poll so a mission created later appears (D1)', async () => {
    vi.useFakeTimers();
    let rows = [{ id: 'm1', label: 'First', status: 'running', frecency: 0 }];
    stubRoutes(() => recent(rows));
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await vi.advanceTimersByTimeAsync(0);
    flushSync();
    expect(document.body.textContent).toContain('First');
    rows = [...rows, { id: 'm2', label: 'Second', status: 'received', frecency: 0 }];
    await vi.advanceTimersByTimeAsync(4000);
    flushSync();
    expect(document.body.textContent).toContain('Second');
    unmount(c);
  });

  it('exposes the mission-rail region with focusable items (:m target, C4)', async () => {
    stubRoutes(() => recent([{ id: 'm1', label: 'Fix bug', status: 'running', frecency: 0 }]));
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    const rail = document.querySelector('[data-region="mission-rail"]');
    expect(rail).not.toBeNull();
    const item = rail?.querySelector<HTMLElement>('.item');
    expect(item).not.toBeNull();
    item?.focus();
    expect(document.activeElement).toBe(item);
    unmount(c);
  });

  it('opens the + New form and shows the 400 inline for an empty prompt (H-09)', async () => {
    stubRoutes(() => recent([]));
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    [...document.body.querySelectorAll<HTMLButtonElement>('button')]
      .find((b) => b.textContent?.includes('+ New'))
      ?.click();
    await settle();
    const form = document.querySelector<HTMLFormElement>('.new-form');
    expect(form).not.toBeNull();
    form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(document.body.querySelector('.new-error')?.textContent).toContain(
      'prompt must not be empty',
    );
    unmount(c);
  });

  it('creates a mission via POST /hud/missions and closes the form', async () => {
    stubRoutes((_url, init) => {
      if (init?.method === 'POST')
        return { status: 201, body: { mission_id: 'new-1', status: 'received' } };
      return recent([]);
    });
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    newMissionOpen.set(true);
    await settle();
    const input = document.querySelector<HTMLInputElement>('.new-prompt');
    expect(input).not.toBeNull();
    if (input) {
      input.value = 'ship the rate limiter';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    document
      .querySelector('.new-form')
      ?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    const fetchMock = vi.mocked(fetch);
    expect(
      fetchMock.mock.calls.some(([, i]) => (i as RequestInit | undefined)?.method === 'POST'),
    ).toBe(true);
    expect(document.body.querySelector('.new-form')).toBeNull();
    unmount(c);
  });

  it('shows a server 400 inline when creation is rejected', async () => {
    stubRoutes((_url, init) => {
      if (init?.method === 'POST') return { status: 400, body: { error: 'prompt too long' } };
      return recent([]);
    });
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    newMissionOpen.set(true);
    await settle();
    const input = document.querySelector<HTMLInputElement>('.new-prompt');
    if (input) {
      input.value = 'x';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    }
    document
      .querySelector('.new-form')
      ?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(document.body.querySelector('.new-error')?.textContent).toContain('prompt too long');
    unmount(c);
  });
});

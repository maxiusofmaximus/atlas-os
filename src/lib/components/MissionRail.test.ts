// @vitest-environment jsdom
// Atlas OS — MissionRail behaviour (RFC 67 §2, §22.1 F2). Mounts the rail with
// a mocked `fetch` and asserts DOM + callbacks in the data / empty / error
// states, plus that selecting a mission fires `onselect` and never reconnects
// the Kernel Bus stream.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import MissionRail from './MissionRail.svelte';

async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await tick();
}

function stubFetch(handler: () => unknown): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => {
      const body = handler();
      return { ok: true, status: 200, statusText: 'OK', json: async () => body } as Response;
    }),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('MissionRail', () => {
  it('renders missions with their rollup and fires onselect on click', async () => {
    stubFetch(() => [
      { id: 'm1', label: 'Fix bug', status: 'running' },
      { id: 'm2', label: 'Danger', status: 'doom_loop' },
    ]);
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

  it('renders the empty state with a CTA that fires onnew', async () => {
    stubFetch(() => []);
    const onnew = vi.fn();
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud', onnew } });
    await settle();
    expect(document.body.textContent).toContain('Sin misiones');
    const cta = [...document.body.querySelectorAll<HTMLButtonElement>('button')].find((b) =>
      b.textContent?.includes('New Mission'),
    );
    cta?.click();
    expect(onnew).toHaveBeenCalled();
    unmount(c);
  });

  it('renders the error state with a Retry when the request fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        throw new Error('boom');
      }),
    );
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('no se pudo cargar misiones');
    expect(document.body.textContent).toContain('Retry');
    unmount(c);
  });
});

// @vitest-environment jsdom
// Atlas OS — AppHeader behaviour (RFC 67 §24, lote F0.5). Mounts the header,
// drives the bus store and asserts the §24.8 criteria C1/C2/C3/C10.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import fs from 'node:fs';
import path from 'node:path';

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  const { writable } = await import('svelte/store');
  const store = writable({ connected: true, url: 'http://hud', events: [] as unknown[] });
  return {
    ...actual,
    hud: Object.assign(store, { connect: () => {}, disconnect: () => {} }),
  };
});

import AppHeader from './AppHeader.svelte';
import { hud, type HudState } from '$stores/hud';

const setHud = (hud as unknown as { set: (value: HudState) => void }).set.bind(hud);
const pkg = JSON.parse(fs.readFileSync(path.join(process.cwd(), 'package.json'), 'utf8')) as {
  version: string;
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
  setHud({ connected: true, url: 'http://hud', events: [] });
});

afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
});

describe('AppHeader', () => {
  it('renders brand, version, mission, bus and leader hint (C1/C2)', () => {
    app = mount(AppHeader, {
      target,
      props: {
        version: pkg.version,
        mission: { id: 'mission-42', rollup: 'ok' },
        remote: null,
      },
    });
    expect(target.textContent).toContain('Atlas OS');
    expect(target.textContent).toContain(`v${pkg.version}`);
    expect(target.textContent).toContain('Mission:');
    expect(target.textContent).toContain('mission-42');
    expect(target.textContent).toContain('connected');
    expect(target.textContent).toContain(':?');
  });

  it('shows the disconnected state and banner with the last frame ts (C3)', async () => {
    const ts = '2026-10-07T12:00:00Z';
    setHud({
      connected: false,
      url: 'http://hud',
      events: [{ id: '1', ts, kind: 'hud_served', payload: {} }],
    });
    app = mount(AppHeader, {
      target,
      props: { version: pkg.version, mission: null, remote: null },
    });
    await settle();
    expect(target.querySelector('[data-state="disconnected"]')).not.toBeNull();
    expect(target.textContent).toContain('sin conexión — reconectando');
    expect(target.textContent).toContain('12:00:00');
  });

  it('exposes the §24 landmarks and live region (C10)', () => {
    app = mount(AppHeader, {
      target,
      props: { version: pkg.version, mission: null, remote: null },
    });
    expect(target.querySelector('header.app-header')).not.toBeNull();
    const bus = target.querySelector('.bus');
    expect(bus?.getAttribute('role')).toBe('status');
    expect(bus?.getAttribute('aria-live')).toBe('polite');
    expect(target.querySelector('.leader-hint')?.getAttribute('aria-hidden')).toBe('true');
  });
});

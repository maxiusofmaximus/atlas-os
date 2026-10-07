// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for the sandbox-frame panel (RFC 67 §20 H-08,
// F11-c). Mounts the real component in jsdom and asserts the DOM: the frame, the
// empty frame, the explanatory 404 (`{reason}`, never a generic error) and a
// real 5xx error state.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import SandboxSnapshot from './SandboxSnapshot.svelte';

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: '',
    json: async () => body,
  } as unknown as Response;
}

const frame = {
  run_id: 'run-1',
  sandbox: 'local',
  root: '/tmp/run-1',
  file_count: 1,
  files: [{ path: 'out.txt', sha256: 'a'.repeat(64) }],
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
});

afterEach(() => {
  if (app) unmount(app);
  app = null;
  target.remove();
  vi.unstubAllGlobals();
});

describe('SandboxSnapshot', () => {
  it('renders the frame files and the sandbox kind', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse(frame)),
    );
    app = mount(SandboxSnapshot, { target, props: { hudUrl: 'http://hud', runId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('out.txt');
    expect(target.textContent).toContain('local');
  });

  it('renders the empty state for a frame with no files', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse({ ...frame, file_count: 0, files: [] })),
    );
    app = mount(SandboxSnapshot, { target, props: { hudUrl: 'http://hud', runId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('Sandbox frame is empty');
  });

  it('renders a 404 reason as an explanatory empty state, not a generic error', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse({ reason: 'unknown agent run run-1' }, 404)),
    );
    app = mount(SandboxSnapshot, { target, props: { hudUrl: 'http://hud', runId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('unknown agent run run-1');
    expect(target.querySelector('.unavailable')).not.toBeNull();
    expect(target.querySelector('.error')).toBeNull();
    expect(target.querySelector('[role="alert"]')).toBeNull();
  });

  it('renders a real 5xx as the error state and re-fetches on Retry', async () => {
    let calls = 0;
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        calls += 1;
        return calls === 1 ? jsonResponse({}, 500) : jsonResponse(frame);
      }),
    );
    app = mount(SandboxSnapshot, { target, props: { hudUrl: 'http://hud', runId: 'run-1' } });
    await settle();
    expect(target.querySelector('.error')).not.toBeNull();
    expect(target.textContent).toContain('could not load sandbox frame');
    const retry = target.querySelector('.error button');
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('out.txt');
  });
});

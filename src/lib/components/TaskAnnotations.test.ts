// @vitest-environment jsdom
//
// Atlas OS — behaviour tests for the task-notes panel (RFC 67 §20 H-05, F11-b).
// Mounts the real component in jsdom and asserts the DOM for the data, empty and
// error states, plus that an empty body surfaces the backend 422.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import TaskAnnotations from './TaskAnnotations.svelte';

function jsonResponse(body: unknown, status = 200): Response {
  return {
    ok: status >= 200 && status < 300,
    status,
    statusText: '',
    json: async () => body,
  } as unknown as Response;
}

const note = {
  id: 'a1',
  task_id: 'run-1',
  file_path: null,
  line_no: null,
  body: 'first note',
  author: 'max',
  created_at: '2026-10-07T10:00:00Z',
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

describe('TaskAnnotations', () => {
  it('lists the notes returned by the backend', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () =>
        jsonResponse([note, { ...note, id: 'a2', body: 'second note', author: 'ops' }]),
      ),
    );
    app = mount(TaskAnnotations, { target, props: { hudUrl: 'http://hud', taskId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('first note');
    expect(target.textContent).toContain('second note');
  });

  it('renders the empty state when there are no notes', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => jsonResponse([])),
    );
    app = mount(TaskAnnotations, { target, props: { hudUrl: 'http://hud', taskId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('No task notes yet');
  });

  it('renders the error state and re-fetches on Retry', async () => {
    let calls = 0;
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => {
        calls += 1;
        return calls === 1 ? jsonResponse({}, 500) : jsonResponse([note]);
      }),
    );
    app = mount(TaskAnnotations, { target, props: { hudUrl: 'http://hud', taskId: 'run-1' } });
    await settle();
    expect(target.textContent).toContain('could not load task notes');
    const retry = target.querySelector('.error button');
    expect(retry).not.toBeNull();
    retry?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    await settle();
    expect(target.textContent).toContain('first note');
  });

  it('surfaces the backend 422 when the body is empty', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async (_url: string, init?: RequestInit) =>
        init?.method === 'POST' ? jsonResponse({}, 422) : jsonResponse([]),
      ),
    );
    app = mount(TaskAnnotations, { target, props: { hudUrl: 'http://hud', taskId: 'run-1' } });
    await settle();
    const form = target.querySelector('form');
    expect(form).not.toBeNull();
    form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));
    await settle();
    expect(target.textContent).toContain('422');
  });
});

// @vitest-environment jsdom
// Atlas OS — McpView behaviour (RFC 67 §22.1 F10). Mounts the component with a
// mocked `fetch` and asserts the DOM in the data / empty / error states.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import McpView from './McpView.svelte';

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

function stubFetchHttpError(status: number): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async () => ({ ok: false, status, statusText: 'Server Error' }) as Response),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('McpView', () => {
  it('renders servers and their allowlist when the catalog loads', async () => {
    stubFetch(() => ({
      repo: '/r',
      ok: true,
      reason: null,
      path: '.opencode/mcp.json',
      servers: [
        {
          name: 'context7',
          type: 'stdio',
          enabled: true,
          command: ['npx', 'context7'],
          allowed_tools: ['resolve-library-id'],
        },
      ],
    }));
    const c = mount(McpView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('context7');
    expect(document.body.textContent).toContain('resolve-library-id');
    expect(document.body.textContent).toContain('v1');
    unmount(c);
  });

  it('renders the empty state when no servers are declared', async () => {
    stubFetch(() => ({
      repo: '/r',
      ok: true,
      reason: null,
      path: '.opencode/mcp.json',
      servers: [],
    }));
    const c = mount(McpView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('No MCP servers declared');
    unmount(c);
  });

  it('renders an error when the request fails', async () => {
    stubFetchHttpError(500);
    const c = mount(McpView, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('Error');
    expect(document.body.textContent).toContain('500');
    unmount(c);
  });
});

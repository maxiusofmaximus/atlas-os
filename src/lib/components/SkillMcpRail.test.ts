// @vitest-environment jsdom
// Atlas OS — SkillMcpRail behaviour (RFC 67 §22.1 F10). Mounts the component
// with a mocked `fetch` routed by URL and asserts data / empty / error states.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, tick } from 'svelte';
import SkillMcpRail from './SkillMcpRail.svelte';

async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await tick();
}

function stubRoutes(routes: (url: string) => unknown): void {
  vi.stubGlobal(
    'fetch',
    vi.fn(async (input: RequestInfo | URL) => {
      const body = routes(String(input));
      return { ok: true, status: 200, statusText: 'OK', json: async () => body } as Response;
    }),
  );
}

afterEach(() => {
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('SkillMcpRail', () => {
  it('renders the skills catalog and MCP servers when loaded', async () => {
    stubRoutes((url) =>
      url.includes('/tail/skills')
        ? [
            {
              skill_id: 'prompt-clarify',
              version: '1.0.0',
              verified: true,
              requires_sandbox: false,
            },
          ]
        : {
            repo: '/r',
            ok: true,
            reason: null,
            servers: [{ name: 'context7', type: 'stdio', enabled: true, command: ['npx'] }],
          },
    );
    const c = mount(SkillMcpRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('prompt-clarify');
    expect(document.body.textContent).toContain('context7');
    expect(document.body.textContent).toContain('v1');
    unmount(c);
  });

  it('renders both empty states when nothing is declared', async () => {
    stubRoutes((url) =>
      url.includes('/tail/skills') ? [] : { repo: '/r', ok: true, servers: [] },
    );
    const c = mount(SkillMcpRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('No skill manifests.');
    expect(document.body.textContent).toContain('No servers declared');
    unmount(c);
  });

  it('renders an error when a request fails', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(async () => ({ ok: false, status: 503, statusText: 'Unavailable' }) as Response),
    );
    const c = mount(SkillMcpRail, { target: document.body, props: { hudUrl: 'http://hud' } });
    await settle();
    expect(document.body.textContent).toContain('Error');
    unmount(c);
  });
});

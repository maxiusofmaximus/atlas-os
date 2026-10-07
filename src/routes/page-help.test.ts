// @vitest-environment jsdom
// Atlas OS — +page integration: the `:` `?` leader combo opens HelpOverlay
// (RFC 67 §24.5). Mounts the real page with a routed fetch mock and a no-op
// WebSocket, then drives window keydown.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import Page from './+page.svelte';
import type { PageData } from './$types';

function route(url: string): unknown {
  const u = url.replace(/^https?:\/\/[^/]+/, '');
  if (u.startsWith('/tail/')) return [];
  if (u.startsWith('/hud/missions')) return { sort: 'recent', count: 0, missions: [] };
  if (u.startsWith('/hud/cost'))
    return {
      window: 200,
      totals: { invocations: 0, cost_usd: 0 },
      by_model: [],
      cumulative_usd: 0,
      budget_usd: null,
      remaining: null,
      pressure: { level: 'ok', warn_usd: 5, crit_usd: 20 },
      pending_resets: [],
    };
  if (u.startsWith('/hud/health'))
    return {
      agent_events: { total: 0, by_type: [], recent: [] },
      agent_runs: [],
      swarm_agents: [],
    };
  if (u.startsWith('/hud/audit')) return { rows: [], count: 0 };
  if (u.startsWith('/hud/worktrees')) return { repo: '', ok: true, reason: null, entries: [] };
  if (u.startsWith('/hud/mcp')) return { repo: '', ok: true, reason: null, servers: [] };
  if (u.startsWith('/hud/secrets')) return { service: 'x', slots: [] };
  if (u.startsWith('/hud/journal')) return { entries: [], total: 0, limit: 20, offset: 0 };
  if (u.startsWith('/hud/availability'))
    return {
      enabled: false,
      policy: { eta_ms: 0, weight_threshold: 0, horizon_ms: 0, enabled: false },
      availability: 'RunNow',
      pending_mission: null,
    };
  if (u.startsWith('/hud/eval/summary'))
    return {
      summary: {
        runs: 0,
        total: 0,
        passed: 0,
        failed: 0,
        errored: 0,
        pass_rate: 0,
        tokens_total: 0,
        tokens_per_solved: 0,
        no_action_turns: 0,
        cost_usd: 0,
        cost_per_solved: 0,
        failure_kinds: {},
      },
      groups: [],
    };
  if (u.startsWith('/hud/demos')) return { artifacts: [], count: 0 };
  if (u.startsWith('/hud/reliability')) return { policy: null, models: [] };
  if (u.startsWith('/remote/status'))
    return { local_only: true, oidc_configured: false, token_configured: false, oidc_issuer: null };
  return {};
}

async function settle(): Promise<void> {
  await new Promise((r) => setTimeout(r, 0));
  await tick();
  flushSync();
}

let app: ReturnType<typeof mount> | null = null;

afterEach(() => {
  if (app) unmount(app);
  app = null;
  vi.unstubAllGlobals();
  document.body.innerHTML = '';
});

describe('+page leader keys', () => {
  it('opens HelpOverlay on : then ? and closes on Esc', async () => {
    vi.stubGlobal(
      'fetch',
      vi.fn(
        async (input: RequestInfo | URL) =>
          ({
            ok: true,
            status: 200,
            statusText: 'OK',
            json: async () => route(String(input)),
          }) as Response,
      ),
    );
    vi.stubGlobal(
      'WebSocket',
      class {
        addEventListener(): void {}
        removeEventListener(): void {}
        close(): void {}
        send(): void {}
      },
    );

    app = mount(Page, {
      target: document.body,
      props: { data: { hudUrl: 'http://hud' } as unknown as PageData },
    });
    await settle();
    expect(document.querySelector('[role="dialog"]')).toBeNull();

    window.dispatchEvent(new KeyboardEvent('keydown', { key: ':' }));
    window.dispatchEvent(new KeyboardEvent('keydown', { key: '?' }));
    await settle();
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');

    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await settle();
    expect(document.querySelector('[role="dialog"]')).toBeNull();
  });
});

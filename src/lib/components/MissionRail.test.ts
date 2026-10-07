// Atlas OS — tests for the Mission Rail (RFC 67 §2, §22.1 F2).
// Behaviour is asserted structurally (the repo convention — see
// `SwarmConsole.test.ts`: client-side `mount` needs a new test dependency,
// which AGENTS.md §4 forbids). The Mission-Rail-specific invariants that do
// not need a DOM — overview dissolved, mission selection not reconnecting the
// WS — are asserted against the stores directly.

import { describe, it, expect } from 'vitest';
import { compile } from 'svelte/compiler';
import { get } from 'svelte/store';
import fs from 'node:fs';
import path from 'node:path';

const source = fs.readFileSync(
  path.join(process.cwd(), 'src/lib/components/MissionRail.svelte'),
  'utf8',
);

describe('MissionRail compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
  });
});

describe('MissionRail states', () => {
  it('renders the loading state as skeletons', () => {
    expect(source).toContain('class="skeleton"');
  });

  it('renders the empty state with a CTA', () => {
    expect(source).toContain('class="empty"');
    expect(source).toContain('Sin misiones');
    expect(source).toContain('+ New Mission');
  });

  it('renders the error state with a Retry', () => {
    expect(source).toContain('class="error"');
    expect(source).toContain('no se pudo cargar misiones');
    expect(source).toContain('Retry');
  });
});

describe('MissionRail wiring', () => {
  it('fetches missions from the tail route', () => {
    expect(source).toContain('fetchMissions');
  });

  it('rolls up severity into a color + glyph + label spine', () => {
    expect(source).toContain('data-sev');
    expect(source).toContain("'blocked'");
    expect(source).toContain("'failed'");
    expect(source).toContain("glyph: '?'");
  });

  it('is a labelled, keyboard-navigable navigation landmark', () => {
    expect(source).toContain('aria-label="Missions"');
    expect(source).toContain('aria-current');
    expect(source).toContain('ArrowDown');
    expect(source).toContain('ArrowUp');
    expect(source).toContain('Home');
    expect(source).toContain('End');
  });

  it('does not touch the WS connection when selecting a mission', () => {
    expect(source).not.toContain('hud.connect(');
    expect(source).not.toContain('hud.disconnect(');
  });
});

describe('overview dissolved + mission selection keeps the stream', () => {
  it('overview is no longer a view and the default is a projection', async () => {
    const { VIEWS, activeView } = await import('../stores/views');
    expect(VIEWS.some((v) => (v.id as string) === 'overview')).toBe(false);
    expect(get(activeView)).toBe('kanban');
  });

  it('setting the active mission does not reconnect the WS', async () => {
    const { hud } = await import('../stores/hud');
    const { activeMissionId } = await import('../stores/views');
    const before = get(hud).connected;
    activeMissionId.set('00000000-0000-0000-0000-000000000001');
    expect(get(hud).connected).toBe(before);
  });
});

// Atlas OS — tests for the Swarm Console HUD card (RFC 31 SECTOR B 4.5).
// The card's behaviour lives in the store projections (`projectSwarmAgents`,
// `projectSwarmInbox`, `countUnread` — pinned in `hud.test.ts`); this file
// pins the card itself: it must compile to both client and server output
// without warnings, and its template must wire the three 4.5 deliverables
// (office floor with desks, mailbox drawer, per-worktree checks button).
//
// Note: client-side `mount` is out of reach without a new test dependency —
// under this repo's vitest setup bare `svelte` resolves to the server entry
// while `.svelte` files compile to the client build (AGENTS.md §4 forbids
// the extra dependency), so render behaviour is asserted structurally.
import { describe, it, expect } from 'vitest';
import { compile } from 'svelte/compiler';
import fs from 'node:fs';
import path from 'node:path';

const source = fs.readFileSync(
  path.join(process.cwd(), 'src/lib/components/SwarmConsole.svelte'),
  'utf8',
);

describe('SwarmConsole compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Swarm Console');
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Swarm Console');
  });
});

describe('SwarmConsole structure', () => {
  it('renders the office floor with one desk per agent', () => {
    expect(source).toContain('class="floor"');
    expect(source).toContain('class="desk"');
    expect(source).toContain('swarmStateColor(agent.state)');
  });

  it('wires the mailbox drawer per desk with unread badge', () => {
    expect(source).toContain('openMailbox(agent.agent_id)');
    expect(source).toContain('class="drawer"');
    expect(source).toContain('unreadFor(agent.agent_id)');
    expect(source).toContain('postSwarmSend');
  });

  it('wires the per-worktree checks button (CN-004)', () => {
    expect(source).toContain('loadChecks(agent)');
    expect(source).toContain('fetchSwarmChecks');
    expect(source).toContain('class="checks"');
  });
});

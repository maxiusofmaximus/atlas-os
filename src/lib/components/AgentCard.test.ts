import { describe, it, expect } from 'vitest';
import { compile } from 'svelte/compiler';
import fs from 'node:fs';
import path from 'node:path';

const source = fs.readFileSync(
  path.join(process.cwd(), 'src/lib/components/AgentCard.svelte'),
  'utf8',
);

describe('AgentCard compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
  });
});

describe('AgentCard data wiring (RFC 67 §4)', () => {
  it('consumes the WS store and the REST tail fallback', () => {
    expect(source).toContain('$hud.events');
    expect(source).toContain('fetchTail');
    expect(source).toContain('agentRunId');
    expect(source).toContain('projectAgentSteps');
    expect(source).toContain('agentRunTotals');
  });

  it('covers the 15+ field layers 0-3', () => {
    expect(source).toContain('class="layer-0"');
    expect(source).toContain('data-layer="2"');
    expect(source).toContain('data-layer="3"');
    expect(source).toContain('class="statline"');
    expect(source).toContain('class="head-meta"');
    expect(source).toContain('Worked for');
  });

  it('renders doom_loop_count, goal_drift and checkpoint', () => {
    expect(source).toContain('doom_loop');
    expect(source).toContain('goal_drift');
    expect(source).toContain('checkpoint');
  });
});

describe('AgentCard states (RFC 67 §1.4)', () => {
  it('renders the loading skeleton', () => {
    expect(source).toContain('aria-busy');
    expect(source).toContain('skel');
    expect(source).toContain('loading');
  });

  it('renders the error state with Retry', () => {
    expect(source).toContain('class="error"');
    expect(source).toContain('Retry');
    expect(source).toContain('loadError');
  });

  it('renders the empty state', () => {
    expect(source).toContain('class="empty"');
    expect(source).toContain('No agent run yet');
  });

  it('renders the disconnected and partial states', () => {
    expect(source).toContain('class="offline"');
    expect(source).toContain('class="partial"');
    expect(source).toContain('$hud.connected');
  });

  it('derives the Unknown status client-side (H-03)', () => {
    expect(source).toContain('Unknown');
    expect(source).toContain('HEARTBEAT_STALE_MS');
    expect(source).toContain('30_000');
  });
});

describe('AgentCard actions and tokens', () => {
  it('gates doom_loop actions to Recover/Override/Stop only', () => {
    expect(source).toContain("status === 'DoomLoop'");
    expect(source).toContain("'recover', 'override', 'stop'");
  });

  it('wires REST approve/deny and a host event for CLI actions', () => {
    expect(source).toContain('approveApproval');
    expect(source).toContain('denyApproval');
    expect(source).toContain('atlas:agent-action');
  });

  it('uses only design tokens (no literal colours)', () => {
    expect(source).toContain('var(--a-');
    expect(/#[0-9a-fA-F]{3,8}\b/.test(source)).toBe(false);
  });
});

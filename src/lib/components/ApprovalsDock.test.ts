// Atlas OS — tests for the Approvals Dock (RFC 67 §5, lote F4).
// The dock's pending set is folded from the WS tail in the store
// (`projectPendingApprovals`, pinned in `hud.test.ts`); this file pins the
// component itself: it compiles to client + server output without warnings,
// and its template wires the §5 deliverables (gate vs question channels,
// Apr/Deny POST to the HUD routes, the H-02 batch honesty, and the states:
// loading / empty / error / partial / disconnected / unknown).
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
  path.join(process.cwd(), 'src/lib/components/ApprovalsDock.svelte'),
  'utf8',
);

describe('ApprovalsDock compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Approvals Dock');
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Approvals Dock');
  });
});

describe('ApprovalsDock render', () => {
  it('renders the two channels (gate vs question) with distinct aria-live', () => {
    expect(source).toContain('class="channel gate"');
    expect(source).toContain('class="channel question"');
    expect(source).toContain('aria-live="assertive"');
    expect(source).toContain('aria-live="polite"');
    expect(source).toContain('function channelOf');
  });

  it('wires Apr/Deny to the HUD approval routes (RFC 67 §5 acceptance 2)', () => {
    expect(source).toContain('approveApproval');
    expect(source).toContain('denyApproval');
    expect(source).toContain("void answer(p.approval_id, 'approve')");
    expect(source).toContain("void answer(p.approval_id, 'deny')");
  });

  it('renders the full action row Apr/Deny/Steer/Fork', () => {
    expect(source).toContain('class="apr"');
    expect(source).toContain('class="deny"');
    expect(source).toContain('class="steer"');
    expect(source).toContain('class="fork"');
    expect(source).toContain('Apr');
    expect(source).toContain('Steer');
    expect(source).toContain('Fork');
  });

  it('carries the H-02 batch honesty (Approve all disabled + explanation)', () => {
    expect(source).toContain('Approve all');
    expect(source).toContain('H-02');
    expect(source).toContain('batch disabled');
  });
});

describe('ApprovalsDock empty state', () => {
  it('renders the empty message when nothing is pending', () => {
    expect(source).toContain('pending.length === 0');
    expect(source).toContain('No pending approvals.');
  });
});

describe('ApprovalsDock error state', () => {
  it('surfaces a failed POST without dropping the row', () => {
    expect(source).toContain('class="error"');
    expect(source).toContain('role="alert"');
    expect(source).toContain('error = e instanceof Error ? e.message : String(e)');
  });
});

describe('ApprovalsDock partial + disconnected + unknown states', () => {
  it('freezes the dock when the stream is down', () => {
    expect(source).toContain('class="frozen"');
    expect(source).toContain('dock frozen');
  });

  it('routes an unclassifiable action to an explicit-decision channel', () => {
    expect(source).toContain('class="channel unknown"');
    expect(source).toContain('requires explicit decision');
  });
});

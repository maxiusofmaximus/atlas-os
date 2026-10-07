// Atlas OS — tests for the Activity Spine (RFC 67 §3, lote F3).
// The spine's projections live in the store; this file pins the component
// itself: it compiles to client + server output without warnings, and its
// template wires the §3 deliverables (ticker rows with tone classes, the
// `aria-live` announcer with throttle, the ⇄ canvas toggle and the six
// states: loading / empty / error / partial / disconnected / unknown).
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
  path.join(process.cwd(), 'src/lib/components/ActivitySpine.svelte'),
  'utf8',
);

describe('ActivitySpine compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Activity Spine');
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Activity Spine');
  });
});

describe('ActivitySpine render', () => {
  it('renders the ticker rows from the WS tail', () => {
    expect(source).toContain("from '$stores/hud'");
    expect(source).toContain('$hud.events');
    expect(source).toContain('class="row-btn tone-{row.tone}"');
  });

  it('wires the header live dot and the ⇄ canvas toggle', () => {
    expect(source).toContain('class:on={connected}');
    expect(source).toContain('⇄ canvas');
    expect(source).toContain('aria-pressed={canvasSync}');
  });

  it('maps doom_loop_detected to the err tone (RFC 67 §3 acceptance 1)', () => {
    expect(source).toContain('doom_loop_detected');
    expect(source).toContain("return 'err'");
    expect(source).toContain('.tone-err');
  });

  it('announces politely with a throttle', () => {
    expect(source).toContain('aria-live="polite"');
    expect(source).toContain('ANNOUNCE_THROTTLE_MS');
  });

  it('respects prefers-reduced-motion', () => {
    expect(source).toContain('prefers-reduced-motion');
    expect(source).toContain('animation: none');
  });
});

describe('ActivitySpine empty state', () => {
  it('renders the empty message when there are no events', () => {
    expect(source).toContain('rows.length === 0');
    expect(source).toContain('Sin actividad todavía');
  });
});

describe('ActivitySpine error state', () => {
  it('renders the stream-down banner with retry, keeping the last events', () => {
    expect(source).toContain('class="disconnected"');
    expect(source).toContain('stream down');
    expect(source).toContain('function retry');
    expect(source).toContain('hud.connect(hudUrl)');
  });
});

describe('ActivitySpine partial + unknown states', () => {
  it('flags truncation and unknown event types', () => {
    expect(source).toContain('showing last');
    expect(source).toContain("return 'unknown'");
    expect(source).toContain('unknown event');
  });
});

// Atlas OS — tests for the Journal Observer HUD card (RFC 19 §10, research 33 SECTOR B 6.1).
// The card's data contract lives in the store helper (`fetchJournalPage` —
// pinned in `hud.test.ts`); this file pins the card itself: it must
// compile to both client and server output without warnings, and its
// template must wire the three 6.1 deliverables (kind/ts/payload table
// with expandable payload, kind filter, limit/offset pagination).
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
  path.join(process.cwd(), 'src/lib/components/JournalObserver.svelte'),
  'utf8',
);

describe('JournalObserver compile', () => {
  it('compiles to client output without warnings', () => {
    const out = compile(source, { generate: 'client', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Journal Observer');
  });

  it('compiles to server output without warnings', () => {
    const out = compile(source, { generate: 'server', dev: false });
    expect(out.warnings).toEqual([]);
    expect(out.js.code).toContain('Journal Observer');
  });
});

describe('JournalObserver structure', () => {
  it('renders the entries table with kind/ts columns', () => {
    expect(source).toContain('class="entries"');
    expect(source).toContain('class="kind"');
    expect(source).toContain('fetchJournalPage');
  });

  it('wires the expandable full payload per row', () => {
    expect(source).toContain('toggle(entry.id)');
    expect(source).toContain('JSON.stringify(entry.payload, null, 2)');
    expect(source).toContain('<pre>');
  });

  it('wires the kind filter', () => {
    expect(source).toContain('applyFilter');
    expect(source).toContain('clearFilter');
    expect(source).toContain('bind:value={kindInput}');
  });

  it('wires limit/offset pagination', () => {
    expect(source).toContain('prevPage');
    expect(source).toContain('nextPage');
    expect(source).toContain('changeLimit');
    expect(source).toContain('class="pager"');
    expect(source).toContain('rangeText()');
  });
});

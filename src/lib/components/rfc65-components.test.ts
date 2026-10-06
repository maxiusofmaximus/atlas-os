// Atlas OS — structural tests for the RFC 65 Mission Control components.
// Follows the repo convention (see `SwarmConsole.test.ts`): client-side `mount`
// needs a new test dependency, which AGENTS.md §4 forbids, so each component is
// asserted structurally — it compiles to client + server output without
// warnings, and its template wires the fetcher / contract it exists to render.

import { describe, it, expect } from 'vitest';
import { compile } from 'svelte/compiler';
import fs from 'node:fs';
import path from 'node:path';

const read = (file: string): string =>
  fs.readFileSync(path.join(process.cwd(), 'src/lib/components', file), 'utf8');

const components: Array<{ file: string; marker: string }> = [
  { file: 'CostDashboard.svelte', marker: 'fetchCost' },
  { file: 'HealthKPIs.svelte', marker: 'fetchHealth' },
  { file: 'AuditTimeline.svelte', marker: 'fetchAudit' },
  { file: 'CanvasView.svelte', marker: 'GraphView' },
  { file: 'OutlineView.svelte', marker: 'fetchPayload' },
  { file: 'TimelineView.svelte', marker: 'fetchJournalPage' },
  { file: 'WorktreesView.svelte', marker: 'fetchWorktrees' },
  { file: 'DemoPane.svelte', marker: 'fetchDemos' },
  { file: 'SkillMcpRail.svelte', marker: 'fetchMcp' },
  { file: 'SettingsView.svelte', marker: 'fetchSecretSlots' },
  { file: 'McpView.svelte', marker: 'fetchMcp' },
];

describe('RFC 65 components — compile', () => {
  for (const { file } of components) {
    it(`${file} compiles to client and server output without warnings`, () => {
      const source = read(file);
      const client = compile(source, { generate: 'client', dev: false });
      expect(client.warnings).toEqual([]);
      const server = compile(source, { generate: 'server', dev: false });
      expect(server.warnings).toEqual([]);
    });
  }
});

describe('RFC 65 components — wiring + empty/error states', () => {
  for (const { file, marker } of components) {
    it(`${file} wires ${marker}`, () => {
      const source = read(file);
      expect(source).toContain(marker);
      expect(source).toContain('error');
      expect(source).toContain('empty');
    });
  }
});

describe('settings view is catalogued', () => {
  it('Settings is in VIEWS with the `s` hotkey and a P1 fase', async () => {
    const { VIEWS } = await import('../stores/views');
    const settings = VIEWS.find((v) => v.id === 'settings');
    expect(settings).toBeDefined();
    expect(settings?.key).toBe('s');
    expect(settings?.fase).toBe('P1');
  });
});

describe('mcp view is catalogued', () => {
  it('MCP is in VIEWS with the `m` hotkey and a P1 fase', async () => {
    const { VIEWS, viewForKey } = await import('../stores/views');
    const mcp = VIEWS.find((v) => v.id === 'mcp');
    expect(mcp).toBeDefined();
    expect(mcp?.key).toBe('m');
    expect(mcp?.fase).toBe('P1');
    expect(viewForKey('m')).toBe('mcp');
  });
});

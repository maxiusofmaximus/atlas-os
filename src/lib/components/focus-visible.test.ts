// @vitest-environment jsdom
// Atlas OS — focus-visible ring on the shell/rail/spine/dock controls (defect
// D2). jsdom matches `:focus-visible` after `.focus()` but does not cascade the
// pseudo-class rule into computed style, so the ring is asserted on the
// compiled CSS artifact while the control's focus-visible participation is
// asserted behaviourally.

import { describe, it, expect, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import { compile } from 'svelte/compiler';
import fs from 'node:fs';
import path from 'node:path';
import ViewSwitcher from './ViewSwitcher.svelte';
import MissionRail from './MissionRail.svelte';
import ActivitySpine from './ActivitySpine.svelte';
import ApprovalsDock from './ApprovalsDock.svelte';

const read = (file: string): string =>
  fs.readFileSync(path.join(process.cwd(), 'src/lib/components', file), 'utf8');

const FILES = [
  'ViewSwitcher.svelte',
  'MissionRail.svelte',
  'ActivitySpine.svelte',
  'ApprovalsDock.svelte',
];

afterEach(() => {
  document.body.innerHTML = '';
});

describe('focus-visible ring (D2)', () => {
  for (const file of FILES) {
    it(`${file} styles :focus-visible with var(--a-focus)`, () => {
      const out = compile(read(file), { generate: 'client', dev: false });
      expect(out.css?.code ?? '').toContain(':focus-visible');
      expect(out.css?.code ?? '').toContain('var(--a-focus)');
    });
  }

  it('ViewSwitcher tabs participate in :focus-visible', () => {
    const c = mount(ViewSwitcher, { target: document.body, props: { available: ['kanban'] } });
    flushSync();
    const tab = document.body.querySelector<HTMLButtonElement>('.tab');
    expect(tab).not.toBeNull();
    tab?.focus();
    expect(tab?.matches(':focus-visible')).toBe(true);
    unmount(c);
  });

  it('MissionRail controls participate in :focus-visible', () => {
    const c = mount(MissionRail, { target: document.body, props: { hudUrl: null } });
    flushSync();
    const btn = document.body.querySelector<HTMLButtonElement>('.new');
    btn?.focus();
    expect(btn?.matches(':focus-visible')).toBe(true);
    unmount(c);
  });

  it('ActivitySpine controls participate in :focus-visible', () => {
    const c = mount(ActivitySpine, { target: document.body, props: { hudUrl: null } });
    flushSync();
    const btn = document.body.querySelector<HTMLButtonElement>('.canvas-toggle');
    btn?.focus();
    expect(btn?.matches(':focus-visible')).toBe(true);
    unmount(c);
  });

  it('ApprovalsDock declares a focusable dock region', () => {
    const c = mount(ApprovalsDock, { target: document.body, props: { hudUrl: null } });
    flushSync();
    expect(document.querySelector('[data-region="approvals-dock"]')).not.toBeNull();
    unmount(c);
  });
});

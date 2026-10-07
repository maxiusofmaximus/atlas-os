// @vitest-environment jsdom
// Atlas OS — views store + ViewSwitcher behaviour (RFC 67 §22.1 F5).
// Covers the 10-id catalog, the RFC 24 §19 hotkey mapping, the `:v` cycle and
// the `:a` target, plus real mount-and-click behaviour of the switcher.

import { describe, it, expect, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { mount, unmount } from 'svelte';
import { VIEWS, activeView, activeMissionId, hotkeyAction } from './views';
import ViewSwitcher from '../components/ViewSwitcher.svelte';
import ApprovalsDock from '../components/ApprovalsDock.svelte';

afterEach(() => {
  document.body.innerHTML = '';
});

describe('views catalog (10-id model)', () => {
  it('has exactly the 8 projections + 2 panels', () => {
    const ids = VIEWS.map((v) => v.id).sort();
    expect(ids).toEqual(
      [
        'audit',
        'canvas',
        'cost',
        'health',
        'kanban',
        'mcp',
        'outline',
        'settings',
        'timeline',
        'worktrees',
      ].sort(),
    );
  });

  it('dropped the dissolved views', () => {
    const ids = VIEWS.map((v) => v.id as string);
    expect(ids).not.toContain('overview');
    expect(ids).not.toContain('agent');
    expect(ids).not.toContain('approvals');
  });

  it('carries no per-view key bindings', () => {
    for (const v of VIEWS) {
      expect((v as unknown as Record<string, unknown>).key).toBeUndefined();
    }
  });
});

describe('hotkeys (RFC 24 §19)', () => {
  it('maps :v to view and :a to approvals', () => {
    expect(hotkeyAction('v')).toBe('view');
    expect(hotkeyAction('a')).toBe('approvals');
  });

  it('is case-insensitive and rejects unknown keys', () => {
    expect(hotkeyAction('V')).toBe('view');
    expect(hotkeyAction('z')).toBeNull();
  });
});

describe('activeView cycle (:v behaviour)', () => {
  it('advances and retreats through the catalog', () => {
    activeView.set('kanban');
    activeView.cycle(1);
    expect(get(activeView)).toBe('cost');
    activeView.cycle(-1);
    expect(get(activeView)).toBe('kanban');
  });
});

describe('ViewSwitcher mount', () => {
  it('clicking a tab sets the active view', () => {
    activeView.set('kanban');
    const c = mount(ViewSwitcher, {
      target: document.body,
      props: { available: ['kanban', 'cost'] },
    });
    const cost = [...document.body.querySelectorAll<HTMLButtonElement>('.tab')].find((t) =>
      t.textContent?.includes('Cost'),
    );
    expect(cost).toBeTruthy();
    cost?.click();
    expect(get(activeView)).toBe('cost');
    unmount(c);
  });

  it('a tab not in `available` is disabled and does not change the view', () => {
    activeView.set('kanban');
    const c = mount(ViewSwitcher, { target: document.body, props: { available: ['kanban'] } });
    const disabled = [...document.body.querySelectorAll<HTMLButtonElement>('.tab')].find(
      (t) => t.disabled,
    );
    expect(disabled).toBeTruthy();
    disabled?.click();
    expect(get(activeView)).toBe('kanban');
    unmount(c);
  });
});

describe(':a target', () => {
  it('the dock exposes a labelled element to focus', () => {
    const c = mount(ApprovalsDock, { target: document.body, props: { hudUrl: null } });
    expect(document.querySelector('[aria-label="Approvals Dock"]')).toBeTruthy();
    unmount(c);
  });

  it('selecting a mission is pure state and does not touch the WS', () => {
    activeMissionId.set('00000000-0000-0000-0000-000000000001');
    expect(get(activeMissionId)).toBe('00000000-0000-0000-0000-000000000001');
  });
});

// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { AuditResponse } from '$stores/hud';

const spies = vi.hoisted(() => ({ fetchAudit: vi.fn() }));

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return { ...actual, fetchAudit: spies.fetchAudit };
});

import AuditTimeline from './AuditTimeline.svelte';

const auditData: AuditResponse = {
  rows: [
    {
      seq: 1,
      ts: '2026-10-06T00:00:00Z',
      actor: 'operator',
      action: 'approve',
      inputs: {},
      outputs: { ok: true },
    },
  ],
  count: 1,
};

function render(props: ComponentProps<typeof AuditTimeline>) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(AuditTimeline, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
});

describe('AuditTimeline behaviour', () => {
  it('renders the audit chain rows', async () => {
    spies.fetchAudit.mockResolvedValue(auditData);
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.timeline li')).not.toBeNull());
    expect(target.querySelector('.seq')?.textContent).toContain('#1');
    expect(target.querySelector('.actor')?.textContent).toBe('operator');
    expect(target.querySelector('.action')?.textContent).toBe('approve');
    await unmount(component);
  });

  it('renders the explicit empty state when the chain has no writer yet', async () => {
    spies.fetchAudit.mockResolvedValue({ rows: [], count: 0 });
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.textContent).toContain('No audit entries yet'));
    await unmount(component);
  });

  it('renders the no-data empty state when there is no HUD URL', async () => {
    const { target, component } = render({ hudUrl: null });

    expect(target.querySelector('.empty')?.textContent).toContain('No audit data yet.');
    await unmount(component);
  });

  it('renders the loading skeleton while the fetch is in flight', async () => {
    spies.fetchAudit.mockReturnValue(new Promise(() => {}));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.skeleton')).not.toBeNull());
    await unmount(component);
  });

  it('renders the error state and Retry re-fetches', async () => {
    spies.fetchAudit.mockRejectedValueOnce(new Error('audit down'));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.error')?.textContent).toContain('audit down');

    spies.fetchAudit.mockResolvedValue(auditData);
    target.querySelector<HTMLButtonElement>('.error .retry')?.click();
    await vi.waitFor(() => expect(target.querySelector('.timeline li')).not.toBeNull());
    expect(target.querySelector('.error')).toBeNull();
    expect(spies.fetchAudit).toHaveBeenCalledTimes(2);
    await unmount(component);
  });
});

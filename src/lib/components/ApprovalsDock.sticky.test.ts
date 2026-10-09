// @vitest-environment jsdom
// Atlas OS — ApprovalsDock behaviour (FASE 13, G7): the dock renders. The
// always-visible (sticky) guarantee is verified by a real Playwright capture
// (docs/design/captures/g13-approvals-dock-{dark,light}.png), not a source/CSS
// string test.

import { describe, it, expect, afterEach } from 'vitest';
import { mount, unmount, flushSync } from 'svelte';
import ApprovalsDock from './ApprovalsDock.svelte';

afterEach(() => {
  document.body.innerHTML = '';
});

describe('ApprovalsDock — render', () => {
  it('renders the dock region', () => {
    const c = mount(ApprovalsDock, { target: document.body, props: { hudUrl: null } });
    flushSync();
    expect(document.querySelector('[data-region="approvals-dock"]')).not.toBeNull();
    unmount(c);
  });
});

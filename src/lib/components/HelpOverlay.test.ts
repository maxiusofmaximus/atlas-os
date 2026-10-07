// @vitest-environment jsdom
// Atlas OS — HelpOverlay behaviour (RFC 67 §24.5, criterion C5). Mounts the
// panel, drives open/close through a reactive host, and asserts the modal, the
// hotkey table, Esc/close/scrim and focus restoration.

import { describe, it, expect, vi, afterEach } from 'vitest';
import { mount, unmount, flushSync, tick } from 'svelte';
import HelpOverlay from './HelpOverlay.svelte';
import { helpHost } from './__help-host.svelte';

let app: ReturnType<typeof mount> | null = null;

async function settle(): Promise<void> {
  await new Promise((resolve) => setTimeout(resolve, 0));
  await tick();
  flushSync();
}

afterEach(() => {
  if (app) unmount(app);
  app = null;
  document.body.innerHTML = '';
});

describe('HelpOverlay', () => {
  it('opens a modal dialog listing the RFC 24 §19 hotkeys (C5)', async () => {
    const props = helpHost(true);
    app = mount(HelpOverlay, { target: document.body, props });
    await settle();
    const dialog = document.querySelector('[role="dialog"]');
    expect(dialog).not.toBeNull();
    expect(dialog?.getAttribute('aria-modal')).toBe('true');
    expect(document.body.textContent).toContain(':?');
    expect(document.body.textContent).toContain('Enfocar Mission Rail');
    expect(document.activeElement).toBe(document.querySelector('.close'));
  });

  it('Esc closes via onclose', async () => {
    const onclose = vi.fn();
    const props = helpHost(true, onclose);
    app = mount(HelpOverlay, { target: document.body, props });
    await settle();
    document
      .querySelector('[role="dialog"]')
      ?.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(onclose).toHaveBeenCalled();
  });

  it('the scrim click closes via onclose', async () => {
    const onclose = vi.fn();
    const props = helpHost(true, onclose);
    app = mount(HelpOverlay, { target: document.body, props });
    await settle();
    document
      .querySelector('.help-scrim')
      ?.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(onclose).toHaveBeenCalled();
  });

  it('restores focus to the invoker when closed (C5)', async () => {
    const trigger = document.createElement('button');
    document.body.appendChild(trigger);
    trigger.focus();
    const props = helpHost(true);
    app = mount(HelpOverlay, { target: document.body, props });
    await settle();
    expect(document.activeElement).toBe(document.querySelector('.close'));
    props.open = false;
    await settle();
    expect(document.activeElement).toBe(trigger);
  });
});

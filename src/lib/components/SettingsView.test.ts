// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { mount, unmount, type ComponentProps } from 'svelte';
import type { SecretsResponse } from '$stores/hud';

const spies = vi.hoisted(() => ({
  fetchSecretSlots: vi.fn(),
  postSecret: vi.fn(),
  deleteSecret: vi.fn(),
}));

vi.mock('$stores/hud', async (importOriginal) => {
  const actual = await importOriginal<typeof import('$stores/hud')>();
  return {
    ...actual,
    fetchSecretSlots: spies.fetchSecretSlots,
    postSecret: spies.postSecret,
    deleteSecret: spies.deleteSecret,
  };
});

import SettingsView from './SettingsView.svelte';

const secretsData: SecretsResponse = {
  service: 'atlas',
  slots: [
    { account: 'OPENAI_API_KEY', present: true },
    { account: 'ANTHROPIC_API_KEY', present: false },
  ],
};

function render(props: ComponentProps<typeof SettingsView>) {
  const target = document.createElement('div');
  document.body.appendChild(target);
  const component = mount(SettingsView, { target, props });
  return { target, component };
}

beforeEach(() => {
  vi.resetAllMocks();
  document.body.innerHTML = '';
});

describe('SettingsView behaviour', () => {
  it('renders the secret slots with their present state', async () => {
    spies.fetchSecretSlots.mockResolvedValue(secretsData);
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelectorAll('.slots li').length).toBe(2));
    expect(target.querySelector('.slots')?.textContent).toContain('OPENAI_API_KEY');
    expect(target.querySelectorAll('.state')[0]?.textContent).toContain('stored');
    expect(target.querySelectorAll('.state')[1]?.textContent).toContain('not set');
    await unmount(component);
  });

  it('renders the empty state when there is no HUD URL', async () => {
    const { target, component } = render({ hudUrl: null });

    expect(target.querySelector('.empty')?.textContent).toContain('No secret slots yet.');
    await unmount(component);
  });

  it('renders the loading skeleton while the fetch is in flight', async () => {
    spies.fetchSecretSlots.mockReturnValue(new Promise(() => {}));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.skeleton')).not.toBeNull());
    await unmount(component);
  });

  it('renders the error state and Reload re-fetches', async () => {
    spies.fetchSecretSlots.mockRejectedValueOnce(new Error('secrets down'));
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.error')).not.toBeNull());
    expect(target.querySelector('.error')?.textContent).toContain('secrets down');

    spies.fetchSecretSlots.mockResolvedValue(secretsData);
    target.querySelector<HTMLButtonElement>('.settings header button')?.click();
    await vi.waitFor(() => expect(target.querySelector('.slots')).not.toBeNull());
    expect(target.querySelector('.error')).toBeNull();
    await unmount(component);
  });

  it('submits a key to the OS keychain through postSecret', async () => {
    spies.fetchSecretSlots.mockResolvedValue(secretsData);
    spies.postSecret.mockResolvedValue(undefined);
    const { target, component } = render({ hudUrl: 'http://hud' });

    const keyInput = target.querySelectorAll<HTMLInputElement>('input')[1];
    if (!keyInput) throw new Error('key input missing');
    keyInput.value = 'sk-test';
    keyInput.dispatchEvent(new Event('input', { bubbles: true }));

    const form = target.querySelector('form');
    form?.dispatchEvent(new Event('submit', { bubbles: true, cancelable: true }));

    await vi.waitFor(() =>
      expect(spies.postSecret).toHaveBeenCalledWith('http://hud', 'OPENCODE_GO_KEY', 'sk-test'),
    );
    await unmount(component);
  });

  it('removes a stored key through deleteSecret', async () => {
    spies.fetchSecretSlots.mockResolvedValue(secretsData);
    spies.deleteSecret.mockResolvedValue(undefined);
    const { target, component } = render({ hudUrl: 'http://hud' });

    await vi.waitFor(() => expect(target.querySelector('.del')).not.toBeNull());
    target.querySelector<HTMLButtonElement>('.del')?.click();
    await vi.waitFor(() =>
      expect(spies.deleteSecret).toHaveBeenCalledWith('http://hud', 'OPENAI_API_KEY'),
    );
    await unmount(component);
  });
});

// OpenCode OS — tests for the HUD Svelte store.
// Phase 0: verifies initial state shape and the `toWsUrl` canonicaliser
// that the desktop shell and any future remote-HUD embedder (RFC 24 §16)
// depend on.
import { describe, it, expect } from 'vitest';
import { get } from 'svelte/store';
import { hud, toWsUrl } from './hud';

describe('hud store', () => {
  it('starts disconnected', () => {
    const state = get(hud);
    expect(state.connected).toBe(false);
    expect(state.events).toHaveLength(0);
    expect(state.url).toBeNull();
  });
});

describe('toWsUrl', () => {
  it('converts http URL with trailing slash to ws URL ending in /ws', () => {
    expect(toWsUrl('http://localhost:57457/')).toBe('ws://localhost:57457/ws');
  });

  it('converts https URL to wss URL and preserves port', () => {
    expect(toWsUrl('https://127.0.0.1:8443/')).toBe('wss://127.0.0.1:8443/ws');
  });

  it('strips trailing slash and does not double-append /ws', () => {
    expect(toWsUrl('http://localhost:57457/ws')).toBe('ws://localhost:57457/ws');
  });

  it('is idempotent: applying twice yields the same value', () => {
    const once = toWsUrl('http://localhost:57457/');
    expect(toWsUrl(once)).toBe('ws://localhost:57457/ws');
  });
});

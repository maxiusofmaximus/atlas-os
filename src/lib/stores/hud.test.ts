// OpenCode OS — tests for the HUD Svelte store.
// Phase 0: verifies initial state shape and the `toWsUrl` canonicaliser
// that the desktop shell and any future remote-HUD embedder (RFC 24 §16)
// depend on.
// Phase 1: also pins `fetchTail` URL/query construction so the Mission
// Control tail boxes hit the right axum route.
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { hud, toWsUrl, fetchTail, type TailKind } from './hud';

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

describe('fetchTail', () => {
  const fetchMock = vi.fn();
  const ok = (body: unknown) => ({
    ok: true,
    status: 200,
    json: async () => body,
  });

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('requests /tail/<kind> with no query when last is omitted', async () => {
    fetchMock.mockResolvedValueOnce(ok([{ id: 'abc' }]));
    const rows = await fetchTail('http://localhost:57457/', 'journal');
    expect(rows).toEqual([{ id: 'abc' }]);
    expect(fetchMock).toHaveBeenCalledWith('http://localhost:57457/tail/journal');
  });

  it('appends ?last=N when last is provided', async () => {
    fetchMock.mockResolvedValueOnce(ok([]));
    await fetchTail('http://127.0.0.1:9999', 'patterns', 50);
    expect(fetchMock).toHaveBeenCalledWith('http://127.0.0.1:9999/tail/patterns?last=50');
  });

  it('strips a trailing slash on hudUrl before joining', async () => {
    fetchMock.mockResolvedValueOnce(ok([]));
    await fetchTail('http://localhost:57457/', 'plans');
    expect(fetchMock).toHaveBeenCalledWith('http://localhost:57457/tail/plans');
  });

  it('throws when the server returns a non-OK status', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ERR' });
    await expect(fetchTail('http://x/', 'missions')).rejects.toThrow();
  });

  it('covers every TailKind route name', async () => {
    const kinds: TailKind[] = [
      'journal',
      'missions',
      'verdicts',
      'consolidated',
      'plans',
      'diffs',
      'validation_reports',
      'repairs',
      'patterns',
      'checkpoints',
      'skills',
    ];
    for (const k of kinds) {
      fetchMock.mockResolvedValueOnce(ok([]));
      await fetchTail('http://h/', k);
    }
    expect(fetchMock).toHaveBeenCalledTimes(kinds.length);
    const urls = fetchMock.mock.calls.map((c) => c[0] as string);
    expect(new Set(urls).size).toBe(kinds.length);
  });
});

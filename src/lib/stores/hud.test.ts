// OpenCode OS — tests for the HUD Svelte store.
// Phase 0: verifies initial state shape and the `toWsUrl` canonicaliser
// that the desktop shell and any future remote-HUD embedder (RFC 24 §16)
// depend on.
// Phase 1: also pins `fetchTail` URL/query construction so the Mission
// Control tail boxes hit the right axum route.
import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import { get } from 'svelte/store';
import { hud, toWsUrl, fetchTail, fetchAnnotations, postAnnotation, type TailKind } from './hud';

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

describe('fetchAnnotations', () => {
  const fetchMock = vi.fn();
  const ok = (body: unknown) => ({ ok: true, status: 200, json: async () => body });

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('GETs /diff/<id>/annotation and returns rows', async () => {
    const ann = [{ id: 'a1', diff_id: 'd1', body: 'hi', author: 'max' }];
    fetchMock.mockResolvedValueOnce(ok(ann));
    const rows = await fetchAnnotations('http://localhost:57457/', 'd1');
    expect(rows).toEqual(ann);
    expect(fetchMock).toHaveBeenCalledWith('http://localhost:57457/diff/d1/annotation');
  });

  it('throws on non-OK status', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ERR' });
    await expect(fetchAnnotations('http://x/', 'd1')).rejects.toThrow();
  });
});

describe('postAnnotation', () => {
  const fetchMock = vi.fn();

  beforeEach(() => {
    fetchMock.mockReset();
    vi.stubGlobal('fetch', fetchMock);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
  });

  it('POSTs minimal body with body+author only', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'a1', diff_id: 'd1', created_at: 'now' }),
    });
    const posted = await postAnnotation('http://localhost:57457', 'd1', {
      body: 'note',
      author: 'max',
    });
    expect(posted.id).toBe('a1');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const call = fetchMock.mock.calls[0]!;
    expect(call[0]).toBe('http://localhost:57457/diff/d1/annotation');
    const init = call[1] as RequestInit;
    expect(init.method).toBe('POST');
    const payload = JSON.parse(String(init.body)) as Record<string, unknown>;
    expect(payload.body).toBe('note');
    expect(payload.author).toBe('max');
    expect('file_path' in payload).toBe(false);
    expect('line_no' in payload).toBe(false);
  });

  it('includes file_path and line_no when provided', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'a2', diff_id: 'd1', created_at: 'now' }),
    });
    await postAnnotation('http://h', 'd1', {
      body: 'fix',
      author: 'max',
      file_path: 'src/lib.rs',
      line_no: 42,
    });
    const init = fetchMock.mock.calls[0]![1] as RequestInit;
    const payload = JSON.parse(String(init.body)) as Record<string, unknown>;
    expect(payload.file_path).toBe('src/lib.rs');
    expect(payload.line_no).toBe(42);
  });

  it('omits file_path when empty string', async () => {
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'a3', diff_id: 'd1', created_at: 'now' }),
    });
    await postAnnotation('http://h', 'd1', {
      body: 'fix',
      author: 'max',
      file_path: '   ',
    });
    const init = fetchMock.mock.calls[0]![1] as RequestInit;
    const payload = JSON.parse(String(init.body)) as Record<string, unknown>;
    expect('file_path' in payload).toBe(false);
  });

  it('throws on non-OK status', async () => {
    fetchMock.mockResolvedValueOnce({ ok: false, status: 422, statusText: 'UP' });
    await expect(postAnnotation('http://h', 'd1', { body: 'x', author: 'y' })).rejects.toThrow();
  });
});

// ────────────── RFC 28 §A — Autoresearch telemetry helpers ──────────────
import { postAutoresearchCancel, type AutoresearchSnapshot } from './hud';

describe('postAutoresearchCancel', () => {
  beforeEach(() => {
    vi.resetModules();
    globalThis.fetch = vi.fn() as unknown as typeof fetch;
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('posts to /autoresearch/cancel and resolves on 200', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: true, status: 200 } as Response);
    await expect(
      postAutoresearchCancel('http://h/', {
        run_id: '00000000-0000-0000-0000-000000000abc',
        outcome: 'aborted',
      }),
    ).resolves.toBeUndefined();
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const args = fetchMock.mock.calls[0]!;
    const [url, init] = args;
    expect(String(url)).toBe('http://h/autoresearch/cancel');
    expect(init?.method).toBe('POST');
  });

  it('accepts 204 No Content (canonical stub response)', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: true, status: 204 } as Response);
    await expect(
      postAutoresearchCancel('http://h', { run_id: 'x', outcome: 'aborted' }),
    ).resolves.toBeUndefined();
  });

  it('rejects on 4xx validation error (400/422)', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 400, statusText: 'BR' } as Response);
    await expect(
      postAutoresearchCancel('http://h', { run_id: 'x', outcome: 'aborted' }),
    ).rejects.toThrow(/400/);
  });

  it('rejects on 5xx (real error, not "not wired")', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ISE' } as Response);
    await expect(
      postAutoresearchCancel('http://h', { run_id: 'x', outcome: 'aborted' }),
    ).rejects.toThrow(/500/);
  });
});

describe('AutoresearchSnapshot type', () => {
  it('accepts canonical shape produced by the Rust supervisor', () => {
    const snap: AutoresearchSnapshot = {
      id: '00000000-0000-0000-0000-000000000abc',
      mission_id: '00000000-0000-0000-0000-000000000001',
      baseline_metric: 1.0,
      best_metric: 0.95,
      git_sha_start: 'abc1234',
      git_sha_end: 'def5678',
      metric_command: "rg -c 'error' src",
      max_steps: 50,
      timebox_seconds: 600,
      step_count: 12,
      outcome: 'running',
      ts_started: 1722000000,
      ts_ended: null,
    };
    expect(snap.outcome).toBe('running');
    expect(snap.best_metric).toBeLessThan(snap.baseline_metric);
  });
});

// ────────────── RFC 28 §C item 7 — graph view client ──────────────
import {
  fetchGraph,
  type MissionGraph,
  type GraphNode,
  type GraphEdge,
  type Provenance,
  type NodeKind,
  type EdgeKind,
} from './hud';

describe('fetchGraph', () => {
  beforeEach(() => {
    vi.resetModules();
    globalThis.fetch = vi.fn() as unknown as typeof fetch;
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  const sampleGraph: MissionGraph = {
    mission_id: 'mission-1',
    nodes: [
      {
        id: 'mission-1:root',
        mission_id: 'mission-1',
        kind: 'mission',
        label: 'Root mission',
        provenance: 'EXTRACTED',
        attrs_json: '{}',
      },
    ],
    edges: [
      {
        id: 'mission-1:e1',
        mission_id: 'mission-1',
        src: 'mission-1:root',
        dst: 'mission-1:s1',
        kind: 'calls',
        precondition: null,
        guard: null,
        visit_count: 0,
      },
    ],
  };

  it('requests GET /graph/<id> and returns parsed MissionGraph on 200', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => sampleGraph,
    } as Response);
    const got = await fetchGraph('http://localhost:57457', 'mission-1');
    expect(got).toEqual(sampleGraph);
    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:57457/graph/mission-1',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('trims a trailing slash from hudUrl before appending /graph', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => sampleGraph,
    } as Response);
    await fetchGraph('http://localhost:57457/', 'mission-1');
    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:57457/graph/mission-1',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('URL-encodes the mission id when it contains a slash', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => sampleGraph,
    } as Response);
    await fetchGraph('http://localhost:57457', 'profile/m1');
    expect(fetchMock).toHaveBeenCalledWith(
      'http://localhost:57457/graph/profile%2Fm1',
      expect.objectContaining({ method: 'GET' }),
    );
  });

  it('rejects with a descriptive message on 404', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 404,
      statusText: 'Not Found',
      json: async () => sampleGraph,
    } as Response);
    await expect(fetchGraph('http://localhost:57457', 'mission-404')).rejects.toThrow(
      /No graph persisted for mission mission-404/,
    );
  });

  it('rejects with a generic message on 5xx', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: false,
      status: 500,
      statusText: 'ISE',
      json: async () => sampleGraph,
    } as Response);
    await expect(fetchGraph('http://localhost:57457', 'mission-1')).rejects.toThrow(/500/);
  });
});

describe('MissionGraph type contract', () => {
  it('accepts the payload shape emitted by GET /graph/:id', () => {
    const node: GraphNode = {
      id: 'm1:n1',
      mission_id: 'm1',
      kind: 'engine_state',
      label: 'Planning',
      provenance: 'INFERRED',
      attrs_json: '{"k":"v"}',
    };
    const edge: GraphEdge = {
      id: 'm1:e1',
      mission_id: 'm1',
      src: 'm1:n1',
      dst: 'm1:n2',
      kind: 'transitions_to',
      precondition: 'step_done',
      guard: 'has_steps',
      visit_count: 3,
    };
    const graph: MissionGraph = {
      mission_id: 'm1',
      nodes: [node],
      edges: [edge],
    };
    expect(graph.nodes).toHaveLength(1);
    expect(graph.edges[0]?.visit_count).toBe(3);
    const p: Provenance = node.provenance;
    const k: NodeKind = node.kind;
    const ek: EdgeKind = edge.kind;
    expect(p).toBe('INFERRED');
    expect(k).toBe('engine_state');
    expect(ek).toBe('transitions_to');
  });
});

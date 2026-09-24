// Atlas OS — tests for the HUD Svelte store.
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
    const call = fetchMock.mock.calls[0] ?? [];
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
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
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
    const init = fetchMock.mock.calls[0]?.[1] as RequestInit;
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
    const args = fetchMock.mock.calls[0] ?? [];
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

// ────────────── RFC 28 §H.3 / §H.4 — reset-window card helpers ──────────────
import {
  postProfileSwitch,
  postMissionResume,
  type SpendLimitErrorCardPayload,
  type ModelReadyCardPayload,
} from './hud';

describe('postProfileSwitch', () => {
  beforeEach(() => {
    vi.resetModules();
    globalThis.fetch = vi.fn() as unknown as typeof fetch;
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('POSTs /profile/switch with backup_profile_id and resolves on 200', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ ok: true, new_profile_id: 'personal' }),
    } as Response);
    const got = await postProfileSwitch('http://h', 'personal');
    expect(got.new_profile_id).toBe('personal');
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(String(url)).toBe('http://h/profile/switch');
    expect(init?.method).toBe('POST');
    const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
    expect(body.backup_profile_id).toBe('personal');
  });

  it('rejects with descriptive error when hudUrl is null', async () => {
    await expect(postProfileSwitch(null, 'personal')).rejects.toThrow(/unavailable/i);
  });

  it('rejects on 5xx', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ISE' } as Response);
    await expect(postProfileSwitch('http://h', 'personal')).rejects.toThrow(/500/);
  });
});

describe('postMissionResume', () => {
  beforeEach(() => {
    vi.resetModules();
    globalThis.fetch = vi.fn() as unknown as typeof fetch;
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('POSTs /mission/resume with mission_id and resolves on 200', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ ok: true, mission_id: 'm1' }),
    } as Response);
    const got = await postMissionResume('http://h', 'm1');
    expect(got.ok).toBe(true);
    expect(fetchMock).toHaveBeenCalledTimes(1);
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(String(url)).toBe('http://h/mission/resume');
    expect(init?.method).toBe('POST');
    const body = JSON.parse(String(init?.body)) as Record<string, unknown>;
    expect(body.mission_id).toBe('m1');
  });

  it('rejects when hudUrl is null', async () => {
    await expect(postMissionResume(null, 'm1')).rejects.toThrow(/unavailable/i);
  });

  it('rejects on 4xx', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 404, statusText: 'NF' } as Response);
    await expect(postMissionResume('http://h', 'm1')).rejects.toThrow(/404/);
  });
});

describe('SpendLimitErrorCardPayload type', () => {
  it('accepts the canonical shape emitted by the Rust bus event', () => {
    const p: SpendLimitErrorCardPayload = {
      provider: 'anthropic',
      model: 'claude-3-5-sonnet',
      status_code: 429,
      error_type: 'rate_limit',
      resets_at: '2026-08-03T13:00:00Z',
      request_id: 'req_1',
      toast_enqueued_id: 42,
    };
    expect(p.error_type).toBe('rate_limit');
    expect(p.toast_enqueued_id).toBe(42);
  });
});

describe('ModelReadyCardPayload type', () => {
  it('accepts the canonical shape emitted by the §F scheduler', () => {
    const p: ModelReadyCardPayload = {
      provider: 'anthropic',
      model: 'claude-3-5-sonnet',
      resets_at: '2026-08-03T13:00:00Z',
      mission_id: '00000000-0000-0000-0000-00000000abc',
      toast_queue_id: 99,
    };
    expect(p.toast_queue_id).toBe(99);
    expect(p.mission_id).not.toBeNull();
  });

  it('allows null mission_id for orphan model-ready toast', () => {
    const p: ModelReadyCardPayload = {
      provider: 'openai',
      model: 'gpt-5',
      resets_at: '2026-08-03T14:00:00Z',
      mission_id: null,
      toast_queue_id: 5,
    };
    expect(p.mission_id).toBeNull();
  });
});

// ────────────── RFC 31 §B 4.5 — Swarm Console store ──────────────
import {
  projectSwarmAgents,
  projectSwarmInbox,
  countUnread,
  swarmStateColor,
  fetchSwarmAgents,
  fetchSwarmInbox,
  postSwarmSend,
  fetchSwarmChecks,
  SWARM_EVENT_KINDS,
  type HudEvent,
  type SwarmAgent,
  type SwarmMessage,
  type SwarmAgentState,
} from './hud';

const swarmAgent = (over: Partial<SwarmAgent> = {}): SwarmAgent => ({
  agent_id: '11111111-2222-3333-4444-555555555555',
  mission_id: 'mission-1',
  role: 'backend',
  model_id: 'atlas-weak',
  state: 'working',
  worktree_path: '~/.opencode/worktrees/mission-1/backend',
  updated_at: '2026-09-24T00:00:00Z',
  ...over,
});

const swarmMessage = (over: Partial<SwarmMessage> = {}): SwarmMessage => ({
  id: 'msg-1',
  from_agent: 'planner-id',
  to_agent: 'backend-id',
  body: 'implement the API',
  read_at: null,
  created_at: '2026-09-24T00:01:00Z',
  ...over,
});

const busEvt = (id: string, kind: string, payload: unknown): HudEvent => ({
  id,
  ts: '2026-09-24T00:00:00Z',
  kind,
  payload,
});

describe('SWARM_EVENT_KINDS', () => {
  it('declares the three Kernel Bus broadcast kinds', () => {
    expect([...SWARM_EVENT_KINDS]).toEqual([
      'swarm_agent_spawned',
      'swarm_message',
      'swarm_state_changed',
    ]);
  });
});

describe('swarmStateColor', () => {
  it('maps every agent state to a colour token', () => {
    const cases: Array<[SwarmAgentState, string]> = [
      ['spawned', 'grey'],
      ['idle', 'grey'],
      ['working', 'blue'],
      ['waiting_review', 'amber'],
      ['blocked', 'red'],
      ['done', 'green'],
      ['failed', 'red'],
    ];
    for (const [state, color] of cases) {
      expect(swarmStateColor(state)).toBe(color);
    }
  });
});

describe('projectSwarmAgents', () => {
  it('folds spawns into one desk per agent_id', () => {
    const events = [
      busEvt('e1', 'swarm_agent_spawned', { agent: swarmAgent() }),
      busEvt('e2', 'swarm_agent_spawned', {
        agent: swarmAgent({ agent_id: 'other-id', role: 'reviewer' }),
      }),
    ];
    const desks = projectSwarmAgents(events);
    expect(desks).toHaveLength(2);
    expect(desks.map((d) => d.role).sort()).toEqual(['backend', 'reviewer']);
  });

  it('patches state on swarm_state_changed', () => {
    const events = [
      busEvt('e1', 'swarm_agent_spawned', { agent: swarmAgent({ state: 'working' }) }),
      busEvt('e2', 'swarm_state_changed', {
        agent_id: '11111111-2222-3333-4444-555555555555',
        mission_id: 'mission-1',
        state: 'done',
        updated_at: '2026-09-24T00:05:00Z',
      }),
    ];
    const desks = projectSwarmAgents(events);
    expect(desks).toHaveLength(1);
    expect(desks[0]?.state).toBe('done');
    expect(desks[0]?.updated_at).toBe('2026-09-24T00:05:00Z');
  });

  it('last spawn wins for a respawned agent_id', () => {
    const events = [
      busEvt('e1', 'swarm_agent_spawned', { agent: swarmAgent({ model_id: 'old' }) }),
      busEvt('e2', 'swarm_agent_spawned', { agent: swarmAgent({ model_id: 'new' }) }),
    ];
    const desks = projectSwarmAgents(events);
    expect(desks).toHaveLength(1);
    expect(desks[0]?.model_id).toBe('new');
  });

  it('ignores state changes for unknown agents and malformed payloads', () => {
    const events = [
      busEvt('e1', 'swarm_state_changed', {
        agent_id: 'ghost',
        mission_id: 'mission-1',
        state: 'done',
        updated_at: 'now',
      }),
      busEvt('e2', 'swarm_agent_spawned', { agent: null }),
      busEvt('e3', 'journal', { whatever: true }),
    ];
    expect(projectSwarmAgents(events)).toEqual([]);
  });
});

describe('projectSwarmInbox', () => {
  it('returns only messages addressed to the agent, in order', () => {
    const events = [
      busEvt('e1', 'swarm_message', { message: swarmMessage({ id: 'm1' }) }),
      busEvt('e2', 'swarm_message', {
        message: swarmMessage({ id: 'm2', to_agent: 'someone-else' }),
      }),
      busEvt('e3', 'swarm_message', { message: swarmMessage({ id: 'm3' }) }),
    ];
    const inbox = projectSwarmInbox(events, 'backend-id');
    expect(inbox.map((m) => m.id)).toEqual(['m1', 'm3']);
  });

  it('returns empty for a desk with no mail', () => {
    expect(projectSwarmInbox([], 'backend-id')).toEqual([]);
  });
});

describe('countUnread', () => {
  it('counts messages with null read_at', () => {
    const box = [
      swarmMessage({ id: 'm1' }),
      swarmMessage({ id: 'm2', read_at: '2026-09-24T00:02:00Z' }),
    ];
    expect(countUnread(box)).toBe(1);
  });
});

describe('swarm REST helpers', () => {
  beforeEach(() => {
    globalThis.fetch = vi.fn() as unknown as typeof fetch;
  });
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('fetchSwarmAgents GETs /swarm/<mission>/agents', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [swarmAgent()],
    } as Response);
    const got = await fetchSwarmAgents('http://h/', 'mission-1');
    expect(got).toHaveLength(1);
    expect(fetchMock).toHaveBeenCalledWith('http://h/swarm/mission-1/agents');
  });

  it('fetchSwarmInbox GETs /swarm/inbox/<agent>', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [swarmMessage()],
    } as Response);
    const got = await fetchSwarmInbox('http://h', 'backend-id');
    expect(got).toHaveLength(1);
    expect(fetchMock).toHaveBeenCalledWith('http://h/swarm/inbox/backend-id');
  });

  it('postSwarmSend POSTs the envelope and returns the id', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => ({ id: 'msg-9' }),
    } as Response);
    const got = await postSwarmSend('http://h', {
      from_agent: 'a',
      to_agent: 'b',
      body: 'go',
    });
    expect(got.id).toBe('msg-9');
    const [url, init] = fetchMock.mock.calls[0] ?? [];
    expect(String(url)).toBe('http://h/swarm/send');
    expect(init?.method).toBe('POST');
    expect(JSON.parse(String(init?.body))).toEqual({ from_agent: 'a', to_agent: 'b', body: 'go' });
  });

  it('fetchSwarmChecks GETs /swarm/<mission>/<agent>/checks', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({
      ok: true,
      status: 200,
      json: async () => [{ name: 'clippy', status: 'pass', detail: null }],
    } as Response);
    const got = await fetchSwarmChecks('http://h/', 'mission-1', 'backend-id');
    expect(got[0]?.status).toBe('pass');
    expect(fetchMock).toHaveBeenCalledWith('http://h/swarm/mission-1/backend-id/checks');
  });

  it('rejects on non-OK status (failure path)', async () => {
    const fetchMock = vi.mocked(globalThis.fetch);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ISE' } as Response);
    await expect(fetchSwarmAgents('http://h', 'mission-1')).rejects.toThrow(/500/);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 404, statusText: 'NF' } as Response);
    await expect(fetchSwarmInbox('http://h', 'ghost')).rejects.toThrow(/404/);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 422, statusText: 'UP' } as Response);
    await expect(
      postSwarmSend('http://h', { from_agent: 'a', to_agent: 'b', body: 'x' }),
    ).rejects.toThrow(/422/);
    fetchMock.mockResolvedValueOnce({ ok: false, status: 500, statusText: 'ISE' } as Response);
    await expect(fetchSwarmChecks('http://h', 'm', 'a')).rejects.toThrow(/500/);
  });
});

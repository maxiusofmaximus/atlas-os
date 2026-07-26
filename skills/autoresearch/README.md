# autoresearch — Karpathy hill-climbing greedy loop (RFC 28 §A)

This skill wraps Karpathy's `autoresearch/program.md` experiment pattern so OpenCode OS can run it as a typed mission option (`opencode mission new --autoresearch --metric "<deterministic command>"`).

## License & provenance

`program.md` is a **verbatim copy** of `karpathy/autoresearch/program.md` (MIT License, Copyright Andrej Karpathy). The attribution header at the top of that file must be preserved on every redistribution.

`NEVER_STOP.md` quotes the two governing guards verbatim from `program.md` so other skills can cite them without re-quoting the full document.

## Skill contract

### Inputs

- `mission_id` (string) — the Mission this autoresearch run is bound to.
- `metric_command` (string, required) — a shell command whose stdout parses to a single `f64` (lower is better). Must be deterministic. Example: `rg -c 'error' src-tauri/ | wc -l`.
- `max_steps` (integer, default 50, hard ceiling 200) — ceiling on candidate count per run (RFC 28 §A doom-loop safeguard).
- `timebox_seconds` (integer, default 300 = 5min, max 600) — per-candidate wall-clock budget.
- `git_branch_prefix` (string, default `autoresearch/`) — the run branch.

### Outputs (persisted to Journal — schema M13)

- One `autoresearch_runs` row per mission run, with `outcome` ∈ `{running, improved, plateau, timeout, aborted}`.
- N `autoresearch_candidates` rows (one per step): `(step, git_sha, diff_hunk, metric_baseline_at_step, metric_after, kept, rationale)`.
- HUD telemetry via Kernel Bus channel `autoresearch:<run_id>`. `AutoresearchCard.svelte` renders baseline → best → step → sparkline → Pause/Stop.

### Behavior (supervisor-side, NOT prompt-side)

The guards in `program.md` belong in code, not in the agent's prompt:

1. Supervisor reads `autoresearch_runs.metric_command`, spawns a subshell to evaluate it on the current HEAD → `baseline_metric`.
2. Loop until `step_count == max_steps` OR `timebox_seconds` exhausted:
   - Call Coder subagent to propose an edit to the in-scope files. Coder must produce a unified diff or `null`.
   - `git apply` the diff; commit.
   - Evaluate `metric_command` again → `metric_after`.
   - If `metric_after < metric_baseline_at_step` → keep; advance `metric_baseline_at_step = metric_after`. Else → `git reset --hard HEAD~1`, kept=0.
   - Persist candidate row.
3. Streak detection: 3 consecutive reverts → `outcome=plateau`. Otherwise at exit: improved if any kept, else plateau.

The agent is never asked "should we keep going". The supervisor decides.

## Files

- `program.md` — verbatim source (MIT, Karpathy).
- `NEVER_STOP.md` — extract of the two governing guards, with a table mapping them to Rust invariants in the supervisor.
- `skill.toml` — skill manifest (RFC 23 §7.2 format).

---

> This skill is the data contract for `src-tauri/src/supervisor/loop.rs::Autoresearch` branch (RFC 28 §A). Phase 1.5b implements it.

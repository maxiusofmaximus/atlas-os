# research/59 — Fase 37: Swarm merger de `Diff`s + sharding

- **Fecha:** 2026-10-03
- **Estado:** **COMPLETA** (v37.0).
- **Contexto:** Fase 26 dejó el coding loop de un solo modelo. El Swarm (RFC 05) aporta
  agentes paralelos por archivo (file locks) y un Merger que fusiona sus diffs.

## 1. Hallazgo

`swarm/pool.rs` ya implementa pool/roles/topología:
- `SwarmRunner::{run_one, run_parallel, run_topology}` (5 fases: Planner → Researcher/Architect
  → ejecutores → Reviewer → Merger).
- `FileLockRegistry` (un archivo, un agente), `BackPressure`, checkpoints por agente,
  `rebase_after_merge`.
- Presets (10 roles), worktrees.

El gap real: **no había el Merger de diffs** (RFC 05 §4: "el Merger aplica en orden
determinístico") ni una forma de **repartir los steps** del plan entre varios agentes.

## 2. Decisión

- Kernel **puro** `swarm/merge.rs::merge_diffs`: toma N `Diff`s y produce uno.
  - Orden **determinista**: por `agent_id` ascendente; dentro de un archivo, los hunks en el
    orden del diff del agente.
  - Agrupa por `path`; **conflicto** = dos agentes reclaman el **mismo rango semiabierto**
    (`old_start`,`old_end`) del mismo archivo → se **reporta** (`MergeConflict`), no se
    resuelve en silencio. Rangos distintos del mismo archivo conviven.
- `orchestrator/execute.rs::plan_steps(steps, n)` — shard round-robin clampeado
  (`1..=steps.len()`), y `merge_coding_diffs(&[CodingStepOutcome])` que delega en `merge_diffs`.

## 3. Sub-fase

- **v37.0 COMPLETA** — `swarm/merge.rs` (`merge_diffs`, `MergeOutcome`, `MergeConflict`),
  `swarm/mod.rs` re-exports, `orchestrator/execute.rs` (`plan_steps`, `merge_coding_diffs`),
  `orchestrator/mod.rs` re-exports. 5 tests (disjoint files, distintos rangos, mismo rango →
  conflicto, determinismo entre órdenes de entrada, sharding). **F37 CERRADA.**

## 4. Diferido (deliberado)

- Ejecutar el shard en el `SwarmRunner` real (registrar agentes, locks por archivo, backpressure)
  desde el CLI: la composición pura ya existe; el wiring a `run_parallel` con worktrees es
  posterior (requiere decidir mapping step→archivos).
- Reviewer automático entre merge y validación: la Validation Engine ya cumple ese rol para el
  diff fusionado.

## 5. Fuentes

- RFC 05 §4 (protocolo de merge) y §5 (pool swarm estilo Kimi).
- `swarm/pool.rs` (`run_parallel`/`run_topology`, `FileLockRegistry`).
- research/55 (Fase 26), research/58 (Fase 36).

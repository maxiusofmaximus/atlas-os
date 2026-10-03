# 46 - Phase 20: journal refactor (steps 1-4)

## Objetivo

Dividir el módulo gigante `journal` sin romper tests. Esta fase es **step 1**:
mover el impl block `model_reset_upsert`/`link_model_reset_toast` a un módulo adjunto
con API pública equivalente.

## Implementación

- `journal/mod.rs`: de 2356 a ~2050 líneas — cortado el impl block `// Section H` (modelo reset-window helpers) y declarar `pub mod model_resets_ops;`.
- Nuevo `journal/model_resets_ops.rs` contiene ese impl block y `use crate::journal::*;`.
- API pública estable: los métodos siguen siendo Inherent Methods sobre `Journal`,
  visibles desde `crate::journal::model_resets_ops` (no se requirió cambiar callers).

## Verificación

- `cargo check`: OK.
- `cargo test --lib journal`: **228 passed; 0 failed** (red preservada).
- `cargo fmt --check`: OK (tras `cargo fmt`).

## Step 2 — dag_mode_ops (HECHO)

- `impl Journal` del bloque `dag_mode` (`persist_learning_graph`,
  `retrieve_similar_learning_graphs`, `read_mission_graph`, `seed_test_graph_node`)
  → `journal/dag_mode_ops.rs` (`8dcc148`).

## Step 3 — model_invocation (HECHO)

- `ModelInvocationRow` + `read_model_invocation_means`/`record_model_invocation`
  extraídos a `journal/model_invocation.rs`; `mod.rs` re-exporta
  `pub use model_invocation::ModelInvocationRow;`. API pública y callers intactos.
- `model_resets_ops.rs` queda sólo con el dominio de resets/afinidad.

## Step 4 — tests por dominio (HECHO)

- `journal/tests.rs` (2498 LOC, 99 tests) partido en `journal/tests/`:
  `tests/mod.rs` declara los 15 antiguos módulos `*_tests` (journal_store,
  verdict, plan, diff, validation, repair, pattern, checkpoint, skill,
  mission_graph_schema, model_resets_schema, research_m25/m26/m27_schema,
  journal_phase80), cada uno en su propio archivo.
- Split puramente mecánico: misma ruta de módulo
  (`crate::journal::tests::<name>`), mismos imports `crate::…`, cero cambios
  de cuerpo.

## Verificación (steps 1-4)

- `cargo test --lib journal`: **228 passed; 0 failed**.
- `cargo test --lib`: **1106 passed; 0 failed**.
- `cargo check` (default y `--features dag_mode`): OK.
- `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`: OK.

## Estado

Phase 20 (journal refactor) queda **CERRADA**: `mod.rs` sin los impl blocks
de resets/dag_mode/model_invocation y sin el blob de tests; red de 228 tests
journal intacta.

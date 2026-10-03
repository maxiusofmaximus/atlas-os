# 46 - Phase 20: journal refactor (steps 1-3)

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

## Verificación (steps 1-3)

- `cargo test --lib journal`: **228 passed; 0 failed**.
- `cargo check` (default y `--features dag_mode`): OK.
- `cargo fmt --check` + `cargo clippy --all-targets -- -D warnings`: OK.

## Siguiente (Phase 20.3)

- Dividir `journal/tests.rs` por dominio (985 LOC, aislado del código de producción).

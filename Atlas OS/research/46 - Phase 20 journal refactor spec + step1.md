# 46 - Phase 20: journal refactor step 1 (model_resets_ops)

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

## Siguiente (Phase 20.1+)

- Mover el impl block `dag_mode` (feature `dag_mode`) a su submódulo.
- Dividir `ModelInvocationRow` + record/read path a `journal/model_invocation.rs`.
- Dividir `journal/tests.rs` por dominio cuando haya capacidad.

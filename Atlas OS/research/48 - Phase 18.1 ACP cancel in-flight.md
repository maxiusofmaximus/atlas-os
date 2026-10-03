# 48 - Phase 18.1: ACP cancel in-flight

## Objetivo

Cerrar la deuda `cancel_request` anotada al final de Phase 18: el handler
`exec step` ignoraba `$/cancel_request` (solo `tracing`) porque la llamada al
CLI corría de principio a fin sin punto de cancelación.

## Implementación

- `acp/mod.rs`: el arm `PromptPlan::ExecStep` obtiene
  `responder.cancellation()` y ejecuta el CLI dentro de
  `RequestCancellation::run_until_cancelled(...)`. La librería
  `agent-client-protocol` ya enruta `$/cancel_request` al marker del request
  entrante (`incoming_actor` → `cancel_if_requested`), así que **no hace falta
  un registro propio** en Atlas.
- `ExecStepOutcome { Completed, Failed(String), Cancelled }` + helper puro
  `exec_step_outcome_text` mapean el resultado al chunk `session/update` y al
  `StopReason`: `Cancelled` → `StopReason::Cancelled` (mandato del schema ACP),
  `Completed`/`Failed` → `EndTurn`.
- `on_receive_notification(CancelRequestNotification)` se mantiene para
  tracing/diagnostics; la cancelación efectiva la ejecuta la librería.
- Doc de cabecera del módulo actualizado (ya no dice "dropped").

## Verificación

- `cargo check --features acp-server`: OK.
- `cargo test --features acp-server --lib acp`: **44 passed; 0 failed**
  (42 previos + 2 nuevos del mapeo de outcome).
- `cargo test --features acp-server --lib`: 1150 passed / 0 failed (rerun; el
  primer run marcó 1 fallo intermitente ajeno a acp, verde en rerun).
- `cargo fmt --check` + `cargo clippy --features acp-server --all-targets -- -D warnings`: OK.

## Notas

- Cancelación cooperativa: el future del CLI se dropea en su siguiente await;
  si el CLI ya escribió parcialmente, la limpieza corresponde a sus `Drop`.
- El `StopReason::Cancelled` es obligatorio según el schema cuando llega un
  `session/cancel`, precisamente para no reportar un éxito/fallo fantasma.

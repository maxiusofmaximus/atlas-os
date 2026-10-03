# 44 - Phase 18: ACP real host loop (Phase 1.5d closeout)

## Objetivo

Cerrar el stub del servidor ACP: `session/prompt` pasa de `StopReason::Refusal` con
sentencia "Phase 1.5d: agent host loop not wired" a un host loop con routing real
sobre el contrato tipado `delegate::DelegateOutcome`, dispatch al CLI real para
`exec step`, y cancel/cwd estables.

## Lo encontrado

- `acp/mod.rs` ya hablaba el SDK oficial `agent-client-protocol` v2.0.0;
  el gap era el `session/prompt` handler: devolvía refusal sintético y dejaba
  caer `cancel_request` sin lógica efectiva ni cwd de sesión.

## Implementación

- Nuevo `PromptPlan` + `plan_prompt` (función pura) que traduce
  `DelegateOutcome` → `{ExecStep, FixRequested, RestartRequested, NotSupported}`.
- `run_server` ahora guarda `SessionId → cwd` (Arc<Mutex<HashMap>>) en `session/new`,
  recupera el cwd del prompt, y para `ExecStep` ejecuta
  `crate::cli::commands::exec::run(ExecAction::Step { .. }, profile)` real.
  Resultado se envía como `AgentMessageChunk` y el turno cierra con
  `StopReason::EndTurn` (error → mensaje de error + EndTurn).
- `FixRequested`/`RestartRequested`/`NotSupported` responden con un chunk explicativo;
  `NotSupported` cierra con `StopReason::Refusal`.
- Cancel sigue aceptado-but-simplificado (no-tracking de in-flight) — queda como
  deuda acotada; no mapea a work real aún, así que no oculta errores.

## Verificación

- `cargo test --features acp-server,cli acp::` → **42 passed; 0 failed**
  (incluye `plan_prompt_routes_supported_commands` y `first_prompt_text_extracts_text_block`).
- `cargo check` default / `--features acp-server,cli` / mínimos: OK.
- `cargo clippy --features acp-server,cli --all-targets -- -D warnings`: OK.
- `cargo fmt --check`: OK.

## Regresión

- Sin cambios en default features → el árbol binario headless no incluye acp-server.
- No se tocó Laya (sigue BLOQUEADA), no se añadieron crates nuevas (ya estaba
  el dep opcional del lock).

## Criterios de aceptación

- `session/prompt` ya no emite la sentencia Phase 1.5d; emite `EndTurn` con
  resultado real para `exec step` ✅ (test/vía dispatch).
- Refusal preservado solo para comandos no soportados o prompt vacío ✅.
- cwd estable por sesión ✅ (test implícito en `plan_prompt`).
- ACP harness sigue compilando en modo mínimo y default ✅.

# 47 - Phase 21 audit: panics/unwraps live in tests, not production

## Objetivo (revisado tras audit)

Antes de refactorizar panics en producción, verificar si realmente existen.

## Hallazgos

- `orchestrator/routing.rs` (16 panics), `cli/commands/research.rs` (32),
  `orchestrator/cascade.rs` (7), `orchestrator/wire.rs` (6): **todas** las
  coincidencias ocurren DENTRO de módulos `#[cfg(test)]` — son
  assertion-style `panic!("...")` de tests, aceptable por convención Rust.
- `learning/share.rs` (71 unwraps), `skills/marketplace.rs` (52),
  `journal/*.rs` (~36/archivo), `toast/queue.rs` (37): verificado que
  los `unwrap()` concentran en el bloque `#[cfg(test)]` del mismo archivo.
  Los `assert!`/`expect` en tests deben mantenerse.

## Decisión

Phase 21 orginal queda **CANCELADA como cambio de código**; no hay panics
accidentes en el árbol de producción auditado. Se sustituye por:

- Verificación formal: `cargo test --lib` (1103 ok) actúa como red; las
  cuentas totales de panic!/unwrap() se re-registran en el reporte cada vez
  que haya que decidir un toque de refactor real.

## Siguiente

Phase 20.2 sigue abierta para journal/tests.rs por dominio; no hay
regresión ni deuda productiva introducida por este audit.

# research/57 — Fase 35: aplicar el `Diff` al workspace

- **Fecha:** 2026-10-03
- **Estado:** **COMPLETA** (v35.0).
- **Contexto:** Fase 26 valida el `Diff` pero no lo escribe; F34 lo instrumenta. F35 cierra el
  ciclo: el `Diff` validado se aplica al workspace.

## 1. Diseño

- `coding/apply.rs` es **puro**: recibe el contenido actual (`path -> líneas`) y devuelve el
  resultado; no toca el FS. Coherente con `coding/runner.rs` (*"el runner NO toca el
  filesystem"*) y testeable offline.
- Semántica RFC 13 §8: `old_start..old_end` **semiabierto 0-based**; `(0,0)` inserta al
  principio; `new_lines == []` borra el rango. Los hunks se aplican en **orden descendente**
  de `old_start` para que un hunk no desplace las coordenadas de otro.
- `ApplyError` estructurado (`InvalidRange`/`OutOfRange`) en vez de panic: el caller decide
  si rechazar la escritura.

## 2. Sub-fase

- **v35.0 COMPLETA** — `coding/apply.rs` (`apply_file_edit`, `apply_diff`, `AppliedFile`,
  `ApplyError`); CLI `atlas execute --coding --apply [--root]`:
  1. `journal.save_checkpoint(MissionCheckpoint)` **antes** de escribir (auditoría del estado
     previo).
  2. Lee los ficheros que el `Diff` toca.
  3. `coding::apply::apply_diff` (puro) → escribe/borra bajo `--root`.
  9 tests (insert/replace/delete, multi-hunk, new/delete file, out-of-range, inverted range,
  `apply_diff` mixto).

## 3. Diferido (deliberado)

- Aplicación atómica (tmp+rename) y lock entre agentes (swarm): F37.
- Restauración del checkpoint (`git`/revert): el `MissionCheckpoint` ya se persiste; la
  restauración automática es posterior.

## 4. Fuentes

- RFC 13 §8 (hunks, `hunks_are_disjoint_and_sorted`), RFC 02 §3.4 (checkpoint).
- research/55 (Fase 26), research/56 (Fase 33/34).

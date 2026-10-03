# Laya re-audit - 2026-10-03

Generado por `scripts/laya_reaudit.ps1`. Fuente: crates.io API
(`https://crates.io/api/v1/crates/laya`) y GitHub API (`aovestdipaperino/laya-rust`).

## Evidencia

| Criterio | Valor observado | Cumple |
|---|---|---|
| `laya` > 0.2.x | `newest_version = 0.1.1` (updated 2026-09-20T12:58:25.265037Z, downloads 137) | NO |
| Mantenedores >= 2 | 2 contributors: aovestdipaperino(4), enzinol(2) | SI |
| Repo vivo | pushed_at 2026-09-30T15:23:16Z, archived False, stars 12, forks 2, open_issues 1 | contexto |

## Veredicto

**criterio CUMPLIDO en al menos un eje -> revisar Phase 13 (requiere decision del operador)**

Nota: el eje "mantenedores" se aproxima con el recuento de *contributors*
(autores de commits) que expone la API publica de GitHub. No equivale a
colaboradores con permiso de escritura; si ese recuento cambia el veredicto,
confirmar el write-access antes de desbloquear Phase 13.

## rand / tokenizers

| Dependencia upstream (Cargo.toml, rama main) | Linea observada |
|---|---|
| tokenizers | `tokenizers = "0.21"` |
| rand | `no declarado (sub-problema resuelto upstream)` |

El sub-problema original (rand/tokenizers) no forma parte del unlock por si
solo, pero se registra cada re-audit para no perder el contexto.
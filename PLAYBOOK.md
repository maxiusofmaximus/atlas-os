# Atlas OS — Playbook de ejecución (orden secuencial)

Decisión del operador: **cerrar B (baseline externo) antes de seguir ampliando el
harness**, y **hacer que la compuerta de fiabilidad gobierne qué modelo produce el
`Diff`**. Este fichero es el orden de ejecución vigente; el backlog de fases (Fases
3/4/6/7 y anteriores) vive en `Atlas OS/20 - Roadmap.md` y el plan de investigación
en `Atlas OS/research/`.

## Estado (reconciliado con `20 - Roadmap.md` y git, 2026-10-06)

- **F32 MEDIDO** — baseline externo (Terminal-Bench 2): `pass_rate` 0.000, oracle 0.88 (`86b844e`).
- **F33–F37 CERRADAS** — reliability gate (`0735daf`), coste por `Diff` (`98ea6bc`), apply del `Diff` (`e39edc6`), research evidence (`db75839`), swarm merge (`4248293`). Refs roadmap §503–551.
- **F38 corrido** — agent-mode adapter + corrida registrada **0/11** (`0bf6b0e`); el gap restante es razonamiento de frontera.
- **F39 CERRADA** — agent loop verificado contra Groq (roadmap §F39).

> El detalle de estado de F32–F39 es autoritativo en `Atlas OS/20 - Roadmap.md`; este playbook es el **orden de ejecución**, no la fuente de estado.

## Cómo leer cada ítem

- **DoD** = Definition of Done (criterio objetivo de cierre).
- **Dep** = prerequisitos. Cada paso deja el repo verde: `cargo clippy --all-targets
-- -D warnings` + `cargo test --lib` + `cargo fmt --check`.
- Los ítems ⚠️ requieren infraestructura que puede no estar en esta máquina.
- Un ítem terminal = un commit + push (`origin main`), Conventional Commits.

---

## F32 — Cerrar B: baseline externo (doce pasos)

1. **F32.1** Instalar el harness e inspeccionar el schema real. `uv tool install harbor`
   (o `pip`); `harbor datasets list | grep terminal-bench` → confirmar la etiqueta
   (`terminal-bench@2.0`). Abrir el `jobs/<job-id>/result.json` de ejemplo y listar los
   campos reales. **DoD:** rutas+claves documentadas en `tools/harbor_atlas/README.md`.
   **Dep:** red. ⚠️
2. **F32.2** Cerrar el contrato `JobResult`/`TrialResult` (y `state.json` si es la fuente).
   Escribir un `serde_json::Value` mínimo del schema real como fixture de test.
   **DoD:** test que parsea el fixture; `eval/harbor.rs` alineado. **Dep:** F32.1.
3. **F32.3** Superficie de config del harness. `atlas execute|mission plan` deben poder
   fijar el **endpoint/modelo** usado por el harness vía perfil `harbor` (deployment +
   `api_key_env`) o flags; documentar el mapeo en `README.md`. **DoD:** un `harbor` profile
   con seed de deployments carga y `atlas execute --help` refleja la config.
4. **F32.4** Script de build de la imagen. Dockerfile/asset que (a) instala las deps de la
   tarea, (b) copia el binario `atlas` a `PATH`, (c) expone las envs de credenciales.
   **DoD:** documentado en `tools/harbor_atlas/README.md`. ⚠️
5. **F32.5** Validar el ciclo `install`/`run` del adaptador. `atlas_agent.py` con
   `exec_as_root`/`exec_as_agent` y captura del `mission_id`; probar `install()`+`run()` en
   una tarea trivial (1 tarea). **DoD:** una trial completa produce `result.json`. **Dep:** F32.4. ⚠️
6. **F32.6** ✓ (offline, ya hecho en esta máquina) Confirmar que `atlas --profile harbor
mission new --force "<task>"` produce misión + `plan` + `execute`. Ref: `f47c00d`.
7. **F32.7** Corrida smoke real (1–3 tareas, n-concurrent 1). `harbor run -d
terminal-bench@2.0 --agent harbor_atlas.atlas_agent:AtlasAgent --model <m> --n-concurrent 1`
   → `jobs/<id>`. Rellenar la tabla del runbook con el resultado. **DoD:** `jobs/<id>` existe
   y se inspecciona. **Dep:** F32.5, un endpoint de modelo con creds. ⚠️
8. **F32.8** Ingestión real. `atlas eval import jobs/<job-id>` → `model_invocations` +
   `eval_runs`/`eval_cases` (harness=terminal-bench, model=m, pass_rate, coste).
   **DoD:** `atlas eval report` muestra la fila. **Dep:** F32.7.
9. **F32.9** Par harness–modelo publicado. Actualizar `tools/harbor_atlas/README.md` (tabla:
   modelo, tareas, pass_rate, tokens, coste) + `README.md §Evaluation`. **DoD:** tabla en el
   repo. **Dep:** F32.8.
10. **F32.10** Integrar B como gate de CI **opt-in** (no bloquear PRs sin Docker):
    `.github/workflows/eval-baseline.yml` con runners Docker, o comentario manual de los
    números en el README. **DoD:** workflow manual/scheduled añadido. **Dep:** F32.9.
11. **F32.11** Presupuesto offline ampliado. Añadir golden tasks para los motores aún sin
    cobertura mientras B no corre (budget para CI barato). **DoD:** ≥1 golden nueva, suite
    verde en el `eval-gate`.
12. **F32.12** Revisión de cierre. `research/56` con hallazgos del baseline, impactos en el
    roadmap, y actualización de `20 - Roadmap.md` / `26 - Index`. **DoD:** docs de cierre.

**DoD-F32:** un número externo de Atlas (harness×modelo, pass_rate/coste) ingerido y
publicado, o —si no hay infra— F32.1–F32.6 completados + el runbook listo, con F32.7–F32.12
marcados bloqueados por infra.

---

## F33 — Compuerta de fiabilidad en el coding loop

1. **F33.1** Inspeccionar el seed: cómo se enumeran `Deployment` por `model_id`
   (`Registry::from_bundled_seed`) y de dónde salen las `Reliability` (`reliabilities_from_journal`).
2. **F33.2** Resolver política. Leer `reliability_gate` (M38/v37) y `eval_models`;
   producir `Vec<Reliability>` + el flag on/off. La elección del primario de un `Diff` usa
   `gate_refs` para **filtrar candidatos** (fail-safe: nunca deja 0).
3. **F33.3** Cablear: el caller que hoy arma el primario (`execute_coding_step` / CLI
   `execute --coding`) usa `gate_refs` sobre los deployments antes de `call_diff_with_cascade`.
   **DoD:** un test con una `Reliability` denegada prueba que ese modelo no es primario.
   **Dep:** F33.2.
4. **F33.4** Persistir la decisión: journal (`model_invocations.route_taken_json` o una fila
   dedicada) + HUD/report muestra el modelo elegido y por qué (gate application).
   **DoD:** la fila de la corrida coding registra `gate=on/off`, elegido, denegados.
   **Dep:** F33.3.
5. **F33.5** Cierre: golden `orchestrator.reliability_routing` + docs (`20`/`26`).

**DoD-F33:** con el gate ON, el loop de coding elige modelos por fiabilidad histórica y la
decisión queda auditada; con el gate OFF, comportamiento actual intacto.

---

## F34 — Coste/observabilidad del coding loop

1. **F34.1** `execute --coding` persiste una `ModelInvocationRow` **con `route_taken_json` y
   el id de la `Diff`** por step (hoy solo el chat loop persiste invocaciones).
2. **F34.2** Publicar `BusEventKind::AgentTokens`/`agent.diff` por `Diff` (tokens, coste,
   ficheros, validación, repair).
3. **F34.3** HUD: card/endpoint que liste los `Diff` de una corrida con su validación+repair
   (`GET /hud/eval/…` extendido o `/hud/coding`).
4. **F34.4** Cierre: test de que el coste por `Diff` queda en el journal; docs.

**DoD-F34:** cada `Diff` de una corrida coding deja tokens/coste/validación/repair auditables.

---

## F35 — Aplicar el `Diff` al workspace

1. **F35.1** Kernel puro de aplicación: `apply_diff(file_edit, workspace_files)` con la
   semántica de hunks (`old_start..old_end`, inserción/borrado) y validación
   `hunks_are_disjoint_and_sorted`. Tests de propiedad.
2. **F35.2** Checkpoint previo (journal `WorkspaceFile.sha`/snapshot) antes de escribir
   (RFC 02 §3.4), con apply solo si la validación lo permite.
3. **F35.3** CLI/HUD: `atlas execute --apply` (o flag) escribe al workspace y reporta.
4. **F35.4** Cierre: test end-to-end (workspace inyectado) + docs.

**DoD-F35:** un `Diff` validado puede escribirse al workspace de forma determinista y auditable.

---

## F36 — Motor de Research (Roadmap Fase 3)

1. **F36.1** RFC de Research si no está (ver `Atlas OS/03`/roadmap Fase 3).
2. **F36.2** `ResearchRun` real: `atlas research query` + persistencia + refs en el `Diff`.
3. **F36.3** Conectar con la EvidenceGate: un `Diff` de código con refs de research pasa la
   gate (hoy falla por falta de refs — ver test `coding_step_parses_a_diff_and_verifies_it`).
4. **F36.4** Cierre: golden + docs.

**DoD-F36:** un `Diff` de código con research adjunto pasa la validación sin repair por
evidencia.

---

## F37 — Swarm (Roadmap Fase 4)

1. **F37.1** Revisar `swarm/` y el RFC `05`; identificar el gap (multi-agente real vs pool).
2. **F37.2** Wire del coding loop a varios agentes + merge de `Diff`s (`dispatch_dag`).
3. **F37.3** Cierre: golden + docs.

**DoD-F37:** ≥2 agentes producen diffs concurrentes y el merger los compone.

---

## F38 — Harness end-to-end real (posterior a F36/F37)

1. **F38.1** Re-correr B con el ciclo completo (coding + research + swarm) y comparar contra
   F32.2 (delta por capacidad).
2. **F38.2** Publicar la curva harness×modelo×capacidad.

**DoD-F38:** una comparativa antes/después reproducible.

---

## Notas de estado

- ✅ Hecho (esta sesión): Fase 25 (execution loop), Fase 26 (LLM-driven coding), B-prep
  (adaptador Harbor). Commits `b15ab22`, `5331786`, `c09331c`, `2ca4a0f`, `6dc9f68`, `7220f35`.
- ⚠️ Bloqueado por infra (Docker + endpoint de modelo + creds): F32.1, F32.4, F32.5, F32.7.
- El resto es ejecutable **offline** con mock/harness inyectado.

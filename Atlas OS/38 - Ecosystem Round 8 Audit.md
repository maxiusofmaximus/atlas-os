# 38 - Ecosystem Round 8 Audit

Auditoría del ecosistema de herramientas para testing en Android real. Verificada vía GitHub API + README (Sep 2026). Complementa RFC 35 (Round 7) y el modelo remote-live (8.5, RustDesk lateral).

## 1. Contexto y motivación

El operador (Max) quiere probar la funcionalidad de Atlas OS (HUD mobile, la app Atlas, skills) en dispositivos Android reales. El patrón establecido de integración lateral (RustDesk 8.5 — proceso externo, jamás bundling AGPL) aplica igual aquí.

## 2. google/artemis — Android automation por lenguaje natural (VERIFICADO)

`github.com/google/artemis` (GitHub API: `google/artemis`, Python 3.12+, **Apache-2.0** ✅, 10.5k stars, 1k forks, 119 commits, actualizado 2026-09-28, no archivado, default branch `main`; incluye código de Minitap, Inc. `minitap-ai/mobile-use`):

*"ARTEMIS turns natural-language instructions into reliable Android automation. It automates end-to-end workflows, captures logs, and integrates seamlessly with AI coding assistants such as Antigravity, Codex, and Claude Code. It also achieves 99%+ success rate on AndroidWorld Benchmark."*

### 2.1 Superficies

| Superficie | Qué hace | Comando |
|---|---|---|
| **MCP server nativo** | Conecta IDEs (Antigravity, Claude Code, Windsurf, Codex, Cursor, VS Code, Cline/Roo, OpenClaw) a dispositivos Android REALES: `mobile_run_task`, `mobile_manage_task`, `mobile_get_device_state`, `mobile_inspect_trace`, `mobile_diagnose` (Logcat + screenshots) | `uv run artemis mcp --install all` |
| **CLI** | Ejecución directa de test cases, exploratory stability tests, benchmarks AndroidWorld | `uv run artemis run "..." --profile flash` |
| **Web Console** | Screen mirroring live, prompt sandbox, execution replays, task queue | `uv run artemis ui` (localhost:8000) |
| **Python SDK** | `artemis-client` (packages/artemis-client, zero-runtime-dependency) — pytest/CI/CD con Pydantic outputs + assertions | `uv add "artemis-client @ git+...#subdirectory=packages/artemis-client"` |

### 2.2 Herramientas que auto-instala (start.bat Windows ✅)

ADB, scrcpy, FFmpeg, Python (`uv`) — detecta y auto-instala. **Artemis Accessibility Helper**: servicio de accesibilidad pequeño que lee el layout sin tomar la conexión UiAutomation (fallback UIAutomator2 vía `ARTEMIS_HIERARCHY_BACKEND=uiautomator`); se pre-instala con `artemis helper install` (evita ~3s en la primera tarea), se quita con `helper uninstall`.

### 2.3 Ejecución profiles: Flash vs Pro

- **Flash** (`--profile flash`): loop reactivo ~3-5s/paso, un modelo observa→piensa→actúa, sin graph orchestration. Sin plan/checkpoints/report — rutina determinista.
- **Pro** (`--profile pro`): multi-agent graph ~15-40s/paso — **Planner** (plan Markdown con `verify`/`assert` check items) + **Operator** (Safety Net pre-ejecución XML-first/pixel-fallback + fast-action bursts + execution incidents para recovery) + **Checker** (read-only, verifica checkpoints, `--verification-level: off/final/checkpoints/strict`). Tier del Explorer (`flash`/`pro`/`ultra`) es user setting — **nunca lo elige el agente**.

### 2.4 Benchmarks + arquitectura

- **99%+ task completion** en AndroidWorld (100+ multi-step tasks, 20+ apps).
- Locating: accessibility hierarchies + OCR + visual models (Canvas/Compose/Flutter).
- **History compression**: Flash/Pro reemplazan screenshots viejos con visual summaries y comprimen pasos completados en chunks buscables (`search_history`/`replay_steps`) — **mismo patrón que el System One compaction de Atlas OS (5.3)**.
- Rules file `mcp_server/rules.md`: "Artemis Mobile Testing Mindset" (Active Exploration, Flash vs Pro routing, Dynamic-First Coordinate-Fallback locator).

## 3. Cómo ayuda a Atlas OS

1. **Testing de funcionalidad en Android real**: KPIs de Atlas OS (latencia UI <100ms, alucinaciones ≤1/100 diffs) verificables en dispositivo/emulador real — el HUD móvil de RFC 24 (remote accesible desde cualquier dispositivo) se prueba con artemis en vez de manualmente.
2. **Integración LATERAL** (patrón 8.5 RustDesk): Atlas OS detecta `artemis`/`uv` en PATH y lanza la sesión externa (`std::process::Command` — jamás link/bundle, es Python RFC 25 §11); `ATLAS_ARTEMIS_BIN` env override. Sin artemis → mensaje útil.
3. **Adopción de patrones (ya materializada en Atlas OS)**:
   | Patrón artemis | Equivalente Atlas OS |
   |---|---|
   | Flash vs Pro profiles | `ExecutionProfile` 4 modes (RFC 21) + tiers `flash`/`pro`/`ultra` del Explorer (user setting, jamás el agente) |
   | History compression (visual summaries + chunks) | System One compaction (5.3) + FTS5 journal (8.0) |
   | Planner/Operator/Checker | Planning (12) / Coding (13) / Validation (14) engines |
   | Safety Net pre-ejecución | EvidenceGate (2.5) + `approval_for` (7.1) |
   | Execution incidents para recovery | Execution Supervisor (19) + Repair (15) |
   | MCP tools tipadas | Kernel Bus `KernelCommand` + RFC 06/07 |
4. **Superficie futura "mobile testing"**: los 5 MCP tools de artemis (`mobile_run_task`/`manage_task`/`get_device_state`/`inspect_trace`/`diagnose`) son el modelo para una posible Fase 8.6 / lateral — evaluar cuando Phase 10 cierre.

## 4. Restricciones y boundary rules

- **Python (uv) — RFC 25 §11**: proceso EXTERNO solamente (como RustDesk), jamás link/bundle/Conda. El SDK Python (`artemis-client`) NO se integra en Atlas OS (Python); su uso es del operador manualmente.
- **AGPL**: artemis es Apache-2.0 ✅ (sin problema AGPL — pero sigue siendo Python, misma regla).
- El helper de accesibilidad se instala en EL DISPOSITIVO (no en Atlas OS) — el operador lo gestiona con `artemis helper install/uninstall`.
- Sin RustDesk/artemis en PATH → mensajes útiles (patrón 8.5).

## 5. Priorización por dependencia

```
artemis (lateral, Python externo) ──► `atlas mobile` (8.5-pattern: detect + launch + guide)
                                        └─ requiere: device/emulator + USB debugging (operador)
Phase 10 (SDK + marketplace) ────────► roadmap v1 (en orden, ANTES de mobile testing surface)
```

**Orden recomendado:**
1. **Phase 10 (Plataforma abierta)** — en orden del roadmap (SDK público + marketplace con firma + remixing + learning social).
2. **`atlas mobile`** (S, patrón 8.5): detecta artemis/uv en PATH, `atlas mobile status/guide/run` lanza sesión externa + documenta el flujo de testing (opción post-Phase 10 o lateral 8.6).
3. **Mobile testing surface** (evaluar): los 5 MCP tools como modelo — solo si el operador valida el flujo con un dispositivo real.

## 6. Status de este RFC

- **Versión:** 1.0 (audit completo, Sep 2026).
- **Tipo:** Informativo + priorización. No introduce APIs ni dependencias nuevas inmediatamente.
- **Método:** GitHub API (`api.github.com/repos/google/artemis`) + README fetch directo. Verificado: Apache-2.0, Python 3.12+/uv, 10.5k stars, no archivado, actualizado Sep 2026.

## 7. Fuentes de auditoría

- `https://github.com/google/artemis` — README (superficies, perfiles, benchmarks, arquitectura, licencia)
- `https://api.github.com/repos/google/artemis` — metadatos del repo (verificación)
- `https://github.com/minitap-ai/mobile-use` — código incluido de Minitap, Inc. (referencia)

# ux-catalog — Función × Referente (HUD de orquestación)

Tabla compacta: por cada **función crítica de un HUD de orquestación de agentes**, los referentes que la tienen con evidencia **[Os]** (o **[Link]** = `docs/design/CONSENSUS_AUDIT.md`), y si está en **RFC 24/65/66/67**. Generado por Researcher (pasada 4). Fuente de cada referente: su ficha en `ux-catalog/`.

Leyenda estado RFC: **sí** = documentada · **parcial** = mencionada/implícita · **NO-DOCUMENTADA** = aparece en ≥3 referentes y no consta en RFC 24/65/66/67.

| Función crítica | Referentes con evidencia | Ficha | En RFC 24/65/66/67 |
|---|---|---|---|
| **Estado de agente con rollup** | Herdr (rollup pane→tab→workspace) [Link]; Orca (status dot `permission>done>heuristic`) [Link]; Cate (panel `running/waiting/finished`) [Os]; kodo (log de ciclos con veredictos) [Os]; GridBash (pane activity) [Os] | `ade-orquestador.md`, `terminal.md` | sí (RFC 24 tarjeta + RFC 66 §4/§10) |
| **Cola de aprobaciones HITL** | Orca (decision gates vs `ask`) [Link]; Jazz (pregunta en el canal donde estés) [Os]; Smelt (modos Normal/Plan/Apply/Yolo + permisos) [Os]; Cline (auto-approve + checkpoints) [Link]; Amp (Ship) [Os] | `ade-orquestador.md` | sí (RFC 24 §5, RFC 66) |
| **Steer / fork en caliente** | Warp (steer mid-task / handoff) [Os]; Cline (Restore Task / Files & Task) [Link]; Cate (worktrees desde prompt) [Os] | `terminal.md`, `ade-orquestador.md` | sí (RFC 24 §3.2, RFC 05 §7) |
| **Timeline / audit** | LangSmith (Messages/Turns/Details) [Os]; Langfuse (traces/sessions) [Link]; n8n (execution history) [Link] | `observabilidad.md` | sí (RFC 24 §10) |
| **Demos sobre diffs (review)** | Vibe Kanban (inline diff comments → agente) [Os/Link]; Amp (diff + comentario revisor) [Os]; Cate (side-by-side diffs) [Os] | `kanban-pm.md`, `ade-orquestador.md` | sí (RFC 24 §9) |
| **Command palette** | VS Code (palette con modos) [Link]; opencode (`/` + leader `ctrl+x`) [Os]; Cate (`Cmd+K`) [Os]; Linear (shortcuts board/list) [Os] | `terminal.md`, `kanban-pm.md` | sí (RFC 65 CommandPalette) |
| **Coste / budget en vivo** | darce (cost+tokens en status bar) [Os]; DeerFlow (token budget + gauge de contexto) [Os]; Helicone (gateway+logging) [Os]; MLflow (monitor) [Os] | `terminal.md`, `observabilidad.md` | sí (RFC 24 §6/§7) |
| **Worktrees por agente** | Orca/Crystal (worktree por agente) [Link]/[Os]; GridBash (`--worktrees` por pane) [Os]; Cate (crea worktree+branch) [Os]; Omnigent (sub-agentes en parallel worktrees) [Os] | `ade-orquestador.md` | sí (RFC 05 §3, RFC 24 §12) |
| **Canvas con log sincronizado** | n8n (log docked + selection sync canvas↔log) [Link]; Flowise (visual debugging, execution logs) [Os]; Langfuse (Aggregated↔Expanded) [Os] | `canvas.md` | sí (RFC 66 §10) |
| **Notificaciones / ping HITL** | Cate (ping cuando un panel necesita respuesta) [Os]; Jazz (pregunta en Telegram/Discord/etc.) [Os]; DeerFlow (gauge/contexto) [Os] | `ade-orquestador.md` | parcial (RFC 24 §16 push) |
| **Sesiones que sobreviven reinicios/desconexiones (scrollback + resume)** | Cate (sessions survive restarts, reattach + resume) [Os]; tlbx (sessions survive disconnects) [Os]; GridBash (background panes + resume) [Os]; CliDeck (session resume) [Os] | `ade-orquestador.md` | **NO-DOCUMENTADA** (RFC 19 cubre checkpoints de misión, no la UX de reattach con scrollback) |
| **Panel que refleja turno `running/waiting/finished` y avisa** | Cate (agent-aware terminals + ping) [Os]; GridBash (stable pane activity) [Os]; Herdr (estado por pane) [Link] | `ade-orquestador.md` | **NO-DOCUMENTADA** (RFC 66 tiene estado de agente, pero no el "ping cuando necesita respuesta" por panel) |
| **Agente definido en un único config (modelos/persona/tools/permisos)** | Jazz (one JSON) [Os]; Smelt (`init.lua`) [Os]; OpenCastle (config generada como lockfile) [Os] | `ade-orquestador.md` | **NO-DOCUMENTADA** (RFC 06/07 definen skills/MCP; no un manifiesto único de agente por UI) |
| **Ack/aprobación en el canal donde está el humano** | Jazz (Telegram/Discord/iMessage/WhatsApp) [Os]; OpenClaw (canales + Control UI) [Os]; CliDeck (phone) [Os] | `ade-orquestador.md` | parcial (RFC 24 §16 mobile) |
| **Grid de PTYs por pane (multi-pane de agentes)** | GridBash (hasta 100 PTY panes) [Os]; Cate (dock/split) [Os]; agent-manager/hcom (TUI por pane) [Os] | `ade-orquestador.md` | **NO-DOCUMENTADA** (RFC 24 §12 worktrees, pero no la grid de PTYs) |
| **Proof-of-work al completar (CI/PR/video)** | Symphony (CI status, PR review, complexity, walkthrough video) [Os]; kodo (tester verifica) [Os]; OMK (verifica evidencia antes de done) [Os] | `ade-orquestador.md` | sí (RFC 14 §10 EvidenceGate) |
| **Multi-harness / BYOA (orquestar CLIs de terceros)** | Omnigent (Claude Code/Codex/Cursor/Pi) [Os]; AionUi (20+ CLI) [Os]; cmux (agents en paralelo) [Os]; GridBash/Jazz [Os] | `ade-orquestador.md` | sí (RFC 27 BYOA, RFC 63/64) |

## Funciones NO-DOCUMENTADAS en RFC 24/65/66/67 (aparecen en ≥3 referentes)
1. **Sesiones que sobreviven reinicios/desconexiones con scrollback + reattach/resume.**
2. **Panel con `running/waiting/finished` por agente + aviso/ping cuando requiere respuesta.**
3. **Definición del agente en un único manifiesto (modelos/persona/tools/permisos) gestionable por UI.**
4. **Grid de PTYs multi-pane por agente (layout físico de terminales, no worktrees).**

(El acks en el canal y las notificaciones están **parcialmente** cubiertos por RFC 24 §16.)

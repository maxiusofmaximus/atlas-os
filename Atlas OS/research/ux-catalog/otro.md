# ux-catalog / otro

Fichas de referentes prioridad A que **no encajan** en IDE / ADE / terminal / observabilidad / kanban / canvas / chat (verticales de dominio u otros).
Procedencia como en `terminal.md`.

---

## OpenCADStudio (HakanSeven12) — vertical CAD (Rust) — prioridad A — [Os parcial]

- **URL (en vivo):** https://github.com/HakanSeven12/OpenCADStudio [Os]
- **Funcionalidades clave:** "A **CAD application built with Rust** — 2D/3D drawing, **DWG/DXF support**, and **GPU-accelerated rendering**" [Os]. Vertical de referencia del operador (M. CAD/3D).
- **Layout y navegación:** [P] (el README indexado no detalla UI).
- **Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar:** (a) **Rust + GPU rendering** para un dominio nativo (alineado con el stack de Atlas); (b) es el caso de **engine vertical por dominio** (RFC 64 Domain Packs: `cad`); (c) soporte DWG/DXF como integración de formato.
- **Evitar:** [P].
- **Nota:** en RFC 64 ya se cita como referente del pack `cad`.

---

## Moonshot Kimi — provider — prioridad A — [Os parcial]

- **URL (en vivo):** https://platform.moonshot.ai/ [Os]
- **Funcionalidades clave:** plataforma API **Kimi** (K3 lanzado; "Build with Kimi API") [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P] (es provider, no UI de orquestación).
- **Adoptar / Evitar para Atlas:** adoptar como **provider** del Model Orchestrator (RFC 04); sin UI propia relevante.

## Zhipu GLM (Z.ai) — provider — prioridad A — [Os parcial]

- **URL (en vivo):** https://docs.z.ai/ (developer docs) [Os]
- **Funcionalidades clave:** plataforma de modelos **GLM**; "GLM Coding Plan — monthly access … compatible with top coding tools like Claude Code…" [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P] (provider).
- **Adoptar / Evitar para Atlas:** provider del Model Orchestrator (RFC 04) y del plan de coding; sin UI propia de HUD.

## MiniMax — provider + agente — prioridad A — [Os parcial]

- **URL (en vivo):** https://www.minimax.io/agent [Os]
- **Funcionalidades clave:** plataforma de modelos (M3/M2.7…) + **MiniMax Agent**; la página /agent lista modelos y producto agéntico [Os].
- **Layout / Estados / Aprobaciones / Atajos / Onboarding / Accesibilidad:** [P].
- **Adoptar / Evitar para Atlas:** provider + agente vertical; cubrir su UI de agente a fondo en otra pasada.

## ByteDance Seed (Doubao) — provider — prioridad A — [P]

- **Motivo [P]:** provider de modelo; **sin URL/doc de UI verificada en vivo** esta sesión (pendiente console de ByteDance/Volcengine).

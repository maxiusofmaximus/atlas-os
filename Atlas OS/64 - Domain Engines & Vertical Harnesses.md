# RFC 64 — Domain Engines & Vertical Harnesses (Fase 28)

**Author:** opencode architect agent · **Date:** 2026-10-04
**Status:** In progress — 10/11 implementado. Solo pendiente item 8 (stages de validación de dominio wiring; geometry/units/export declarados en los manifiestos).
**Depends on:** RFC 02 (Kernel / Capability Resolver), RFC 03 (Engines), RFC 06 (Skills), RFC 07 (MCP), RFC 11 (Context / Project Map), RFC 13 (Coding), RFC 14 (Validation), RFC 18 (Security), RFC 25 (Stack), RFC 28 §I (integración lateral de tools), `research/62` (genealogía).
**Scope:** Formaliza lo que la investigación `62` mostró repetirse en el estado del arte — **un engine vertical por dominio** (Cave Engine, Stove 3D, Zoo/CadQuery, MecAgent, mr-mak, artemis) — **sin** inflar el core. Los motores base siguen genéricos; cada dominio se aporta como **Domain Pack declarativo** (Skills + MCP + validation stages + artifact types + model policy) y como **integración lateral** de herramientas externas (jamás bundling, RFC 25 §11).

---

## 1. Contexto y motivación

`research/62` documenta ~425 proyectos objetivo. Un patrón se repite: los productos que "se sienten como un senior" suelen ser **verticales** — CAD (MecAgent, Zoo, CadQuery), game (Cave Engine, Godot, Unity), 3D/asset (**Mixar** — fork de Blender 5.2 con agente integrado, Stove), mobile (artemis), creativo-media (Higgsfield, Flova, mr-mak). Atlas no debe empaquetar cada vertical: debe **ofrecer un contrato** para que un dominio se conecte como un pack, reutilizando los motores existentes.

> **Referente directo:** `Mixar-AI/mixar-app` es exactamente este patrón materializado — un DCC (Blender) con un agente ("Mixie") que opera la herramienta por lenguaje natural, con BYOK. Valida el diseño de Domain Pack + tools laterales de este RFC.

Esta es la evolución natural de la tesis "plataforma para agentes" (RFC 00): el core no crece en features de dominio; crece en **mecanismos de composición**.

## 2. El patrón "engine por dominio"

Un **Domain Engine** es un bundle declarativo, no código en el core:

```toml
# domain.toml (contrato)
[domain]
id = "cad"
title = "Mechanical / Parametric CAD"
detect = ["*.step", "*.stl", "*.f3d", "cadquery", "freecad"]   # señales para Project Map (RFC 11)

[engines]           # mapea a motores base existentes (RFC 03)
planning = "default"
coding   = "code-as-cad"       # CadQuery/OpenSCAD/KCL como representación ejecutable
validation = ["geometry.manifold", "units.consistent", "export.step"]

[skills]            # RFC 06
bundled = ["cad.text-to-cadquery", "cad.parametric-review", "cad.gdt-check"]

[mcp]               # RFC 07 (todos laterales)
servers = ["freecad-mcp", "blender-mcp"]

[tools.lateral]     # RFC 28 §I (proceso externo, sin bundling)
open = ["freecad", "openscad", "zoo"]
probe = "atlas domain probe cad"

[artifacts]         # tipos verificables
types = ["step", "stl", "render.png", "drawing.pdf"]

[policy]            # RFC 21 / RFC 18
sensitive = ["exec.run", "fs.write.*"]
sandbox = "container"
```

El **Capability Resolver** (RFC 02) + el **Project Map** (RFC 11) detectan el dominio y activan el pack; el agente deja de ser "genérico" sin que el core sepa de CAD.

## 3. Catálogo inicial de Domain Packs

| Pack | Dominio | Motores/representación | Integraciones laterales (research/62) |
|---|---|---|---|
| `coding` | Default (software) | LSP, tests, Biome/tsc (existente) | — |
| `cad` | Mecánico / paramétrico | CadQuery / OpenSCAD / KCL | FreeCAD, OpenSCAD, Zoo, MecAgent |
| `gis` | Civil / topografía | scripting + validación geométrica | **OpenCADStudio**, QGIS |
| `game` | Videojuegos | Godot/Bevy (scripting), validation de assets | Cave Engine, Unity, Unreal, Godot, Bevy |
| `creative-media` | 3D/asset/render | Blender MCP, img2threejs | Blender, **Mixar** (agente-en-Blender, GPL-3.0), Stove 3D, Poly Haven/Poly Pizza |
| `media-gen` | Video/imagen/audio | facade a providers | Higgsfield, Flova, fal.ai, Runway, Suno |
| `mobile` | Android (RFC 38) | artemis (lateral) | artemis, MaxAppsHub (RFC 41) |
| `data` | ETL/analítica | SQL/Python, dbt-like stages | drawDB, Neon, Turso |
| `audio` | Voz/música | TTS/STT stages | Fish Audio, Mureka, ElevenLabs |

## 4. Contrato `DomainEngine`

```rust
struct DomainPack {
    id: String, title: String,
    detect: Vec<String>,                 // glms → Project Map
    engines: EngineRouting,              // → RFC 03/04
    skills: Vec<SkillId>,                // → RFC 06
    mcp: Vec<McpServerRef>,              // → RFC 07
    lateral: Vec<LateralTool>,           // → RFC 28 §I
    artifacts: Vec<ArtifactType>,        // → RFC 14 EvidenceGate
    policy: DomainPolicy,                // → RFC 18/21
}
```
`resolve_pack(project_map) -> Option<DomainPack>` (Capability Resolver, RFC 02 §Capability Resolver).

## 5. Integraciones laterales (registro unificado)

Se generaliza el patrón de RFC 28 §I: un **registro** de herramientas externas por dominio, cada una con `probe` (¿está instalada?), `open <args>` (shell-out), `guide` (instalación) y `SensitiveAction`. Cero bundling, cero crates.

```bash
atlas domain tools cad        # lista tools del pack cad + estado (probe)
atlas domain open cad zoo     # shell-out a la herramienta externa
atlas domain guide cad freecad
```

## 6. Seguridad (RFC 18)

- Cada `LateralTool` y cada `Tool` de dominio declara `sensitivity`; los packs firmados (SHA-256, RFC 18) son obligatorios para instalación desde marketplace (RFC 10.x).
- `sandbox = container` por defecto en packs que ejecutan código nativo (CAD/game).
- Un pack nunca hereda secretos del keychain ni de `profiles::Profile`.

## 7. Esquema SQL — M51 (schema v40)

```sql
CREATE TABLE IF NOT EXISTS domain_packs (
  id TEXT PRIMARY KEY, title TEXT NOT NULL, version TEXT NOT NULL,
  manifest_toml TEXT NOT NULL, sha256 TEXT, signed INTEGER NOT NULL DEFAULT 0,
  installed INTEGER NOT NULL DEFAULT 1, ts INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS domain_runs (
  id TEXT PRIMARY KEY, mission_id TEXT NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  domain_id TEXT NOT NULL, pack_version TEXT NOT NULL, status TEXT NOT NULL,
  ts_started INTEGER NOT NULL, ts_ended INTEGER
);
```

## 8. Cambios Rust (plan)

- `src-tauri/src/domain/{mod,manifest,registry,resolver,lateral}.rs` (nuevo) — parse `domain.toml`, registry, `resolve_pack`, registro lateral.
- `core/` (Capability Resolver) — consultar packs por Project Map.
- `journal/domain.rs` — writers M51 + wrappers.
- `cli/commands/domain.rs` (nuevo) — `atlas domain list/use/install/open/probe/guide`.
- `skills/` — loader acepta skills del pack; `validation/stages/` — stages de dominio (geometry, units…).
- Sin migración de motores base (RFC 03 intacto).

## 9. Tests

- `domain.manifest_parse` (happy + malformed) · `domain.resolve_by_project_map` (CAD/game/mobile) · `domain.lateral_probe_not_installed` → guía (no silent no-op) · `domain.pack_signature_required`.

## 10. Actualizaciones a RFCs existentes

- **RFC 02** — Capability Resolver consulta Domain Packs.
- **RFC 03** — §1 aclara "motores base genéricos + Domain Engines declarativos".
- **RFC 06** — packs como fuente de skills; **RFC 07** — MCP por dominio.
- **RFC 28** — §I generalizado a "registro lateral" reutilizable.
- **RFC 26** — cross-refs (`domain/`, `atlas domain`).

## 11. Checklist

1. [x] Contrato `domain.toml` + parser (`domain/manifest.rs`; validación de id + SHA-256).
2. [x] `DomainRegistry` + `resolve_pack` (Capability Resolver, `domain/registry.rs`).
3. [x] Registro lateral (`domain/lateral.rs`) con `probe`/`open`/`guide` (missing tool → guía, no no-op).
4. [x] M51 (`domain_packs`/`domain_runs`, schema v40 + `journal/domain.rs`).
5. [x] CLI `atlas domain list/use/install/open/probe/guide` (verificado por smoke).
6. [x] Packs iniciales `coding`, `cad`, `game`, `creative-media` (embebidos).
7. [x] Packs `mobile` (artemis) + `gis` (OpenCADStudio) + `media-gen` (7 seed packs embebidos).
8. [~] Stages de validación de dominio (geometry/units/export) — **⏳ diferido**: el manifiesto declara los nombres (`engines.validation`) pero RFC 64 no especifica la semántica ni el comando de cada stage. Requiere validadores por dominio o un campo `[validation] command=` en el manifiesto (decisión de diseño pendiente; no se inventan checkers placeholder).
9. [x] Firma obligatoria de packs (RFC 18 §6): `install` exige `--sha256`; mismatch rechazado (verificado).
10. [x] Docs operador por pack (`docs/domain-<id>.md`, 7 guías).
11. [x] Cross-refs RFC 02/03/06/07/26/28 (RFC 26 actualizado).

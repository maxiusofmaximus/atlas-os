# Domain pack — `cad`

> Operator guide for the mechanical / parametric CAD domain pack (RFC 64).

## What it is

Routes coding to `code-as-cad` (CadQuery / OpenSCAD / KCL as the executable
representation) and validates geometry. External DCC tools are **lateral**
(never bundled).

## Manifest (`src-tauri/src/domain/packs/cad.toml`)

| Field | Value |
|---|---|
| `detect` | `*.step`, `*.stp`, `*.stl`, `*.f3d`, `cadquery`, `openscad`, `freecad` |
| `engines.coding` | `code-as-cad` |
| `engines.validation` | `geometry.manifold`, `units.consistent`, `export.step` |
| `skills.bundled` | `cad.text-to-cadquery`, `cad.parametric-review`, `cad.gdt-check` |
| `mcp.servers` | `freecad-mcp`, `blender-mcp` |
| `tools.lateral.open` | `freecad`, `openscad`, `zoo` |
| `artifacts.types` | `step`, `stl`, `render.png`, `drawing.pdf` |
| `policy.sandbox` | `container` |

## Lateral tools

```bash
atlas domain probe cad            # is each tool installed?
atlas domain guide cad freecad    # install recipe
atlas domain open cad openscad -- --version
```

When a tool is missing, `probe`/`open` print the install guide — never a silent
no-op (RFC 28 §I discipline). See the RFC 64 §3 table for the lateral tool set.

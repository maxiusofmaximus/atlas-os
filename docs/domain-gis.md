# Domain pack — `gis`

> Operator guide for the civil / topography / GIS domain pack (RFC 64).

## What it is

Routes geospatial work with geometry/CRS validation. OpenCADStudio and QGIS are
lateral references (RFC 64 §3).

## Manifest (`src-tauri/src/domain/packs/gis.toml`)

| Field | Value |
|---|---|
| `detect` | `*.shp`, `*.geojson`, `*.gpkg`, `*.qgz`, `*.qgs`, `qgis`, `opencadstudio` |
| `engines.validation` | `geometry.valid`, `crs.consistent`, `export.geojson` |
| `skills.bundled` | `gis.layer-review`, `gis.crs-check` |
| `tools.lateral.open` | `qgis` |
| `artifacts.types` | `geojson`, `shp`, `render.png` |
| `policy.sandbox` | `container` |

## Lateral tools

```bash
atlas domain probe gis
atlas domain guide gis qgis
```

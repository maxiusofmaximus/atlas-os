#requires -Version 5.1
<#
    Laya re-audit (Phase 13 unblock check).

    Queries crates.io + the GitHub repo for `laya` and writes dated evidence to
    `Atlas OS/research/laya_reaudit_<yyyy-MM-dd>.md`.

    Criterion (README / 20 - Roadmap.md §Fase 13):
      * crate version > 0.2.x, OR
      * >= 2 maintainers/contributors.
    rand/tokenizers upstream fixes are recorded as context, not as an unlock.

    Read-only against the network; writes exactly one markdown file.
#>
param(
    [string]$Repo = 'aovestdipaperino/laya-rust',
    [string]$Crate = 'laya'
)

$ErrorActionPreference = 'Stop'
$ua = @{ 'User-Agent' = 'atlas-os-laya-reaudit' }
$date = Get-Date -Format 'yyyy-MM-dd'
$researchDir = Join-Path $PSScriptRoot '..\Atlas OS\research'
$out = Join-Path $researchDir "laya_reaudit_$date.md"

$c = Invoke-RestMethod -Uri "https://crates.io/api/v1/crates/$Crate" -Headers $ua -TimeoutSec 40
$newest = [string]$c.crate.newest_version
$crateUpdated = [string]$c.crate.updated_at
$downloads = [string]$c.crate.downloads

$r = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo" -Headers $ua -TimeoutSec 40
$contribCount = 0
$contribList = ''
for ($i = 0; $i -lt 4; $i++) {
    $got = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/contributors?per_page=100" -Headers $ua -TimeoutSec 40
    if ($null -eq $got) { $got = @() }
    $n = @($got).Count
    if ($n -gt $contribCount) {
        $contribCount = $n
        $contribList = (@($got) | ForEach-Object { "$($_.login)($($_.contributions))" }) -join ', '
    }
    if ($contribCount -ge 2) { break }
    Start-Sleep -Seconds 2
}

$versionOk = $false
try { $versionOk = ([version]$newest) -gt ([version]'0.2.0') } catch { $versionOk = $false }
$maintOk = $contribCount -ge 2
$unlocked = $versionOk -or $maintOk

$toml = ''
try {
    $toml = (Invoke-WebRequest -Uri "https://raw.githubusercontent.com/$Repo/$($r.default_branch)/Cargo.toml" -Headers $ua -TimeoutSec 30 -UseBasicParsing).Content
} catch { $toml = '' }
$tomlLines = $toml -split "`n"
$tokenizersLine = ((@($tomlLines) | Where-Object { $_ -match '^\s*tokenizers\s*=' } | Select-Object -First 1) -replace '\s+$','')
$randLine = ((@($tomlLines) | Where-Object { $_ -match '^\s*rand\s*=' } | Select-Object -First 1) -replace '\s+$','')
if (-not $tokenizersLine) { $tokenizersLine = 'no declarado' }
if (-not $randLine) { $randLine = 'no declarado (sub-problema resuelto upstream)' }

$versionCell = if ($versionOk) { 'SI' } else { 'NO' }
$maintCell = if ($maintOk) { 'SI' } else { 'NO' }
$verdict = if ($unlocked) { 'criterio CUMPLIDO en al menos un eje -> revisar Phase 13 (requiere decision del operador)' } else { 'criterio NO cumplido -> Phase 13 sigue BLOQUEADA' }

$text = @"
# Laya re-audit - $date

Generado por ``scripts/laya_reaudit.ps1``. Fuente: crates.io API
(``https://crates.io/api/v1/crates/$Crate``) y GitHub API (``$Repo``).

## Evidencia

| Criterio | Valor observado | Cumple |
|---|---|---|
| ``$Crate`` > 0.2.x | ``newest_version = $newest`` (updated $crateUpdated, downloads $downloads) | $versionCell |
| Mantenedores >= 2 | $contribCount contributors: $contribList | $maintCell |
| Repo vivo | pushed_at $($r.pushed_at), archived $($r.archived), stars $($r.stargazers_count), forks $($r.forks_count), open_issues $($r.open_issues_count) | contexto |

## Veredicto

**$verdict**

Nota: el eje "mantenedores" se aproxima con el recuento de *contributors*
(autores de commits) que expone la API publica de GitHub. No equivale a
colaboradores con permiso de escritura; si ese recuento cambia el veredicto,
confirmar el write-access antes de desbloquear Phase 13.

## rand / tokenizers

| Dependencia upstream (`Cargo.toml`, rama $($r.default_branch)) | Linea observada |
|---|---|
| tokenizers | ``$tokenizersLine`` |
| rand | ``$randLine`` |

El sub-problema original (rand/tokenizers) no forma parte del unlock por si
solo, pero se registra cada re-audit para no perder el contexto.
"@

[System.IO.File]::WriteAllText($out, $text, (New-Object System.Text.UTF8Encoding($false)))
Write-Host "wrote $out"
Write-Host "version=$newest versionOk=$versionOk contributors=$contribCount maintOk=$maintOk unlocked=$unlocked"

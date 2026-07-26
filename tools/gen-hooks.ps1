# gen-hooks.ps1 — Emit CLI-specific hooks.json variants from hooks.template.json
#
# Source: microsoft/intelligent-terminal wt-agent-hooks (MIT).
# Pattern ported per RFC 28 §B Fase 0 (item IT-003).
# Copyright (c) Microsoft Corporation. Schema attribution preserved in output files.
#
# Usage:
#   pwsh tools/gen-hooks.ps1                           # write 5 files under src-tauri/hooks/<cli>/
#   pwsh tools/gen-hooks.ps1 -CLIs claude,opencode     # only specified CLIs
#   pwsh tools/gen-hooks.ps1 -DryRun                   # print to stdout only
#
# Output: src-tauri/hooks/<cli>/hooks.json for <cli> in claude, copilot, codex, gemini, opencode.
# Each file is the template with __CLI__ replaced by the actual CLI name.

[CmdletBinding()]
param(
    [string[]]$CLIs = @('claude', 'copilot', 'codex', 'gemini', 'opencode'),
    [switch]$DryRun
)

$ErrorActionPreference = 'Stop'

$repoRoot = Split-Path -Parent $PSScriptRoot
$templatePath = Join-Path $repoRoot 'src-tauri\hooks\hooks.template.json'

if (-not (Test-Path -LiteralPath $templatePath)) {
    throw "Template not found: $templatePath"
}

$templateRaw = Get-Content -LiteralPath $templatePath -Raw
$templateObj = $templateRaw | ConvertFrom-Json -Depth 32
if (-not $templateObj.hooks) {
    throw 'Template is missing the .hooks property — refusing to emit.'
}

$count = 0
foreach ($cli in $CLIs) {
    $cliTrim = $cli.Trim().ToLowerInvariant()
    if ($cliTrim -notmatch '^[a-z0-9_-]+$') {
        throw "Invalid CLI name: '$cliTrim' (must match ^[a-z0-9_-]+$)"
    }
    $output = $templateRaw.Replace('__CLI__', $cliTrim)
    if ($DryRun) {
        Write-Host "=== $cliTrim ==="
        Write-Host $output
        continue
    }
    $outDir = Join-Path $repoRoot "src-tauri\hooks\$cliTrim"
    if (-not (Test-Path -LiteralPath $outDir)) {
        New-Item -ItemType Directory -Path $outDir -Force | Out-Null
    }
    $outFile = Join-Path $outDir 'hooks.json'
    Set-Content -LiteralPath $outFile -Value $output -Encoding utf8 -NoNewline:$false
    $count++
    Write-Host "  wrote $outFile"
}

if (-not $DryRun) {
    Write-Host "Done. $count file(s) emitted under src-tauri/hooks/<cli>/hooks.json"
}

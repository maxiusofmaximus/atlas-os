# Atlas OS - Toast smoke-test script (RFC 28 sec F item 9)
#
# Verifies the Windows Toast queue surface end-to-end using only the SQLite
# queue (the WinRT dispatch itself is owned by the desktop process - see
# `docs/toast-integration.md`), so this runs anywhere:
#
#   1. Confirms `atlas toast queue|list|cancel` is compiled in (the `toast`
#      feature; OFF by default). If absent, every step is SKIPPED with a
#      rebuild hint.
#   2. Enqueues a `model_ready` toast and parses the JSON `id` from the output.
#   3. Lists the queue and finds the row we just queued.
#   4. Cancels it (idempotent): first cancel returns `"cancelled":true`, a
#      second returns `"cancelled":false`.
#
# Usage:
#   tools/toast-smoke.ps1
#   tools/toast-smoke.ps1 -Bin .\src-tauri\target\release\atlas.exe
#
# Exits 0 when every assertion passed (or was legitimately skipped), non-zero
# on the first real failure.

[CmdletBinding()]
param(
    [string]$Bin = ".\src-tauri\target\debug\atlas.exe"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-True($cond, $msg) {
    if (-not $cond) {
        Write-Host "FAIL: $msg" -ForegroundColor Red
        exit 1
    }
    Write-Host ". $msg" -ForegroundColor DarkGreen
}

function Assert-Contains($haystack, $needle, $msg) {
    if ($haystack -notlike "*$needle*") {
        Write-Host "FAIL: $msg" -ForegroundColor Red
        Write-Host "Expected substring: $needle" -ForegroundColor Red
        exit 1
    }
    Write-Host ". $msg" -ForegroundColor DarkGreen
}

function Skip($msg) { Write-Host "SKIP: $msg" -ForegroundColor Yellow }

if (-not (Test-Path -LiteralPath $Bin)) {
    Write-Host "FAIL: atlas binary not found at: $Bin" -ForegroundColor Red
    Write-Host "Hint: run 'cargo build --manifest-path src-tauri/Cargo.toml --bin atlas --features toast' first." -ForegroundColor Yellow
    exit 2
}

Write-Host "Atlas OS - toast smoke test" -ForegroundColor Cyan
Write-Host "Binary: $Bin"

$hadProfile = $false
$priorProfile = $env:OC_PROFILE
if ($null -ne $priorProfile) { $hadProfile = $true }

$scratch = "oc_smoke_toast_$([guid]::NewGuid().ToString('N').Substring(0,8))"
$scratchRoot = Join-Path $env:USERPROFILE ".opencode\profiles\$scratch"
$env:OC_PROFILE = $scratch
New-Item -ItemType Directory -Force $scratchRoot | Out-Null

try {
    # ---------- 1. Feature gate ----------
    # Probe by exit code via cmd.exe: a build without the `toast` feature makes
    # clap exit non-zero with a stderr message, which under PS 5.1 trips
    # $ErrorActionPreference=Stop if invoked directly.
    Write-Host "`n[1/4] Checking 'atlas toast' namespace..." -ForegroundColor Cyan
    $null = cmd /c "`"$Bin`" toast list -n 1 >nul 2>nul"
    if ($LASTEXITCODE -ne 0) {
        Skip "the 'toast' feature is not compiled in (rebuild with --features toast)"
    } else {
        Write-Host ". toast queue/list/cancel present" -ForegroundColor DarkGreen

        # ---------- 2. Enqueue ----------
        Write-Host "`n[2/4] Enqueuing a model_ready toast..." -ForegroundColor Cyan
        $q = (& $Bin toast queue model_ready --title "smoke" --body "hello" 2>&1 | Out-String).Trim()
        Assert-True ($LASTEXITCODE -eq 0) "toast queue returned exit code 0 (got $LASTEXITCODE)"
        Assert-Contains $q '"queued":true' "queue replied queued:true"
        $m = [regex]::Match($q, '"id":(\d+)')
        Assert-True $m.Success "queue output carries a numeric id"
        $id = $m.Groups[1].Value

        # ---------- 3. List ----------
        Write-Host "`n[3/4] Listing the queue..." -ForegroundColor Cyan
        $l = (& $Bin toast list -n 5 2>&1 | Out-String)
        Assert-True ($LASTEXITCODE -eq 0) "toast list returned exit code 0 (got $LASTEXITCODE)"
        Assert-Contains $l ('"id":' + $id) "queued row $id is listed"
        Assert-Contains $l '"title":"smoke"' "listed row carries the title"

        # ---------- 4. Cancel (idempotent) ----------
        Write-Host "`n[4/4] Cancelling the toast..." -ForegroundColor Cyan
        $c1 = (& $Bin toast cancel $id 2>&1 | Out-String)
        Assert-Contains $c1 '"cancelled":true' "first cancel cancels the row"
        $c2 = (& $Bin toast cancel $id 2>&1 | Out-String)
        Assert-Contains $c2 '"cancelled":false' "second cancel is a no-op (idempotent)"
    }
} finally {
    if ($hadProfile) { $env:OC_PROFILE = $priorProfile }
    else { Remove-Item Env:\OC_PROFILE -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "`nPASS: toast smoke test completed" -ForegroundColor Green
exit 0

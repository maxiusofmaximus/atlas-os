# Atlas OS - Secrets (OS keychain) smoke-test script (RFC 25 sec 3.10)
#
# Exercises the real OS credential store end-to-end via the CLI (the unit
# tests only touch the read-only path, to avoid writing to the user's
# keychain):
#
#   1. `atlas secrets set <acct>` reads the value from stdin (not echoed).
#   2. `atlas secrets get <acct>` prints a masked preview (not the raw value).
#   3. `atlas secrets get <acct> --show` prints the raw value.
#   4. `atlas secrets list` shows the account as [stored].
#   5. `atlas secrets delete <acct>` removes it; get -> no entry; second
#      delete -> "no entry to delete".
#
# A unique, random account name is used so the smoke never clobbers a real
# provider key; it is deleted at the end.
#
# Usage:
#   tools/secrets-smoke.ps1
#   tools/secrets-smoke.ps1 -Bin .\src-tauri\target\release\atlas.exe
#
# Exits 0 on full pass, non-zero on first failure.

[CmdletBinding()]
param(
    [string]$Bin = ".\src-tauri\target\debug\atlas.exe"
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Assert-True($cond, $msg) {
    if (-not $cond) { Write-Host "FAIL: $msg" -ForegroundColor Red; exit 1 }
    Write-Host ". $msg" -ForegroundColor DarkGreen
}

function Assert-Contains($haystack, $needle, $msg) {
    if ($haystack -notlike "*$needle*") {
        Write-Host "FAIL: $msg (missing: $needle)" -ForegroundColor Red
        exit 1
    }
    Write-Host ". $msg" -ForegroundColor DarkGreen
}

if (-not (Test-Path -LiteralPath $Bin)) {
    Write-Host "FAIL: atlas binary not found at: $Bin" -ForegroundColor Red
    Write-Host "Hint: cargo build --manifest-path src-tauri/Cargo.toml --bin atlas" -ForegroundColor Yellow
    exit 2
}

Write-Host "Atlas OS - secrets (keychain) smoke test" -ForegroundColor Cyan
Write-Host "Binary: $Bin"

$acct = "atlas-smoke-$([guid]::NewGuid().ToString('N').Substring(0,8))"
$value = "sk-smoke-$([guid]::NewGuid().ToString('N').Substring(0,12))"
$scratch = "oc_smoke_sec_$([guid]::NewGuid().ToString('N').Substring(0,8))"
$scratchRoot = Join-Path $env:USERPROFILE ".opencode\profiles\$scratch"
$hadProfile = $null -ne $env:OC_PROFILE
$prior = $env:OC_PROFILE
$env:OC_PROFILE = $scratch
New-Item -ItemType Directory -Force $scratchRoot | Out-Null

try {
    Write-Host "`n[1/3] set (value on stdin)..." -ForegroundColor Cyan
    $out = ($value | & $Bin secrets set $acct 2>&1 | Out-String)
    Assert-True ($LASTEXITCODE -eq 0) "set returned exit 0"
    Assert-Contains $out $acct "set reports the account"

    Write-Host "`n[2/3] get (masked) + get --show..." -ForegroundColor Cyan
    $masked = (& $Bin secrets get $acct 2>&1 | Out-String)
    Assert-Contains $masked $acct "get names the account"
    Assert-True ($masked -notlike "*$value*") "masked get does NOT leak the raw value"
    $shown = (& $Bin secrets get $acct --show 2>&1 | Out-String).Trim()
    Assert-True ($shown -eq $value) "get --show returns the exact value"
    $list = (& $Bin secrets list 2>&1 | Out-String)
    Assert-Contains $list $acct "list includes the account"
    Assert-Contains $list "[stored]" "list marks it stored"

    Write-Host "`n[3/3] delete + idempotence..." -ForegroundColor Cyan
    $del = (& $Bin secrets delete $acct 2>&1 | Out-String)
    Assert-Contains $del "deleted" "first delete removes it"
    $after = (& $Bin secrets get $acct 2>&1 | Out-String)
    Assert-Contains $after "no entry" "get after delete reports no entry"
    $del2 = (& $Bin secrets delete $acct 2>&1 | Out-String)
    Assert-Contains $del2 "no entry to delete" "second delete is a no-op"
} finally {
    # Best-effort cleanup in case an assertion aborted mid-way.
    if ($acct) { & $Bin secrets delete $acct 2>&1 | Out-Null }
    if ($hadProfile) { $env:OC_PROFILE = $prior } else { Remove-Item Env:\OC_PROFILE -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "`nPASS: secrets smoke test completed" -ForegroundColor Green
exit 0

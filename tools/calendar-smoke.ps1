# Atlas OS - Calendar smoke-test script (RFC 28 sec G item 10)
#
# Drives the G subsystem end-to-end so a broken build fails loudly instead
# of shipping a silent no-op:
#
#   1. Confirms the `atlas calendar` namespace and the always-present verbs
#      (`feed`, `busy`, `availability`, `policy`, `proactive`) are wired.
#   2. Runs `atlas calendar feed` twice on a scratch profile and asserts the
#      printed subscription URL is byte-identical - the ICS token is minted
#      once and persisted (`calendar_ics_token.txt`), never regenerated.
#   3. Asserts the persisted token is `base64url(16 bytes)` = 22 URL-safe
#      chars (RFC 28 G.2) and that the profile DB accepts `busy count`.
#   4. (Live) Attaches to an ALREADY-RUNNING HUD via the active profile: reads
#      the `webcal://` URL from `calendar feed`, fetches
#      `GET /atlas-calendar.ics?token=...` and validates the RFC 5545 skeleton
#      (`BEGIN:VCALENDAR` / `VERSION:2.0` / `END:VCALENDAR`). A wrong token
#      must be rejected with 401. If no HUD is running, or the route returns
#      404 (build lacks `calendar-ics`), the step is SKIPPED - not failed.
#
# NOTE ON SPAWNING: the HUD axum server is owned by the Tauri desktop process
# (`atlas-os-desktop`); the headless CLI has no `--serve` and `atlas hud` only
# *reads* the published port. This smoke therefore attaches to a running HUD
# rather than spawning one.
#
# The Graph READ verbs (`login`/`sync`/`status`) and the `.ics` subscription
# verbs (`sync-ics`/`subscribe`/`subscriptions`/`sync-all`) are feature-gated
# and only advertised when `calendar-graph` / `calendar-ics` are compiled in;
# the script reports which are present rather than requiring them.
#
# Usage:
#   tools/calendar-smoke.ps1
#   tools/calendar-smoke.ps1 -Bin .\src-tauri\target\release\atlas.exe
#   tools/calendar-smoke.ps1 -Live        # also fetch the feed from a running HUD
#
# Exits 0 when every assertion passed (or was legitimately skipped), non-zero
# on the first real failure.

[CmdletBinding()]
param(
    [string]$Bin = ".\src-tauri\target\debug\atlas.exe",
    [switch]$Live
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

# ---------- 0. Preflight ----------
if (-not (Test-Path -LiteralPath $Bin)) {
    Write-Host "FAIL: atlas binary not found at: $Bin" -ForegroundColor Red
    Write-Host "Hint: run 'cargo build --manifest-path src-tauri/Cargo.toml --bin atlas --features calendar' first." -ForegroundColor Yellow
    exit 2
}

Write-Host "Atlas OS - calendar smoke test" -ForegroundColor Cyan
Write-Host "Binary: $Bin"

$hadProfile = $false
$priorProfile = $env:OC_PROFILE
if ($null -ne $priorProfile) { $hadProfile = $true }

$scratch = "oc_smoke_cal_$([guid]::NewGuid().ToString('N').Substring(0,8))"
$scratchRoot = Join-Path $env:USERPROFILE ".opencode\profiles\$scratch"
$env:OC_PROFILE = $scratch
New-Item -ItemType Directory -Force $scratchRoot | Out-Null

function Restore-Profile {
    if ($hadProfile) { $env:OC_PROFILE = $priorProfile }
    else { Remove-Item Env:\OC_PROFILE -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

try {
    # ---------- 1. Namespace + always-present verbs ----------
    Write-Host "`n[1/4] Checking 'atlas calendar' namespace..." -ForegroundColor Cyan
    $help = & $Bin calendar --help 2>&1 | Out-String
    Assert-Contains $help "feed"         "calendar feed verb present"
    Assert-Contains $help "busy"         "calendar busy verb present"
    Assert-Contains $help "availability" "calendar availability verb present"
    Assert-Contains $help "policy"       "calendar policy verb present"
    Assert-Contains $help "proactive"    "calendar proactive verb present"

    $hasGraph = ($help -match '\blogin\b')
    $hasIcsSubs = ($help -match 'sync-ics')
    if ($hasGraph) {
        Write-Host ". calendar-graph compiled in (login/sync/status)" -ForegroundColor DarkGreen
    } else {
        Skip "calendar-graph not compiled (login/sync/status absent)"
    }
    if ($hasIcsSubs) {
        Write-Host ". calendar-ics subscriptions compiled in (sync-ics/subscribe/sync-all)" -ForegroundColor DarkGreen
    } else {
        Skip "calendar-ics subscriptions not compiled (sync-ics/subscribe/sync-all absent)"
    }

    # ---------- 2. feed is idempotent (token persisted, not regenerated) ----------
    Write-Host "`n[2/4] Checking 'calendar feed' is idempotent..." -ForegroundColor Cyan
    $feed1 = (& $Bin calendar feed 2>&1 | Out-String).Trim()
    Assert-True ($LASTEXITCODE -eq 0) "calendar feed returned exit code 0 (got $LASTEXITCODE)"
    $feed2 = (& $Bin calendar feed 2>&1 | Out-String).Trim()
    Assert-True ($feed1 -eq $feed2) "calendar feed is byte-identical across runs (token persisted)"
    Assert-Contains $feed1 "atlas-calendar.ics" "feed advertises the /atlas-calendar.ics endpoint"

    # ---------- 3. Token format + schema reachable ----------
    Write-Host "`n[3/4] Checking ICS token format + busy-window storage..." -ForegroundColor Cyan
    $tokenFile = Join-Path $scratchRoot "calendar_ics_token.txt"
    Assert-True (Test-Path -LiteralPath $tokenFile) "token persisted at calendar_ics_token.txt"
    $token = (Get-Content -LiteralPath $tokenFile -Raw).Trim()
    Assert-True ($token.Length -eq 22) "token is 22 base64url chars (got $($token.Length))"
    Assert-True ($token -cmatch '^[A-Za-z0-9_-]+$') "token is URL-safe (base64url, RFC 4648 section 5)"

    $null = & $Bin calendar busy count 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "calendar busy count returned exit code 0 (M18 schema reachable)"

    # ---------- 4. (Live) RFC 5545 feed from an already-running HUD ----------
    if ($Live) {
        Write-Host "`n[4/4] Live: fetching the ICS feed from the active profile's HUD..." -ForegroundColor Cyan
        # The HUD is owned by the desktop process; query the ACTIVE profile's
        # feed URL (unset the scratch profile for this probe only).
        Remove-Item Env:\OC_PROFILE -ErrorAction SilentlyContinue
        $activeFeed = (& $Bin calendar feed 2>&1 | Out-String).Trim()

        $m = [regex]::Match($activeFeed, 'webcal://127\.0\.0\.1:(\d+)/atlas-calendar\.ics\?token=([A-Za-z0-9_-]+)')
        if (-not $m.Success) {
            Skip "no running HUD for the active profile (start atlas-os-desktop, then re-run -Live)"
        } else {
            $port = $m.Groups[1].Value
            $liveToken = $m.Groups[2].Value
            $url = "http://127.0.0.1:$port/atlas-calendar.ics?token=$liveToken"
            $liveFetched = $false
            try {
                $resp = Invoke-WebRequest -Uri $url -UseBasicParsing -TimeoutSec 10
                Assert-Contains $resp.Content "BEGIN:VCALENDAR" "feed body has BEGIN:VCALENDAR"
                Assert-Contains $resp.Content "VERSION:2.0"      "feed body has VERSION:2.0"
                Assert-Contains $resp.Content "END:VCALENDAR"    "feed body has END:VCALENDAR"
                $liveFetched = $true
            } catch {
                $status = $null
                if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
                if ($status -eq 404) {
                    Skip "GET /atlas-calendar.ics returned 404 - rebuild with --features calendar-ics to exercise the RFC 5545 feed"
                } else {
                    throw
                }
            }

            # The auth gate must reject a wrong token.
            if ($liveFetched) {
                $badUrl = "http://127.0.0.1:$port/atlas-calendar.ics?token=wrongtokenthatis22ch"
                try {
                    Invoke-WebRequest -Uri $badUrl -UseBasicParsing -TimeoutSec 10 | Out-Null
                    Assert-True $false "wrong token should not return 200"
                } catch {
                    $status = $null
                    if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
                    Assert-True ($status -eq 401) "wrong token gives 401 (got $status)"
                }
            }
        }
    } else {
        Write-Host "`n[4/4] Live feed check skipped (pass -Live to enable)" -ForegroundColor DarkGray
    }
} finally {
    Restore-Profile
}

Write-Host "`nPASS: calendar smoke test completed" -ForegroundColor Green
exit 0

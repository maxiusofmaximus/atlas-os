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
#   4. (Live) SPAWNS the headless HUD with `atlas serve` (RFC 29 3.A - a real
#      daemon that runs the axum server without the Tauri webview), waits for
#      `hud_port.txt`, fetches `GET /atlas-calendar.ics?token=...` and
#      validates the RFC 5545 skeleton (`BEGIN:VCALENDAR` / `VERSION:2.0` /
#      `END:VCALENDAR`). If the route returns 404 the build lacks
#      `calendar-ics`, and the step is SKIPPED with a rebuild hint.
#   5. (Live) A wrong token must be rejected with 401.
#
# The Graph READ verbs (`login`/`sync`/`status`) and the `.ics` subscription
# verbs (`sync-ics`/`subscribe`/`subscriptions`/`sync-all`) are feature-gated
# and only advertised when `calendar-graph` / `calendar-ics` are compiled in;
# the script reports which are present rather than requiring them.
#
# Usage:
#   tools/calendar-smoke.ps1
#   tools/calendar-smoke.ps1 -Bin .\src-tauri\target\release\atlas.exe
#   tools/calendar-smoke.ps1 -Live        # also spawn `atlas serve` + fetch the feed
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
$hudJob = $null
$port = $null

try {
    # ---------- 1. Namespace + always-present verbs ----------
    Write-Host "`n[1/5] Checking 'atlas calendar' namespace..." -ForegroundColor Cyan
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
    Write-Host "`n[2/5] Checking 'calendar feed' is idempotent..." -ForegroundColor Cyan
    $feed1 = (& $Bin calendar feed 2>&1 | Out-String).Trim()
    Assert-True ($LASTEXITCODE -eq 0) "calendar feed returned exit code 0 (got $LASTEXITCODE)"
    $feed2 = (& $Bin calendar feed 2>&1 | Out-String).Trim()
    Assert-True ($feed1 -eq $feed2) "calendar feed is byte-identical across runs (token persisted)"
    Assert-Contains $feed1 "atlas-calendar.ics" "feed advertises the /atlas-calendar.ics endpoint"

    # ---------- 3. Token format + schema reachable ----------
    Write-Host "`n[3/5] Checking ICS token format + busy-window storage..." -ForegroundColor Cyan
    $tokenFile = Join-Path $scratchRoot "calendar_ics_token.txt"
    Assert-True (Test-Path -LiteralPath $tokenFile) "token persisted at calendar_ics_token.txt"
    $token = (Get-Content -LiteralPath $tokenFile -Raw).Trim()
    Assert-True ($token.Length -eq 22) "token is 22 base64url chars (got $($token.Length))"
    Assert-True ($token -cmatch '^[A-Za-z0-9_-]+$') "token is URL-safe (base64url, RFC 4648 section 5)"

    $null = & $Bin calendar busy count 2>&1 | Out-String
    Assert-True ($LASTEXITCODE -eq 0) "calendar busy count returned exit code 0 (M18 schema reachable)"

    # ---------- 4. (Live) spawn the HUD and validate the RFC 5545 feed ----------
    $liveFetched = $false
    if ($Live) {
        Write-Host "`n[4/5] Live: spawning 'atlas serve' and fetching the ICS feed..." -ForegroundColor Cyan
        $hudJob = Start-Job -ScriptBlock {
            param($exe, $prof)
            $env:OC_PROFILE = $prof
            & $exe serve --host 127.0.0.1 --port 0
        } -ArgumentList (Resolve-Path $Bin).Path, $scratch

        # Wait up to ~20 s for the daemon to publish its ephemeral port.
        $portFile = Join-Path $scratchRoot "hud_port.txt"
        for ($i = 0; $i -lt 40; $i++) {
            if (Test-Path -LiteralPath $portFile) {
                $candidate = (Get-Content -LiteralPath $portFile -Raw).Trim()
                if ($candidate -match '^\d+$' -and [int]$candidate -gt 0) { $port = $candidate; break }
            }
            Start-Sleep -Milliseconds 500
        }
        Assert-True ($port -match '^\d+$') "atlas serve published a port to hud_port.txt (got '$port')"

        $url = "http://127.0.0.1:$port/atlas-calendar.ics?token=$token"
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
    } else {
        Write-Host "`n[4/5] Live HUD + feed check skipped (pass -Live to enable)" -ForegroundColor DarkGray
    }

    # ---------- 5. (Live) the auth gate must reject a wrong token ----------
    if ($liveFetched) {
        Write-Host "`n[5/5] Auth gate: wrong token must be rejected..." -ForegroundColor Cyan
        $badUrl = "http://127.0.0.1:$port/atlas-calendar.ics?token=wrongtokenthatis22ch"
        try {
            Invoke-WebRequest -Uri $badUrl -UseBasicParsing -TimeoutSec 10 | Out-Null
            Assert-True $false "wrong token should not return 200"
        } catch {
            $status = $null
            if ($_.Exception.Response) { $status = [int]$_.Exception.Response.StatusCode }
            Assert-True ($status -eq 401) "wrong token gives 401 (got $status)"
        }
    } else {
        Write-Host "`n[5/5] Auth-gate check skipped (needs -Live + a calendar-ics build)" -ForegroundColor DarkGray
    }
} finally {
    if ($hudJob) { Stop-Job $hudJob -ErrorAction SilentlyContinue; Remove-Job $hudJob -Force -ErrorAction SilentlyContinue }
    if ($hadProfile) { $env:OC_PROFILE = $priorProfile }
    else { Remove-Item Env:\OC_PROFILE -ErrorAction SilentlyContinue }
    if (Test-Path -LiteralPath $scratchRoot) {
        Remove-Item -LiteralPath $scratchRoot -Recurse -Force -ErrorAction SilentlyContinue
    }
}

Write-Host "`nPASS: calendar smoke test completed" -ForegroundColor Green
exit 0

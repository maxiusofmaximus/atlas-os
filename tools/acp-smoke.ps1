# Atlas OS — ACP smoke-test script (RFC 28 §B item 8)
#
# Pipes a sequence of JSON-RPC 2.0 requests at the Atlas OS ACP
# host loop on stdin, drives the corresponding responses + notifications
# on stdout, and asserts that the protocol contract holds. In Phase
# 1.5d the host loop is the `agent-client-protocol::Stdio` builtin
# served by `atlas_os::acp::run_server`; we invoke it via
# ATLAS_ACP_FORCE=1 so the smoke test does not require IT to be
# installed on the box.
#
# Usage:
#   tools/acp-smoke.ps1
#   tools/acp-smoke.ps1 -Bin .\src-tauri\target\release\opencode.exe
#
# Exits 0 when every assertion passed, non-zero on first failure.

[CmdletBinding()]
param(
    [string]$Bin = ".\src-tauri\target\debug\opencode.exe",
    [switch]$Trace
)

$ErrorActionPreference = "Stop"
Set-StrictMode -Version Latest

function Send-JsonRpc($proc, $msg) {
    $line = $msg | ConvertTo-Json -Compress -Depth 8
    if ($Trace) { Write-Host "-> $line" -ForegroundColor DarkGray }
    $proc.StandardInput.WriteLine($line)
}

function Wait-JsonRpc($proc, [int]$TimeoutMs = 2000) {
    $deadline = [DateTime]::UtcNow.AddMilliseconds($TimeoutMs)
    while (-not $proc.StandardOutput.EndOfStream -and [DateTime]::UtcNow -lt $deadline) {
        $line = $proc.StandardOutput.ReadLine()
        if ($null -ne $line) {
            if ($Trace) { Write-Host "<- $line" -ForegroundColor DarkGray }
            try { return $line | ConvertFrom-Json } catch { return $null }
        } else {
            Start-Sleep -Milliseconds 50
        }
    }
    return $null
}

function Assert-True($cond, $msg) {
    if (-not $cond) {
        Write-Host "FAIL: $msg" -ForegroundColor Red
        exit 1
    }
    Write-Host "ok  - $msg" -ForegroundColor Green
}

if (-not (Test-Path $Bin)) {
    Write-Host "FAIL: $Bin not found. Run \`pnpm tauri:build -- --features acp-server\` first." -ForegroundColor Red
    exit 2
}

$env:ATLAS_ACP_FORCE = "1"
$psi = New-Object System.Diagnostics.ProcessStartInfo
$psi.FileName = (Resolve-Path $Bin).Path
$psi.RedirectStandardInput = $true
$psi.RedirectStandardOutput = $true
$psi.RedirectStandardError = $true
$psi.UseShellExecute = $false
$psi.CreateNoWindow = $true
$psi.EnvironmentVariables["ATLAS_ACP_FORCE"] = "1"
$proc = [System.Diagnostics.Process]::Start($psi)

try {
    Send-JsonRpc $proc @{
        jsonrpc = "2.0"
        id      = 1
        method  = "initialize"
        params  = @{
            protocolVersion = 1
            clientCapabilities = @{}
        }
    }
    $resp = Wait-JsonRpc $proc
    Assert-True ($null -ne $resp) "initialize returned a JSON-RPC envelope"
    Assert-True ($resp.id -eq 1) "initialize response id matches request"
    Assert-True ($resp.result.agentInfo.name -eq "opencode") "agentInfo.name=`"opencode`""
    Assert-True ($resp.result.agentInfo.title -eq "Atlas OS") "agentInfo.title=`"Atlas OS`""

    Send-JsonRpc $proc @{
        jsonrpc = "2.0"
        id      = 2
        method  = "session/new"
        params  = @{
            cwd        = (Get-Location).Path
            mcpServers = @()
        }
    }
    $resp = Wait-JsonRpc $proc
    Assert-True ($resp.id -eq 2) "session/new response id matches"
    Assert-True ($null -ne $resp.result.sessionId) "session/new returns a sessionId"

    $notif = Wait-JsonRpc $proc
    Assert-True ($notif.method -eq "session/update") "first session/update after session/new"
    $cmds = $notif.params.update.availableCommands
    Assert-True ($cmds.Count -eq 6) "available_commands_update advertises 6 commands"
    Assert-True ($cmds[0].name -eq "opencode mission new") "first command is `"/opencode mission new`""
    Assert-True ($cmds[4].name -eq "opencode fix")        "fifth command is `"/opencode fix`""

    Send-JsonRpc $proc @{
        jsonrpc = "2.0"
        id      = 3
        method  = "session/prompt"
        params  = @{
            sessionId = $resp.result.sessionId
            prompt = @(@{ type = "text"; text = "/opencode fix add more guards" })
        }
    }
    $promptNotif = Wait-JsonRpc $proc
    if ($promptNotif.method -eq "session/update" -and $promptNotif.params.update.sessionUpdate -eq "agent_message_chunk") {
        Assert-True $true "session/prompt streams agent_message_chunk"
    } else {
        Assert-True $false "session/prompt must stream an agent_message_chunk first"
    }
    $promptResp = Wait-JsonRpc $proc
    Assert-True ($promptResp.id -eq 3) "session/prompt response id matches"
    Assert-True ($promptResp.result.stopReason -eq "refusal") "Phase 1.5d refusal is the documented stub behaviour"

    Send-JsonRpc $proc @{
        jsonrpc = "2.0"
        id      = 4
        method  = "session/set_mode"
        params  = @{
            sessionId = $resp.result.sessionId
            modeId    = "architect"
        }
    }
    $setModeResp = Wait-JsonRpc $proc
    Assert-True ($setModeResp.id -eq 4) "session/set_mode response id matches"
    $setModeNotif = Wait-JsonRpc $proc
    if ($setModeNotif.method -eq "session/update" -and $setModeNotif.params.update.sessionUpdate -eq "current_mode_update") {
        Assert-True ($setModeNotif.params.update.currentModeId -eq "architect") "currentModeUpdate echoes back the requested mode"
    } else {
        Assert-True $false "session/set_mode must stream current_mode_update after the response"
    }

    Write-Host ""
    Write-Host "ALL ACP SMOKE-TEST ASSERTIONS PASSED" -ForegroundColor Green
    exit 0
}
finally {
    try {
        $proc.StandardInput.Close()
        if (-not $proc.HasExited) { $proc.Kill() }
    } catch {}
}

param(
    [string]$Dataset  = "terminal-bench/terminal-bench-2",
    [string]$Model    = "moonshotai/kimi-k3",
    [string]$Agent    = "harbor_atlas.atlas_agent:AtlasAgent",
    [int]$Concurrent  = 4,
    # Harbor execution backend: "local" (Docker via WSL2) or a cloud provider
    # ("daytona" | "vercel" | "modal" | "tensorlake") to skip local Docker.
    [string]$Env      = "local",
    [string]$Distro   = "Ubuntu-24.04"
)

$ErrorActionPreference = "Stop"

# Resolve the repo path as WSL sees it.
$RepoWin = (Resolve-Path "$PSScriptRoot\..\..").Path
$RepoWsl = "/mnt/" + ($RepoWin.Substring(0,1).ToLower()) + ($RepoWin.Substring(2).Replace('\','/'))

$envFlag = if ($Env -eq "local") { "" } else { "--env $Env" }

Write-Host "Harbor: dataset=$Dataset model=$Model agent=$Agent concurrent=$Concurrent env=$Env"
Write-Host "Repo (WSL): $RepoWsl"

# Run inside WSL2 (Docker Engine lives there). Keep the session alive so the job
# is not killed when the command returns.
$inner = @"
set -e
export PATH=`$HOME/.local/bin:`$PATH
pgrep dockerd >/dev/null || (sudo dockerd >/var/log/dockerd.log 2>&1 & sleep 6)
cd '$RepoWsl'
PYTHONPATH=tools harbor run -d '$Dataset' --agent '$Agent' --model '$Model' --n-concurrent $Concurrent $envFlag
"@

wsl -d $Distro bash -lc $inner

Write-Host ""
Write-Host "Ingest the baseline into Atlas with:"
Write-Host "  atlas eval import jobs/<job-id>"
Write-Host "  atlas eval report"

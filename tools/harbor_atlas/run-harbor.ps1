param(
    [string]$Dataset  = "terminal-bench@2.0",
    [string]$Model    = "anthropic/claude-opus-4-1",
    [string]$Agent    = "harbor_atlas.atlas_agent:AtlasAgent",
    [int]$Concurrent  = 4
)

$ErrorActionPreference = "Stop"
$env:PYTHONPATH = "tools"

Write-Host "Running Harbor: dataset=$Dataset model=$Model agent=$Agent concurrent=$Concurrent"
uv run harbor run --dataset $Dataset --agent $Agent --model $Model --n-concurrent $Concurrent

Write-Host "Ingest the baseline into Atlas with:"
Write-Host "  atlas eval import jobs/<job-id>"
Write-Host "  atlas eval report"

#!/usr/bin/env bash
set -e
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os

# 1) Atlas profile used inside the container, with the .env deployment.
ATLAS_BIN="/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas"
ENV_FILE="/mnt/c/Users/Max/Desktop/atlas-os/.env"

echo "=== atlas binary: $(file -b "$ATLAS_BIN") ==="

# 2) Mounts: put atlas on PATH and the .env in the working dir.
MOUNTS='[
  {"type":"bind","source":"'"$ATLAS_BIN"'","target":"/usr/local/bin/atlas","read_only":true},
  {"type":"bind","source":"'"$ENV_FILE"'","target":"/app/.env","read_only":true}
]'

echo "=== mounts: $MOUNTS ==="
echo "=== running Atlas adapter on ONE task (hello-world) ==="

# 3) One-task smoke: the simplest Terminal-Bench task.
PYTHONPATH=tools timeout 900 harbor run \
  -d terminal-bench/terminal-bench-2 \
  --agent harbor_atlas.atlas_agent:AtlasAgent \
  --model moonshotai/kimi-k3 \
  --include-task-name terminal-bench/log-summary-date-ranges \
  --n-concurrent 1 \
  --mounts "$MOUNTS" \
  -o jobs/atlas-smoke 2>&1 | tail -n 40

#!/usr/bin/env bash
# Run the Atlas agent on a few Terminal-Bench 2 tasks via OpenCode Go.
set -e
cd /mnt/c/Users/Max/Desktop/atlas-os
export PATH="$HOME/.local/bin:$PATH"
pgrep dockerd >/dev/null || (sudo dockerd >/var/log/dockerd.log 2>&1 & sleep 6)

BIN=/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/x86_64-unknown-linux-musl/release/atlas
chmod +x "$BIN"

PYTHONPATH=tools harbor run \
  -d terminal-bench/terminal-bench-2 \
  --agent harbor_atlas.atlas_agent:AtlasAgent \
  --model deepseek-v4.1-flash \
  --n-concurrent 2 \
  --n-tasks 3 \
  --env-file tools/harbor_atlas/.env.harbor \
  --allow-environment-host opencode.ai \
  --mounts "[{\"type\":\"bind\",\"source\":\"$BIN\",\"target\":\"/usr/local/bin/atlas\",\"read_only\":true}]" \
  -o jobs \
  -y

echo "=== latest job ==="
LATEST=$(ls -1t jobs | head -1)
echo "job=$LATEST"
echo "--- result.json (head) ---"
head -c 800 "jobs/$LATEST/result.json" 2>/dev/null || echo "(no result.json)"

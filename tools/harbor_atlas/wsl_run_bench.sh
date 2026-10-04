#!/usr/bin/env bash
set -e
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os

ATLAS_BIN="/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas"
ENV_FILE="/mnt/c/Users/Max/Desktop/atlas-os/.env"

MOUNTS='[
  {"type":"bind","source":"'"$ATLAS_BIN"'","target":"/usr/local/bin/atlas","read_only":true},
  {"type":"bind","source":"'"$ENV_FILE"'","target":"/app/.env","read_only":true}
]'

# Persistent run in tmux so it survives this shell / WSL teardown.
sudo apt-get install -y -qq tmux >/dev/null 2>&1 || true

tmux kill-session -t bench 2>/dev/null || true
tmux new-session -d -s bench
tmux send-keys -t bench "cd /mnt/c/Users/Max/Desktop/atlas-os" Enter
tmux send-keys -t bench "export PATH=\$HOME/.local/bin:\$PATH" Enter
tmux send-keys -t bench "PYTHONPATH=tools harbor run -d terminal-bench/terminal-bench-2 --agent harbor_atlas.atlas_agent:AtlasAgent --model moonshotai/kimi-k3 --n-concurrent 2 --mounts '$MOUNTS' -o jobs/atlas-bench 2>&1 | tee /tmp/atlas-bench.log" Enter
echo "launched tmux session 'bench'"
tmux ls

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

sudo apt-get install -y -qq tmux >/dev/null 2>&1 || true
tmux kill-session -t bench2 2>/dev/null || true
tmux new-session -d -s bench2
tmux send-keys -t bench2 "cd /mnt/c/Users/Max/Desktop/atlas-os" Enter
tmux send-keys -t bench2 "export PATH=\$HOME/.local/bin:\$PATH ATLAS_AGENT_MODE=agent" Enter
tmux send-keys -t bench2 "PYTHONPATH=tools harbor run -d terminal-bench/terminal-bench-2 --agent harbor_atlas.atlas_agent:AtlasAgent --model openai/gpt-oss-120b --n-concurrent 2 --mounts '$MOUNTS' -o jobs/atlas-agent 2>&1 | tee /tmp/atlas-agent.log" Enter
echo "launched tmux 'bench2'"
tmux ls

#!/usr/bin/env bash
# Quick check: does the static atlas binary run inside a container and reach
# OpenCode Go? Isolates "container/agent broken" from "model too weak".
set -e
cd /mnt/c/Users/Max/Desktop/atlas-os
BIN=src-tauri/target/x86_64-unknown-linux-musl/release/atlas
KEY=$(sed -n 's/^OPENCODE_GO_KEY=//p' tools/harbor_atlas/.env.harbor)
docker run --rm \
  -v "$PWD/$BIN":/usr/local/bin/atlas:ro \
  -e ATLAS_LLM_BASE_URL=https://opencode.ai/zen/go/v1 \
  -e ATLAS_LLM_MODEL=deepseek-v4.1-flash \
  -e OPENCODE_GO_KEY="$KEY" \
  ubuntu:24.04 \
  sh -c 'mkdir -p /tmp/t && /usr/local/bin/atlas agent --root /tmp/t --max-steps 4 "create a file hi.txt containing exactly hi" ; echo EXIT=$? ; echo "--- file ---" ; cat /tmp/t/hi.txt 2>/dev/null'

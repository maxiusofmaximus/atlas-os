#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
ATLAS="/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas"
ENVF="/mnt/c/Users/Max/Desktop/atlas-os/.env"

docker run --rm \
  -v "$ATLAS:/usr/local/bin/atlas:ro" \
  -v "$ENVF:/app/.env:ro" \
  -w /app \
  ubuntu:24.04 \
  bash -lc '
    set -x
    atlas --profile harbor mission new --force "add a function add(a,b) to /app/solution.py and a test"
    ID=$(atlas --profile harbor mission list 2>/dev/null | head -1 || true)
    echo "listed: $ID"
    atlas --profile harbor plan "$ID"; echo "PLAN_EXIT=$?"
    atlas --profile harbor execute --coding --apply --root . "$ID"; echo "EXEC_EXIT=$?"
  ' 2>&1 | tail -n 30

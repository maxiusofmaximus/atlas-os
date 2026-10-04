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
    ID=$(atlas --profile harbor mission new --force "add an add(a,b) function to solution.py" | awk "/^mission /{print \$2; exit}")
    echo "ID=[$ID]"
    atlas --profile harbor plan "$ID"; echo "PLAN_EXIT=$?"
    atlas --profile harbor execute --coding --apply --root . "$ID" 2>&1 | tail -n 6; echo "EXEC_EXIT=${PIPESTATUS[0]}"
  ' 2>&1 | tail -n 20

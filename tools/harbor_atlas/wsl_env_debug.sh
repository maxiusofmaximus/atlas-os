#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
ATLAS="/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/release/atlas"
ENVF="/mnt/c/Users/Max/Desktop/atlas-os/.env"

# Extract the 3 vars to pass as explicit env (mirrors Harbor --ae).
BASE=$(grep -E '^ATLAS_LLM_BASE_URL=' "$ENVF" | cut -d= -f2-)
MODEL=$(grep -E '^ATLAS_LLM_MODEL=' "$ENVF" | cut -d= -f2-)
KEY=$(grep -E '^NVIDIA_API_KEY=' "$ENVF" | cut -d= -f2-)
echo "BASE=$BASE MODEL=$MODEL KEYLEN=${#KEY}"

docker run --rm \
  -v "$ATLAS:/usr/local/bin/atlas:ro" \
  -e ATLAS_LLM_BASE_URL="$BASE" \
  -e ATLAS_LLM_MODEL="$MODEL" \
  -e NVIDIA_API_KEY="$KEY" \
  -w /work \
  ubuntu:24.04 \
  bash -lc '
    echo "pub fn add(a: i32, b: i32) -> i32 { a + b }" > /work/lib.rs
    ID=$(atlas --profile harbor mission new --force "add a doc comment to lib.rs add()" | awk "/^mission /{print \$2; exit}")
    echo "ID=[$ID]"
    atlas --profile harbor plan "$ID" 2>&1 | grep -E "step|SO|blocked" | head -12; echo "PLAN=$?"
    atlas --profile harbor execute --coding --apply --root /work "$ID" 2>&1 | tail -n 8; echo "EXEC=${PIPESTATUS[0]}"
  ' 2>&1 | tail -n 18

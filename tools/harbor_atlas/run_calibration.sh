#!/usr/bin/env bash
# A1 calibration — run the Atlas agent on a task list for one model.
# Usage: run_calibration.sh <MODEL> <task1> [task2 ...]
# One model at a time (single Docker); n-concurrent 2.
set -e
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os
pgrep dockerd >/dev/null || (sudo dockerd >/var/log/dockerd.log 2>&1 & sleep 6)

MODEL="$1"; shift
if [ -z "$MODEL" ]; then echo "usage: $0 <MODEL> [task...]  (no task = full dataset)"; exit 2; fi

BIN=/mnt/c/Users/Max/Desktop/atlas-os/src-tauri/target/x86_64-unknown-linux-musl/release/atlas
chmod +x "$BIN"

ENV_FILE=tools/harbor_atlas/.env.harbor
# Load endpoint + key on the HOST so we can forward them into the container.
# No `set -x`; values are never echoed (see redact_job below for job config.json).
if [ -f "$ENV_FILE" ]; then
  set -a
  # shellcheck disable=SC1090
  . "$ENV_FILE"
  set +a
fi

INCLUDE=""
for t in "$@"; do
  case "$t" in
    */*) INCLUDE="$INCLUDE -i $t" ;;
    *) INCLUDE="$INCLUDE -i terminal-bench/$t" ;;
  esac
done

# Harbor already self-binds/collects the convention dir /logs/artifacts at the
# trial level (models/task/artifacts.py with_convention_entry); binding it again
# here collides on the same container target. Only mount the atlas binary.
MOUNTS="[{\"type\":\"bind\",\"source\":\"$BIN\",\"target\":\"/usr/local/bin/atlas\",\"read_only\":true}]"
SAFE=$(echo "$MODEL" | tr '/:' '__')
JOB_DIR="jobs/calib-${SAFE}-$(date +%Y%m%d-%H%M%S)"

# Base URL override is OPTIONAL: by default use the env-file's ATLAS_LLM_BASE_URL
# (overriding it to /zen/v1 yields NoFallbackConfigured -> 0 model calls).
# The endpoint/model/key must reach the CONTAINER env: the adapter injects only
# ATLAS_LLM_MODEL, so `atlas agent` inside otherwise sees no deployment
# (registry.rs deployments_from_env -> "no deployments", 0 model calls).
# Forward the base URL and the OPENCODE_GO_KEY slot explicitly
# (registry.rs:162 keys off ATLAS_LLM_API_KEY_ENV).
AE_ARGS="--ae ATLAS_AGENT_MODE=agent --ae ATLAS_LLM_MODEL=$MODEL"
if [ -n "${ATLAS_LLM_BASE_URL:-}" ]; then
  AE_ARGS="$AE_ARGS --ae ATLAS_LLM_BASE_URL=$ATLAS_LLM_BASE_URL"
fi
if [ -n "${OPENCODE_GO_KEY:-}" ]; then
  AE_ARGS="$AE_ARGS --ae ATLAS_LLM_API_KEY_ENV=OPENCODE_GO_KEY --ae OPENCODE_GO_KEY=$OPENCODE_GO_KEY"
fi

# Replace the secret in every persisted job config.json before reporting.
# Never prints the key.
redact_job() {
  local dir="$1"
  [ -n "${OPENCODE_GO_KEY:-}" ] || return 0
  local esc
  esc=$(printf '%s' "$OPENCODE_GO_KEY" | sed 's/[][\.*^$/&|+?(){}]/\\&/g')
  find "$dir" -type f -name config.json -print0 2>/dev/null | while IFS= read -r -d '' f; do
    sed -i "s/$esc/REDACTED/g" "$f"
  done
}

echo "MODEL=$MODEL TASKS=$* JOB_DIR=$JOB_DIR BASE=${ATLAS_LLM_BASE_URL:-<unset>}"
set +e
# shellcheck disable=SC2086
PYTHONPATH=tools harbor run \
  -d terminal-bench/terminal-bench-2 \
  --agent harbor_atlas.atlas_agent:AtlasAgent \
  --model "$MODEL" \
  $AE_ARGS \
  --n-concurrent 2 \
  $INCLUDE \
  --env-file "$ENV_FILE" \
  --allow-environment-host opencode.ai \
  --mounts "$MOUNTS" \
  -o "$JOB_DIR" \
  -y
RC=$?
set -e
redact_job "$JOB_DIR"
echo "JOB_DIR=$JOB_DIR"

echo "--- result.json (head) ---"
head -c 600 "$JOB_DIR/result.json" 2>/dev/null || echo "(no result.json)"
exit "$RC"

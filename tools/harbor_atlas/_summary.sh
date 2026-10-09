#!/usr/bin/env bash
# TESTER summary: per-trial reward + agent outcome for the latest job of <safe-model>.
set -u
cd /mnt/c/Users/Max/Desktop/atlas-os || exit 1
SAFE="${1:-longcat-2.5-preview-free}"
d=$(ls -1dt jobs/calib-${SAFE}-* 2>/dev/null | head -1)
echo "JOB=$d"
echo "=== per-trial ==="
for t in $(find "$d" -mindepth 2 -maxdepth 2 -type d -name '*__*' | sort); do
  name=$(basename "$t")
  rw=$(cat "$t/verifier/reward.txt" 2>/dev/null || echo "?")
  log="$t/artifacts/logs/artifacts/atlas_agent.log"
  last=$(grep -aoE 'agent done=[a-z]+ turns=[0-9]+ tools=[0-9]+ tokens_in=[0-9]+ tokens_out=[0-9]+' "$log" 2>/dev/null | tail -1)
  [ -z "$last" ] && last=$(grep -aiE 'error|panic|no deployments|timeout' "$log" 2>/dev/null | tail -1 | cut -c1-120)
  [ -z "$last" ] && last="(no agent log)"
  printf '%-42s reward=%s  %s\n' "$name" "$rw" "$last"
done
echo "=== job stats (eras/exceptions) ==="
res=$(find "$d" -maxdepth 2 -name result.json | head -1)
python3 - "$res" <<'PY'
import json,sys
j=json.load(open(sys.argv[1]))
for k,v in (j.get("stats",{}).get("evals") or {}).items():
    print("mean:",v.get("metrics"))
    print("exceptions:",json.dumps(v.get("exception_stats")))
PY
echo "=== KEY-LEAK CHECK (boolean) ==="
set -a; . tools/harbor_atlas/.env.harbor 2>/dev/null; set +a
if grep -rqF "$OPENCODE_GO_KEY" "$d" 2>/dev/null; then echo "LEAK=YES"; else echo "LEAK=NO"; fi
echo "(end)"

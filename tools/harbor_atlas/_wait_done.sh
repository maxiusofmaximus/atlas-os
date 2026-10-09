#!/usr/bin/env bash
# TESTER: wait until the latest job for <safe-model> has no running trials.
set -u
cd /mnt/c/Users/Max/Desktop/atlas-os || exit 1
SAFE="${1:-longcat-2.5-preview-free}"
MAX="${2:-40}"   # iterations (~30s each)
i=0
while [ "$i" -lt "$MAX" ]; do
  d=$(ls -1dt jobs/calib-${SAFE}-* 2>/dev/null | head -1)
  res=$(find "$d" -maxdepth 2 -name result.json 2>/dev/null | head -1)
  python3 - "$res" <<'PY' 2>/dev/null
import json,sys
try: j=json.load(open(sys.argv[1]))
except Exception: print("(no result yet)"); sys.exit(3)
s=j.get("stats",{})
print("completed:",s.get("n_completed_trials"),"running:",s.get("n_running_trials"),"errored:",s.get("n_errored_trials"))
for k,v in (s.get("evals") or {}).items():
    print("  rewards:",v.get("reward_stats"))
import sys as _s
_s.exit(0 if (s.get("n_running_trials") or 0)==0 and (s.get("n_completed_trials") or 0)>0 else 3)
PY
  rc=$?
  if [ "$rc" -eq 0 ]; then echo "DONE job=$d"; exit 0; fi
  sleep 30
  i=$((i+1))
done
echo "TIMEOUT after $MAX iterations"

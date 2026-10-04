#!/usr/bin/env bash
cd /mnt/c/Users/Max/Desktop/atlas-os
J="jobs/2026-10-03__22-57-25"
echo "=== completed trials (have verifier/reward) ==="
count=0
for d in "$J"/*/; do
  if [ -f "$d/verifier/reward.txt" ]; then
    r=$(cat "$d/verifier/reward.txt" 2>/dev/null)
    name=$(basename "$d")
    echo "  $name -> reward=$r"
    count=$((count+1))
  fi
done
echo "completed: $count"
echo "=== job stats ==="
python3 - "$J/result.json" <<'PY'
import json,sys
d=json.load(open(sys.argv[1]))
print("n_total_trials:", d.get("n_total_trials"))
print("stats:", json.dumps(d.get("stats"), indent=1)[:800])
PY

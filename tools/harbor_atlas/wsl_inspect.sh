#!/usr/bin/env bash
J="jobs/2026-10-03__22-57-25"
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "=== dir ==="
ls -la "$J" 2>&1 | head -8
echo "=== result.json (trimmed) ==="
python3 - "$J/result.json" <<'PY'
import json,sys
try:
    d=json.load(open(sys.argv[1]))
except Exception as e:
    print("ERR", e); raise SystemExit
print("keys:", list(d.keys())[:15])
tr = d.get("trials") or d.get("results") or []
print("n_trials:", len(tr) if isinstance(tr,list) else tr)
if isinstance(tr,list) and tr:
    t=tr[0]
    print("trial keys:", list(t.keys())[:15] if isinstance(t,dict) else type(t))
    if isinstance(t,dict):
        print("task:", t.get("task_name"), "reward:", t.get("reward"), "verifier:", t.get("verifier_result"))
PY
echo "=== docker images built ==="
docker images --format '{{.Repository}}:{{.Tag}}' | head -8

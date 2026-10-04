#!/usr/bin/env python3
import json, glob, os
base = "/mnt/c/Users/Max/Desktop/atlas-os/jobs/atlas-bench/2026-10-04__00-17-06"
d = json.load(open(os.path.join(base, "result.json")))
print("=== stats keys ===")
print(list(d.get("stats", {}).keys()))
print(json.dumps(d.get("stats"), indent=1)[:900])
print("\n=== first trial result.json ===")
for tr in sorted(glob.glob(os.path.join(base, "*", "result.json")))[:1]:
    t = json.load(open(tr))
    print("path:", tr)
    print("keys:", list(t.keys()))
    print(json.dumps(t, indent=1)[:1200])

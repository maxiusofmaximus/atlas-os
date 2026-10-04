#!/usr/bin/env python3
import json
p = "/mnt/c/Users/Max/Desktop/atlas-os/jobs/atlas-bench/2026-10-04__00-17-06/result.json"
d = json.load(open(p))
print("TOP KEYS:", list(d.keys()))
for k, v in d.items():
    if k == "stats":
        continue
    if isinstance(v, list):
        print(f"  {k}: list[{len(v)}]")
        if v:
            print(f"    [0] keys: {list(v[0].keys()) if isinstance(v[0], dict) else type(v[0])}")
            if isinstance(v[0], dict):
                print(f"    [0] sample: {json.dumps(v[0], indent=1)[:700]}")
    elif isinstance(v, dict):
        print(f"  {k}: dict keys {list(v.keys())[:12]}")
    else:
        print(f"  {k}: {v}")

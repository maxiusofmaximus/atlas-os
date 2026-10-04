#!/usr/bin/env python3
import json, glob, os
base = "/mnt/c/Users/Max/Desktop/atlas-os/jobs/atlas-bench/2026-10-04__00-17-06"
t = json.load(open(os.path.join(base, "build-pov-ray__PxAPEdg", "result.json")))
print("verifier_result:", json.dumps(t.get("verifier_result"), indent=1)[:400])
print("agent_result:", json.dumps(t.get("agent_result"), indent=1)[:400])
print("exception_info:", json.dumps(t.get("exception_info"), indent=1)[:200])
print("agent_info:", json.dumps(t.get("agent_info"), indent=1)[:300])

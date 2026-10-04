#!/usr/bin/env python3
import json
p = "/mnt/c/Users/Max/Desktop/atlas-os/jobs/atlas-bench/2026-10-04__00-17-06/result.json"
d = json.load(open(p))
s = d["stats"]
print("n_total_trials:", d["n_total_trials"])
print("completed:", s["n_completed_trials"], "errored:", s["n_errored_trials"],
      "running:", s["n_running_trials"], "pending:", s["n_pending_trials"],
      "cancelled:", s["n_cancelled_trials"], "retries:", s["n_retries"])
print("tokens in/out:", s.get("n_input_tokens"), s.get("n_output_tokens"), "cost:", s.get("cost_usd"))
for name, ev in (s.get("evals") or {}).items():
    print(f"\nEVAL {name}: n_trials={ev.get('n_trials')} n_errors={ev.get('n_errors')}")
    print("  metrics:", ev.get("metrics"))
    rs = ev.get("reward_stats", {}).get("reward", {})
    for r, tasks in rs.items():
        print(f"  reward {r}: {len(tasks)} tasks")
    print("  exceptions:", ev.get("exception_stats"))

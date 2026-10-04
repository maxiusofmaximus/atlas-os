#!/usr/bin/env bash
J=$(ls -td /mnt/c/Users/Max/Desktop/atlas-os/jobs/atlas-smoke/*/ | head -1)
echo "job: $J"
D=$(ls -d "$J"*/ | head -1)
echo "trial: $D"
echo "=== agent dir ==="
ls -la "$D/agent" 2>&1
echo "=== agent stdout/stderr ==="
for f in "$D/agent"/*.txt "$D/agent"/*.log "$D"/*.log; do
  [ -f "$f" ] && { echo "--- $f ---"; tail -n 40 "$f"; }
done
echo "=== trial.log tail ==="
tail -n 50 "$D/trial.log" 2>&1

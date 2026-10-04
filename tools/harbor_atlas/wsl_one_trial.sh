#!/usr/bin/env bash
cd /mnt/c/Users/Max/Desktop/atlas-os
D=$(find jobs/atlas-bench -maxdepth 3 -type d -name "build-pov-ray*" | head -1)
echo "trial: $D"
echo "=== trial.log (filtered) ==="
grep -vE '^\s*$' "$D/trial.log" 2>&1 | tail -n 25
echo "=== agent dir ==="
ls -R "$D/agent" 2>&1 | head -20

#!/usr/bin/env bash
cd /mnt/c/Users/Max/Desktop/atlas-os
D=$(find jobs/atlas-agent -maxdepth 3 -type d -name "*fibsqrt*" | head -1)
echo "trial: $D"
echo "=== trial.log (non-empty) ==="
grep -vE '^\s*$' "$D/trial.log" 2>&1 | tail -n 30
echo "=== agent stdout (if any) ==="
find "$D/agent" -type f 2>/dev/null | head -5

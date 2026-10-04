#!/usr/bin/env bash
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "=== rewards so far ==="
for f in $(find jobs/atlas-bench -name reward.txt 2>/dev/null); do
  echo "$(basename $(dirname $(dirname $f))) -> $(cat $f)"
done
echo "=== breakdown ==="
find jobs/atlas-bench -name reward.txt 2>/dev/null | xargs cat 2>/dev/null | sort | uniq -c

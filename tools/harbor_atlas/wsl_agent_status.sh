#!/usr/bin/env bash
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "rewards:"
for f in $(find jobs/atlas-agent -name reward.txt 2>/dev/null); do
  echo "  $(basename $(dirname $(dirname $f))) -> $(cat $f)"
done
echo "breakdown:"
find jobs/atlas-agent -name reward.txt 2>/dev/null | xargs cat 2>/dev/null | sort | uniq -c
echo "docker active: $(docker ps -q | wc -l)"

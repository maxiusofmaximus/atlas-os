#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "=== docker sanity ==="
docker run --rm hello-world 2>&1 | grep -E "Hello|correctly" || true
echo "=== harbor oracle smoke (1 task) ==="
harbor run -d terminal-bench/terminal-bench-2 -a oracle 2>&1 | tail -n 40

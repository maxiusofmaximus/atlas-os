#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
cd /mnt/c/Users/Max/Desktop/atlas-os
echo "started $(date)" > /tmp/oracle.log
nohup harbor run -d terminal-bench/terminal-bench-2 -a oracle --n-concurrent 4 >> /tmp/oracle.log 2>&1 &
echo "pid=$!"

#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
echo "=== grep mounts/env/ae ==="
harbor run --help 2>&1 | grep -inE 'mount|env|--ae|include-task|extra|jobs-dir|agent' | head -40

#!/usr/bin/env bash
export PATH="$HOME/.local/bin:$PATH"
harbor run --help 2>&1 | grep -iE "mount|--env|task|dataset|agent|ae\b|include-task|jobs-dir" | head -30
echo "=== config help ==="
harbor run --help 2>&1 | sed -n '/Options/,$p' | head -60
